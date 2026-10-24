// =============================================================================
// DBV Unreal AI — Cliente del servidor MCP del editor de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-02, RNF-02, RNF-03
//
// Único sitio que conoce el protocolo del plugin `ModelContextProtocol` (Experimental).
// Comportamiento observado en vivo contra UE 5.8.2 el 2026-10-24 (respuestas reales en
// `tests/fixtures/unreal-mcp/ue-5.8.2/`):
//   · `initialize` → `protocolVersion: "2025-06-18"` y la sesión en la cabecera
//     `Mcp-Session-Id`, que hay que reenviar. `notifications/initialized` → 202.
//   · Las respuestas son `application/json`, formateadas en varias líneas. Se acepta
//     también SSE (`text/event-stream`) por si una versión futura lo usa.
//   · Con *tool search*, `tools/list` solo trae `list_toolsets`, `describe_toolset` y
//     `call_tool`. `list_toolsets` responde en TEXTO (`- Nombre: descripción`) y
//     `describe_toolset` con JSON DENTRO de un bloque de texto.
//   · Los toolsets C++ devuelven `{"returnValue": "<JSON en una cadena>"}`.
//
// La sesión es un dato clonable (URL + id de sesión): ninguna llamada larga retiene un
// cerrojo y otras consultas pueden ir en paralelo.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use super::UnrealError;

/// Versión del protocolo que pedimos (la que habla UE 5.8).
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Tiempo límite por defecto de una llamada (RNF-03).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
/// Tiempo límite de la conexión inicial: si el editor no contesta en esto, no está.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Sesión abierta con el servidor MCP de un editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSession {
    pub url: String,
    pub session_id: Option<String>,
    pub protocol_version: String,
}

/// Una herramienta (nativa MCP o de un toolset).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Un toolset de `list_toolsets`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsetSummary {
    pub name: String,
    pub description: String,
}

/// Un toolset descrito (`describe_toolset`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsetInfo {
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolInfo>,
}

/// Resultado de una herramienta.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    /// Texto de los bloques `text`, tal cual.
    pub text: String,
    /// El servidor marcó el resultado como error (`isError`).
    pub is_error: bool,
    /// `text` interpretado como JSON y desenvuelto (`returnValue` anidados), si lo es.
    pub value: Option<Value>,
}

pub fn endpoint_url(port: u16, path: &str) -> String {
    let path = if path.starts_with('/') { path.to_string() } else { format!("/{path}") };
    format!("http://127.0.0.1:{port}{path}")
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(timeout))
        .build()
        .new_agent()
}

/// Respuesta JSON-RPC con `id` dentro de un cuerpo JSON o SSE.
pub fn parse_rpc_body(body: &str, content_type: &str, id: u64) -> Result<Value, UnrealError> {
    let candidates: Vec<Value> = if content_type.contains("text/event-stream") {
        body.lines().filter_map(|line| line.strip_prefix("data:")).filter_map(|data| serde_json::from_str(data.trim()).ok()).collect()
    } else {
        serde_json::from_str::<Value>(body).map(|v| vec![v]).map_err(|error| UnrealError::Protocol(format!("respuesta no JSON del editor: {error}")))?
    };
    let response = candidates.into_iter().find(|message| message.get("id").and_then(Value::as_u64) == Some(id)).ok_or_else(|| UnrealError::Protocol("el editor no devolvió la respuesta pedida".into()))?;
    let result = match response.get("error") {
        Some(error) => Err(UnrealError::Protocol(error.get("message").and_then(Value::as_str).unwrap_or("error del servidor MCP").to_string())),
        None => Ok(response.get("result").cloned().unwrap_or(Value::Null)),
    };
    result
}

fn classify_transport(url: &str, error: &ureq::Error) -> UnrealError {
    let text = error.to_string();
    if matches!(error, ureq::Error::Timeout(_)) {
        UnrealError::Timeout(format!("el editor no respondió a tiempo ({url}); puede estar ocupado"))
    } else {
        UnrealError::Unreachable(format!("no se pudo conectar con el editor en {url}: {text}"))
    }
}

