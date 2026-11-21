mod address_book;
mod errors;
mod imessage;
mod models;
mod snowflake;

use axum::{
    body::to_bytes,
    extract::State,
    http::{header, HeaderValue},
    response::{IntoResponse, Response, Sse},
    routing::get,
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use crate::imessage::IMessageServer;

#[cfg(target_os = "macos")]
use crate::address_book::AddressBook;

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
}

impl McpServerState {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        let address_book = AddressBook::new().ok();

        #[cfg(not(target_os = "macos"))]
        let address_book: Option<address_book::AddressBook> = None;

        Self {
            imessage_server: IMessageServer::new(None, address_book.clone()),
            #[cfg(target_os = "macos")]
            address_book,
            #[cfg(not(target_os = "macos"))]
            address_book: None,
            initialized: Arc::new(RwLock::new(false)),
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

async fn handle_mcp_get(State(_state): State<Arc<McpServerState>>) -> Response {
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
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = Arc::new(McpServerState::new());

    let app = Router::new()
        .route("/mcp", get(handle_mcp_get).post(handle_mcp_post))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    info!("MCP HTTP streamable server listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
