mod address_book;
mod errors;
mod imessage;
mod models;
mod snowflake;

use std::io::{self, BufRead, BufReader, Write};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::imessage::IMessageServer;

#[cfg(target_os = "macos")]
use crate::address_book::AddressBook;

// JSON-RPC 2.0 types
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

fn send_response(stdout: &mut impl Write, response: JsonRpcResponse) -> Result<()> {
    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
    stdout.flush()?;
    Ok(())
}

fn send_error_response(stdout: &mut impl Write, id: serde_json::Value, code: i32, message: String) -> Result<()> {
    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError { code, message }),
        id,
    };
    send_response(stdout, response)
}

fn main() -> Result<()> {
    // Initialize server state
    #[cfg(target_os = "macos")]
    let address_book = AddressBook::new().ok();

    #[cfg(not(target_os = "macos"))]
    let address_book: Option<address_book::AddressBook> = None;

    let imessage_server = IMessageServer::new(None, address_book);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let reader = BufReader::new(stdin);

    // MCP Initialization state
    let mut initialized = false;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                    id: serde_json::Value::Null,
                };
                send_response(&mut stdout, response)?;
                continue;
            }
        };

        let response_id = request.id.clone().unwrap_or(serde_json::Value::Null);

        let response = if !initialized && request.method == "initialize" {
            initialized = true;
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
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id: response_id,
            }
        } else if initialized {
            match request.method.as_str() {
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
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(tools),
                        error: None,
                        id: response_id,
                    }
                }
                "tools/call" => {
                    let params: ToolCallParams = match serde_json::from_value(
                        request.params.clone().unwrap_or_default(),
                    ) {
                        Ok(p) => p,
                        Err(_) => {
                            send_error_response(&mut stdout, response_id, -32602, "Invalid params".to_string())?;
                            continue;
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
                        "inbox" => imessage_server.get_received_messages(limit),
                        "sent" => imessage_server.get_sent_messages(limit),
                        _ => {
                            send_error_response(&mut stdout, response_id, -32602, format!("Unknown tool: {}", params.name))?;
                            continue;
                        }
                    };

                    let messages = match messages_result {
                        Ok(msgs) => msgs,
                        Err(e) => {
                            send_error_response(&mut stdout, response_id, -32000, e.to_string())?;
                            continue;
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

                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(result),
                        error: None,
                        id: response_id,
                    }
                }
                "ping" => {
                    let result = json!({});
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(result),
                        error: None,
                        id: response_id,
                    }
                }
                _ => {
                    let err = JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                    };
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(err),
                        id: response_id,
                    }
                }
            }
        } else {
            let err = JsonRpcError {
                code: -32000,
                message: "Not initialized".to_string(),
            };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(err),
                id: response_id,
            }
        };

        send_response(&mut stdout, response)?;
    }

    Ok(())
}
