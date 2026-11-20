// =============================================================================
// DBV Unreal AI — Tests: servidor MCP de UEFN simulado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Un servidor HTTP mínimo que habla el MCP de UEFN para los tests: responde `initialize` y las notificaciones, y para
// lo demás llama a `handler` con el cuerpo JSON-RPC y devuelve su `result`. Atiende cada conexión en su hilo, como el
// editor, para poder probar llamadas a la vez (el candado de sesión).

use std::io::{Read, Write};

use serde_json::{json, Value};

const CRLF: &str = "\r\n";

/// `result` grabado de un fichero de `tests/fixtures/unreal-mcp/uefn-43.10/`.
pub fn recorded_result(file: &str) -> Value {
    let text = std::fs::read_to_string(format!("{}/../tests/fixtures/unreal-mcp/uefn-43.10/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap_or_default();
    serde_json::from_str::<Value>(&text).map(|v| v["result"].clone()).unwrap_or(Value::Null)
}

/// `result` de una herramienta que devuelve `text` (como las de UEFN: JSON dentro de un bloque de texto).
pub fn text_result(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

/// Nombre de la herramienta de un `tools/call` (la del toolset si es `call_tool`).
pub fn tool_name(body: &Value) -> String {
    let params = &body["params"];
    params["arguments"]["tool_name"].as_str().or_else(|| params["name"].as_str()).unwrap_or("").to_string()
}

fn read_request(socket: &mut std::net::TcpStream) -> Value {
    let mut received = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = socket.read(&mut buffer).unwrap_or(0);
        received.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&received).to_string();
        let separator = format!("{CRLF}{CRLF}");
        let done = text.find(&separator).is_some_and(|end| {
            let length = text[..end].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
            received.len() >= end + 4 + length
        });
        if done || read == 0 {
            break;
        }
    }
    let request = String::from_utf8_lossy(&received).to_string();
    request.split_once(&format!("{CRLF}{CRLF}")).and_then(|(_, b)| serde_json::from_str(b).ok()).unwrap_or(Value::Null)
}

/// Arranca el servidor y devuelve su URL (`http://127.0.0.1:<puerto>/mcp`).
pub fn serve(handler: impl Fn(&Value) -> Value + Send + Sync + 'static) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let handler = std::sync::Arc::new(handler);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut socket) = stream else { break };
            let handler = handler.clone();
            std::thread::spawn(move || {
                let body = read_request(&mut socket);
                let id = body.get("id").cloned().unwrap_or(Value::Null);
                let payload = match body["method"].as_str().unwrap_or("") {
                    "initialize" => json!({ "jsonrpc": "2.0", "id": id, "result": { "protocolVersion": "2025-06-18", "capabilities": {}, "serverInfo": { "name": "", "title": "", "version": "" } } }).to_string(),
                    "notifications/initialized" => String::new(),
                    _ => json!({ "jsonrpc": "2.0", "id": id, "result": handler(&body) }).to_string(),
                };
                let head = format!("HTTP/1.1 200 OK{CRLF}Content-Type: application/json{CRLF}Mcp-Session-Id: fake{CRLF}Content-Length: {}{CRLF}Connection: close{CRLF}{CRLF}", payload.len());
                let _ = socket.write_all(head.as_bytes());
                let _ = socket.write_all(payload.as_bytes());
            });
        }
    });
    url
}
