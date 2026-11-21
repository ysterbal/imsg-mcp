mod address_book;
mod errors;
mod imessage;
mod models;
mod snowflake;

use axum::{
    body::to_bytes,
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::get,
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use crate::imessage::IMessageServer;

#[cfg(target_os = "macos")]
use crate::address_book::AddressBook;

// Configuration
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    #[serde(default)]
    pub auth_token: Option<String>, // Deprecated: use IMSG_MCP_AUTH_TOKEN env var instead
}

impl ServerConfig {
    pub fn load() -> Self {
        let mut config = ServerConfig::default();
        config.port = 3000;

        let config_paths: Vec<String> = vec![
            "imsg-mcp.json".to_string(),
            ".imsg-mcp.json".to_string(),
            shellexpand::tilde("~/.imsg-mcp.json").to_string(),
            shellexpand::tilde("~/.imsg-mcp/config.json").to_string(),
        ];
        
        for path_str in config_paths {
            if Path::new(&path_str).exists() {
                if let Ok(content) = fs::read_to_string(&path_str) {
                    if let Ok(parsed) = serde_json::from_str::<ServerConfig>(&content) {
                        config.port = parsed.port;
                        if parsed.auth_token.is_some() {
                            tracing::warn!("auth_token found in config file at {}. For security, please remove auth_token from the config file and use the IMSG_MCP_AUTH_TOKEN environment variable instead.", path_str);
                        }
                        info!("Loaded config from {}", path_str);
                        break; // Stop after first found config
                    } else {
                        tracing::warn!("Failed to parse config from {}", path_str);
                    }
                }
            }
        }
        
        // Read auth token from environment variable (overrides config file for security)
        let auth_token = std::env::var("IMSG_MCP_AUTH_TOKEN").ok();
        
        if auth_token.is_some() {
            info!("Authentication enabled via IMSG_MCP_AUTH_TOKEN environment variable");
        } else if config.auth_token.is_some() {
            info!("Authentication enabled via config file (deprecated and insecure)");
        } else {
            info!("Using default config (port: {}, no auth token)", config.port);
        }

        // Return config with auth_token from env var (prioritizes env var for security)
        ServerConfig {
            port: config.port,
            auth_token,
        }
    }
}

// MCP JSON-RPC types
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<serde_json::Value>,
    #[serde(default)]
    id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    id: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    arguments: Option<serde_json::Value>,
}

// Server state
#[derive(Clone)]
pub struct McpServerState {
    pub imessage_server: IMessageServer,
    #[cfg(target_os = "macos")]
    pub address_book: Option<AddressBook>,
    pub initialized: Arc<RwLock<bool>>,
    pub auth_token: Option<String>,
}

impl McpServerState {
    pub fn new(config: &ServerConfig) -> Self {
        #[cfg(target_os = "macos")]
        let address_book = AddressBook::new().ok();

        #[cfg(not(target_os = "macos"))]
        let address_book: Option<address_book::AddressBook> = None;

        let imessage_server = IMessageServer::new(None, address_book.clone());

        // Attempt to initialize DB to trigger permission dialog if needed
        if let Err(e) = imessage_server.init_db() {
            tracing::warn!("Failed to initialize database during startup: {}", e);
        }

        Self {
            imessage_server,
            #[cfg(target_os = "macos")]
            address_book,
            #[cfg(not(target_os = "macos"))]
            address_book: None,
            initialized: Arc::new(RwLock::new(false)),
            auth_token: config.auth_token.clone(),
        }
    }
}

// SSE Event type
#[derive(Serialize)]
struct SseEvent {
    #[serde(rename = "event")]
    event_name: String,
    #[serde(rename = "data")]
    data: serde_json::Value,
}

impl SseEvent {
    fn new(message: serde_json::Value) -> Self {
        Self {
            event_name: "message".to_string(),
            data: message,
        }
    }
}

fn create_json_rpc_response(id: serde_json::Value, result: Option<serde_json::Value>, error: Option<JsonRpcError>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result,
        error,
        id,
    }
}

fn create_error_response(id: serde_json::Value, code: i32, message: String) -> JsonRpcResponse {
    create_json_rpc_response(
        id,
        None,
        Some(JsonRpcError { code, message }),
    )
}