impl McpSession {
    /// Abre una sesión: `initialize` + `notifications/initialized`.
    pub fn connect(url: &str) -> Result<Self, UnrealError> {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0", "id": id, "method": "initialize",
            "params": { "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "dbv-unreal-ai", "version": env!("CARGO_PKG_VERSION") } }
        });
        let mut response = agent(CONNECT_TIMEOUT)
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .send_json(&body)
            .map_err(|error| classify_transport(url, &error))?;
        let status = response.status().as_u16();
        let session_id = response.headers().get("Mcp-Session-Id").and_then(|v| v.to_str().ok()).map(str::to_string);
        let content_type = response.headers().get("Content-Type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if status >= 400 { Err(UnrealError::Protocol(format!("initialize respondió HTTP {status}"))) } else { parse_rpc_body(&text, &content_type, id) }?;
        let session = McpSession {
            url: url.to_string(),
            session_id,
            protocol_version: result.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL_VERSION).to_string(),
        };
        session.notify("notifications/initialized")?;
        Ok(session)
    }

    fn post(&self, body: &Value, timeout: Duration) -> Result<(u16, String, String), UnrealError> {
        let mut request = agent(timeout)
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", &self.protocol_version);
        if let Some(id) = &self.session_id {
            request = request.header("Mcp-Session-Id", id);
        }
        let mut response = request.send_json(body).map_err(|error| classify_transport(&self.url, &error))?;
        let status = response.status().as_u16();
        let content_type = response.headers().get("Content-Type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let text = response.body_mut().read_to_string().map_err(|error| classify_transport(&self.url, &error))?;
        Ok((status, content_type, text))
    }

    fn notify(&self, method: &str) -> Result<(), UnrealError> {
        let (status, _, _) = self.post(&json!({ "jsonrpc": "2.0", "method": method }), CONNECT_TIMEOUT)?;
        let result = if status >= 400 { Err(UnrealError::Protocol(format!("{method} respondió HTTP {status}"))) } else { Ok(()) };
        result
    }

    /// Petición JSON-RPC. Un 404 significa sesión caducada (el editor se reinició): `Expired`.
    pub fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, UnrealError> {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let (status, content_type, text) = self.post(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }), timeout)?;
        let result = match status {
            404 => Err(UnrealError::Expired("la sesión MCP ya no existe (¿se reinició el editor?)".into())),
            400.. => Err(UnrealError::Protocol(format!("{method} respondió HTTP {status}: {}", text.chars().take(300).collect::<String>()))),
            _ => parse_rpc_body(&text, &content_type, id),
        };
        result
    }

    /// `tools/list`.
    pub fn list_tools(&self) -> Result<Vec<ToolInfo>, UnrealError> {
        let result = self.request("tools/list", json!({}), DEFAULT_TIMEOUT)?;
        Ok(parse_tool_list(result.get("tools").unwrap_or(&Value::Null)))
    }

    /// `tools/call` de una herramienta MCP de primer nivel.
    pub fn call(&self, name: &str, arguments: Value, timeout: Duration) -> Result<ToolResult, UnrealError> {
        let result = self.request("tools/call", json!({ "name": name, "arguments": arguments }), timeout)?;
        Ok(tool_result(&result))
    }

    /// Toolsets disponibles (`list_toolsets`).
    pub fn list_toolsets(&self) -> Result<Vec<ToolsetSummary>, UnrealError> {
        let result = self.call("list_toolsets", json!({}), DEFAULT_TIMEOUT)?;
        Ok(parse_toolset_list(&result.text))
    }

    /// Herramientas de un toolset (`describe_toolset`).
    pub fn describe_toolset(&self, toolset: &str) -> Result<ToolsetInfo, UnrealError> {
        let result = self.call("describe_toolset", json!({ "toolset_name": toolset }), DEFAULT_TIMEOUT)?;
        let info = if result.is_error { Err(UnrealError::NotFound(result.text)) } else { parse_toolset_info(&result.text) };
        info
    }

    /// Llama a una herramienta de un toolset (`call_tool`).
    pub fn call_toolset_tool(&self, toolset: &str, tool: &str, arguments: Value, timeout: Duration) -> Result<ToolResult, UnrealError> {
        self.call("call_tool", json!({ "toolset_name": toolset, "tool_name": tool, "arguments": arguments }), timeout)
    }
}