async fn handle_mcp_post(
    State(state): State<Arc<McpServerState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Response {
    // Verify auth token if configured and not empty
    if let Some(token) = &state.auth_token {
        if !token.is_empty() {
            let auth_header = request.headers().get(header::AUTHORIZATION).and_then(|h| h.to_str().ok());
            let expected = format!("Bearer {}", token);
            if auth_header != Some(&expected) {
                let mut response = Response::new(axum::body::Body::from("Unauthorized"));
                *response.status_mut() = StatusCode::UNAUTHORIZED;
                response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
                return response;
            }
        }
    }

    // Check if client wants SSE response before consuming the body
    let accept_header = request.headers().get(header::ACCEPT).and_then(|h| h.to_str().ok()).unwrap_or("");
    let wants_sse = accept_header.contains("text/event-stream");

    let body_bytes = match to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            let resp = create_error_response(serde_json::Value::Null, -32700, "Parse error".to_string());
            return Json(resp).into_response();
        }
    };

    let request_str = match String::from_utf8(body_bytes) {
        Ok(s) => s,
        Err(_) => {
            let resp = create_error_response(serde_json::Value::Null, -32700, "Parse error".to_string());
            return Json(resp).into_response();
        }
    };

    let req: JsonRpcRequest = match serde_json::from_str(&request_str) {
        Ok(r) => r,
        Err(e) => {
            let resp = create_error_response(serde_json::Value::Null, -32700, format!("Parse error: {}", e));
            return Json(resp).into_response();
        }
    };

    let response_id = req.id.clone().unwrap_or(serde_json::Value::Null);
    let mut is_initialized = state.initialized.write().await;

    let response = if !*is_initialized && req.method == "initialize" {
        *is_initialized = true;
        let result = json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "imsg-mcp",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {}
            }
        });
        create_json_rpc_response(response_id, Some(result), None)
    } else if *is_initialized {
        match req.method.as_str() {
            "tools/list" => {
                let tools = json!({
                    "tools": [
                        {
                            "name": "inbox",
                            "description": "Lists the messages in the inbox",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "limit": {
                                        "type": "integer",
                                        "description": "Maximum number of messages to return",
                                        "default": 100
                                    }
                                },
                                "required": []
                            }
                        },
                        {
                            "name": "sent",
                            "description": "Lists the messages in the sent folder",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "limit": {
                                        "type": "integer",
                                        "description": "Maximum number of messages to return",
                                        "default": 100
                                    }
                                },
                                "required": []
                            }
                        }
                    ]
                });
                create_json_rpc_response(response_id, Some(tools), None)
            }
            "tools/call" => {
                let params: ToolCallParams = match serde_json::from_value(
                    req.params.clone().unwrap_or_default(),
                ) {
                    Ok(p) => p,
                    Err(_) => {
                        let resp = create_error_response(response_id, -32602, "Invalid params".to_string());
                        return Json(resp).into_response();
                    }
                };

                let limit = params
                    .arguments
                    .as_ref()
                    .and_then(|v: &serde_json::Value| v.get("limit"))
                    .and_then(|v: &serde_json::Value| v.as_u64())
                    .map(|v| v as usize)
                    .unwrap_or(100);

                let messages_result = match params.name.as_str() {
                    "inbox" => state.imessage_server.get_received_messages(limit),
                    "sent" => state.imessage_server.get_sent_messages(limit),
                    _ => {
                        let resp = create_error_response(response_id, -32602, format!("Unknown tool: {}", params.name));
                        return Json(resp).into_response();
                    }
                };

                let messages = match messages_result {
                    Ok(msgs) => msgs,
                    Err(e) => {
                        let resp = create_error_response(response_id, -32000, e.to_string());
                        return Json(resp).into_response();
                    }
                };

                let text_contents: Vec<serde_json::Value> = messages
                    .iter()
                    .map(|msg| {
                        json!({
                            "type": "text",
                            "text": msg.to_string()
                        })
                    })
                    .collect();

                let result = json!({
                    "content": text_contents,
                    "isError": false
                });

                create_json_rpc_response(response_id, Some(result), None)
            }
            "ping" => {
                let result = json!({});
                create_json_rpc_response(response_id, Some(result), None)
            }
            _ => {
                let err = JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                };
                create_json_rpc_response(response_id, None, Some(err))
            }
        }
    } else {
        let err = JsonRpcError {
            code: -32000,
            message: "Not initialized".to_string(),
        };
        create_json_rpc_response(response_id, None, Some(err))
    };

    if wants_sse {
        let event = SseEvent::new(serde_json::to_value(&response).unwrap_or_else(|_| json!({})));
        let event_str = serde_json::to_string(&event).unwrap();
        let event_stream = futures::stream::once(async move {
            Ok::<_, axum::Error>(axum::response::sse::Event::default().event("message").data(event_str))
        });
        let mut response = Sse::new(event_stream).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        response
    } else {
        Json(response).into_response()
    }
}

async fn handle_mcp_get(
    State(state): State<Arc<McpServerState>>,
    request: axum::http::Request<axum::body::Body>,
) -> Response {
    // Verify auth token if configured and not empty
    if let Some(token) = &state.auth_token {
        if !token.is_empty() {
            let auth_header = request.headers().get(header::AUTHORIZATION).and_then(|h| h.to_str().ok());
            let expected = format!("Bearer {}", token);
            if auth_header != Some(&expected) {
                let mut response = Response::new(axum::body::Body::from("Unauthorized"));
                *response.status_mut() = StatusCode::UNAUTHORIZED;
                response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
                return response;
            }
        }
    }

    // SSE connection for server-to-client messages
    let (tx, rx) = mpsc::channel(100);
    
    // Drop the tx immediately since we don't send server-initiated messages yet
    // This creates a stream that will close immediately, but sets up the SSE connection structure
    drop(tx);
    
    let stream = ReceiverStream::new(rx).map(|_msg: serde_json::Value| {
        Ok::<_, axum::Error>(axum::response::sse::Event::default().event("message").data(""))
    });

    let mut response = Sse::new(stream).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Privacy-first logging configuration:
    // - Default to 'info' level to suppress debug/trace logs that might contain sensitive data
    // - Suppress verbose logging from dependencies (rusqlite, axum, tokio, etc.) that could potentially log SQL queries or request/response bodies
    let log_level = std::env::var("IMSG_MCP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    
    let filter_str = format!(
        "imsg_mcp={},axum=warn,tower_http=warn,rusqlite=warn,tokio=warn,hyper=warn",
        log_level
    );
    
    let filter = tracing_subscriber::EnvFilter::try_new(filter_str).unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let config = ServerConfig::load();

    let state = Arc::new(McpServerState::new(&config));

    let app = Router::new()
        .route("/mcp", get(handle_mcp_get).post(handle_mcp_post))
        .with_state(state);

    let bind_addr = format!("127.0.0.1:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("MCP HTTP streamable server listening on {}", bind_addr);
    if config.auth_token.is_some() {
        info!("Authentication is enabled for this server");
    }

    axum::serve(listener, app).await?;

    Ok(())
}