/// Lista de `tools/list` (o de un toolset) tolerante con campos ausentes.
pub fn parse_tool_list(tools: &Value) -> Vec<ToolInfo> {
    tools
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|tool| {
                    Some(ToolInfo {
                        name: tool.get("name")?.as_str()?.to_string(),
                        description: tool.get("description").and_then(Value::as_str).unwrap_or("").to_string(),
                        input_schema: tool.get("inputSchema").cloned().unwrap_or_else(|| json!({ "type": "object" })),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `- Nombre: descripción` (la descripción puede seguir en varias líneas).
pub fn parse_toolset_list(text: &str) -> Vec<ToolsetSummary> {
    let mut toolsets: Vec<ToolsetSummary> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("- ") {
            let (name, description) = rest.split_once(':').unwrap_or((rest, ""));
            toolsets.push(ToolsetSummary { name: name.trim().to_string(), description: description.trim().to_string() });
        } else if let Some(last) = toolsets.last_mut() {
            if !line.trim().is_empty() {
                if !last.description.is_empty() {
                    last.description.push('\n');
                }
                last.description.push_str(line.trim_end());
            }
        }
    }
    toolsets
}

/// JSON de `describe_toolset`. Los nombres de herramienta vienen con el toolset delante
/// (`DBVMCPFirst.MyCustomSceneToolset.RunPython`): se quedan solo con el último tramo,
/// que es lo que espera `call_tool`.
pub fn parse_toolset_info(text: &str) -> Result<ToolsetInfo, UnrealError> {
    let value: Value = serde_json::from_str(text).map_err(|error| UnrealError::Protocol(format!("describe_toolset no devolvió JSON: {error}")))?;
    let name = value.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let mut tools = parse_tool_list(value.get("tools").unwrap_or(&Value::Null));
    for tool in &mut tools {
        if let Some(short) = tool.name.rsplit('.').next() {
            tool.name = short.to_string();
        }
    }
    let result = ToolsetInfo { name, description: value.get("description").and_then(Value::as_str).unwrap_or("").to_string(), tools };
    Ok(result)
}

/// Desenvuelve los `returnValue` que traen JSON dentro de una cadena (hasta 4 niveles).
pub fn unwrap_value(value: Value) -> Value {
    let mut current = value;
    for _ in 0..4 {
        let inner = match &current {
            Value::Object(map) if map.len() == 1 && map.contains_key("returnValue") => map.get("returnValue").cloned(),
            Value::String(text) if text.trim_start().starts_with(['{', '[']) => serde_json::from_str(text).ok(),
            _ => None,
        };
        match inner {
            Some(next) => current = next,
            None => break,
        }
    }
    current
}

/// `ToolResult` de un `result` de `tools/call`.
pub fn tool_result(result: &Value) -> ToolResult {
    let text: String = result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| blocks.iter().filter_map(|b| b.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    let structured = result.get("structuredContent").cloned();
    let value = structured.or_else(|| serde_json::from_str::<Value>(&text).ok()).map(unwrap_value);
    ToolResult { is_error: result.get("isError").and_then(Value::as_bool).unwrap_or(false), value, text }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("tests").join("fixtures").join("unreal-mcp").join("ue-5.8.2").join(name);
        std::fs::read_to_string(path).unwrap()
    }

    fn result_of(name: &str) -> Value {
        serde_json::from_str::<Value>(&fixture(name)).unwrap()["result"].clone()
    }

    /// @spec REQ-02
    #[test]
    fn interpreta_list_toolsets_en_texto_con_descripciones_de_varias_lineas() {
        let toolsets = parse_toolset_list(&tool_result(&result_of("list_toolsets.response.json")).text);
        assert_eq!(toolsets.len(), 22);
        assert_eq!(toolsets[1].name, "DBVMCPFirst.MyCustomSceneToolset");
        assert!(toolsets.iter().any(|t| t.name == "editor_toolset.toolsets.static_mesh.StaticMeshTools"));
        let automation = toolsets.iter().find(|t| t.name == "AutomationTestToolset.AutomationTestToolset").unwrap();
        assert!(automation.description.contains("DiscoverTests()"), "{}", automation.description);
    }

    /// @spec REQ-02
    #[test]
    fn interpreta_describe_toolset_y_acorta_los_nombres() {
        let info = parse_toolset_info(&tool_result(&result_of("describe_toolset.MyCustomSceneToolset.response.json")).text).unwrap();
        assert_eq!(info.name, "DBVMCPFirst.MyCustomSceneToolset");
        assert_eq!(info.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["RunPython", "GetActors", "CountActorsWithMesh"]);
        assert_eq!(info.tools[0].input_schema["required"][0], "code");
    }

    /// @spec REQ-02
    #[test]
    fn desenvuelve_el_returnvalue_doblemente_codificado() {
        let actors = tool_result(&result_of("call_tool.GetActors.response.json"));
        let value = actors.value.unwrap();
        assert!(value["actors"].as_array().unwrap().iter().any(|a| a["class"] == "DirectionalLight"), "{value}");
        let python = tool_result(&result_of("call_tool.RunPython.readonly.response.json")).value.unwrap();
        assert_eq!(python["success"], true);
        assert!(python["stdout"].as_str().unwrap().contains("5.8.2"));
        assert!(python["stderr"].as_str().unwrap().contains("DeprecationWarning"));
    }

    /// @spec REQ-02
    #[test]
    fn lee_respuestas_json_y_sse_y_los_errores_rpc() {
        assert_eq!(parse_rpc_body(r#"{"jsonrpc":"2.0","id":3,"result":{"ok":1}}"#, "application/json", 3).unwrap(), json!({"ok": 1}));
        let sse = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{\"x\":2}}\n\n";
        assert_eq!(parse_rpc_body(sse, "text/event-stream", 7).unwrap(), json!({"x": 2}));
        let error = parse_rpc_body(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#, "application/json", 1);
        assert_eq!(error, Err(UnrealError::Protocol("Method not found".into())));
    }

    /// Servidor MCP simulado: responde en orden con `responses` (cabeceras + cuerpo; `{{ID}}` se sustituye por el
    /// `id` JSON-RPC de la petición recibida) y devuelve lo recibido.
    pub(crate) fn fake_server(responses: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for response in responses {
                let (mut socket, _) = listener.accept().unwrap();
                let mut received = Vec::new();
                let mut buffer = [0u8; 8192];
                loop {
                    let read = socket.read(&mut buffer).unwrap();
                    received.extend_from_slice(&buffer[..read]);
                    let text = String::from_utf8_lossy(&received).to_string();
                    if let Some(end) = text.find("\r\n\r\n") {
                        let length = text[..end].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        if received.len() >= end + 4 + length {
                            break;
                        }
                    }
                    if read == 0 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&received).to_string();
                let id = request
                    .split_once("\r\n\r\n")
                    .and_then(|(_, body)| serde_json::from_str::<Value>(body).ok())
                    .and_then(|v| v.get("id").and_then(Value::as_u64))
                    .map(|id| id.to_string())
                    .unwrap_or_default();
                let body_start = response.find("\r\n\r\n").map(|i| i + 4).unwrap_or(response.len());
                let body = response[body_start..].replace("{{ID}}", &id);
                let head = response[..body_start].replace("CONTENT_LENGTH", &body.len().to_string());
                socket.write_all(format!("{head}{body}").as_bytes()).unwrap();
                seen.push(request);
            }
            seen
        });
        (url, handle)
    }

    /// Respuesta HTTP para `fake_server` (`CONTENT_LENGTH` se calcula tras sustituir `{{ID}}`).
    pub(crate) fn http(status: &str, headers: &str, body: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: CONTENT_LENGTH\r\nConnection: close\r\n\r\n{body}")
    }

    /// @spec REQ-02
    #[test]
    fn abre_sesion_reenvia_su_id_y_detecta_la_sesion_caducada() {
        let init = r#"{"jsonrpc":"2.0","id":{{ID}},"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}}}}"#;
        let (url, handle) = fake_server(vec![http("200 OK", "Mcp-Session-Id: abc123\r\n", init), http("202 Accepted", "", ""), http("404 Not Found", "", "")]);
        let session = McpSession::connect(&url).unwrap();
        assert_eq!(session.session_id.as_deref(), Some("abc123"));
        assert!(matches!(session.list_tools(), Err(UnrealError::Expired(_))));
        let seen = handle.join().unwrap();
        assert!(seen[1].contains("notifications/initialized"));
        assert!(seen[1].to_lowercase().contains("mcp-session-id: abc123"));
        assert!(seen[2].to_lowercase().contains("mcp-protocol-version: 2025-06-18"));
    }

    /// @spec REQ-02
    #[test]
    fn llama_a_una_herramienta_de_un_toolset_por_call_tool() {
        let init = r#"{"jsonrpc":"2.0","id":{{ID}},"result":{"protocolVersion":"2025-06-18"}}"#;
        let call = r#"{"jsonrpc":"2.0","id":{{ID}},"result":{"content":[{"type":"text","text":"{\"returnValue\":\"{\\\"n\\\": 3}\"}"}]}}"#;
        let (url, handle) = fake_server(vec![http("200 OK", "Mcp-Session-Id: s\r\n", init), http("202 Accepted", "", ""), http("200 OK", "", call)]);
        let session = McpSession::connect(&url).unwrap();
        let result = session.call_toolset_tool("DBVMCPFirst.MyCustomSceneToolset", "CountActorsWithMesh", json!({"meshName": "SM_Cube"}), DEFAULT_TIMEOUT).unwrap();
        assert_eq!(result.value, Some(json!({"n": 3})));
        let seen = handle.join().unwrap();
        let body: Value = serde_json::from_str(seen[2].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["method"], "tools/call");
        assert_eq!(body["params"]["name"], "call_tool");
        assert_eq!(body["params"]["arguments"]["toolset_name"], "DBVMCPFirst.MyCustomSceneToolset");
        assert_eq!(body["params"]["arguments"]["tool_name"], "CountActorsWithMesh");
        assert_eq!(body["params"]["arguments"]["arguments"]["meshName"], "SM_Cube");
    }

    /// @spec REQ-02, RNF-03
    #[test]
    fn un_editor_apagado_se_explica_como_inalcanzable() {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        assert!(matches!(McpSession::connect(&endpoint_url(port, "/mcp")), Err(UnrealError::Unreachable(_))));
    }

    /// Latencia que añade el cliente a una llamada (serializar, HTTP local, interpretar y desenvolver): < 50 ms.
    /// @spec RNF-05
    #[test]
    fn el_cliente_anade_menos_de_50_ms_por_llamada() {
        const CALLS: usize = 20;
        let init = r#"{"jsonrpc":"2.0","id":{{ID}},"result":{"protocolVersion":"2025-06-18"}}"#;
        let call = r#"{"jsonrpc":"2.0","id":{{ID}},"result":{"content":[{"type":"text","text":"{\"returnValue\":\"{\\\"n\\\": 3}\"}"}]}}"#;
        let mut responses = vec![http("200 OK", "Mcp-Session-Id: s\r\n", init), http("202 Accepted", "", "")];
        responses.extend((0..CALLS).map(|_| http("200 OK", "", call)));
        let (url, handle) = fake_server(responses);
        let session = McpSession::connect(&url).unwrap();
        let started = std::time::Instant::now();
        for _ in 0..CALLS {
            assert_eq!(session.call_toolset_tool("T", "x", json!({}), DEFAULT_TIMEOUT).unwrap().value, Some(json!({"n": 3})));
        }
        let average = started.elapsed() / CALLS as u32;
        handle.join().unwrap();
        assert!(average < Duration::from_millis(50), "media por llamada: {average:?}");
    }

    /// Contra el editor real (no corre por defecto): `DBV_UNREAL_MCP_PORT=8000 cargo test -- --ignored en_vivo`.
    /// @spec REQ-02
    #[test]
    #[ignore]
    fn en_vivo_lista_toolsets_y_lee_actores() {
        let port: u16 = std::env::var("DBV_UNREAL_MCP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8000);
        let session = McpSession::connect(&endpoint_url(port, "/mcp")).unwrap();
        let tools = session.list_tools().unwrap();
        assert!(tools.iter().any(|t| t.name == "call_tool"));
        let toolsets = session.list_toolsets().unwrap();
        assert!(toolsets.len() >= 3, "{toolsets:?}");
        let scene = toolsets.iter().find(|t| t.name.ends_with("SceneTools")).expect("SceneTools");
        let info = session.describe_toolset(&scene.name).unwrap();
        assert!(info.tools.iter().any(|t| t.name == "find_actors"));
    }
}
