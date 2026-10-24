// =============================================================================
// DBV Unreal AI — Servidor MCP propio por stdio (`--mcp`)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-20, REQ-11
//
// `dbv-unreal-ai --mcp [--port <n>] [--project <ruta.uproject>]` arranca, en lugar de la ventana,
// un servidor MCP por stdio (patrón de dbv-typst-editor RF-113): lo lanza el agente (Claude Code,
// Cursor, Gemini CLI… o la propia app para los agentes ACP) como proceso hijo, así que no abre
// ningún puerto de red (RNF-06). Expone:
//   · un PUENTE al servidor MCP del editor (`127.0.0.1:<puerto>`), con la validación del Python
//     antes de ejecutarlo (REQ-26);
//   · la biblioteca y «enviar a Unreal» (REQ-17, REQ-19);
//   · la generación con Tripo, que exige `confirm_cost: true` y respeta el tope de créditos (D6).
// La confirmación humana de cada llamada la pone el cliente MCP (todos piden permiso por herramienta).
//
// Reglas de stdio aprendidas en MCPCLiente (`bridge.py`): una respuesta JSON-RPC por línea, nunca
// `id: null` y nunca responder a una notificación.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use serde_json::{json, Value};

use crate::ai::secrets::Secrets;
use crate::generation::commands::GenerationSettings;
use crate::generation::tripo::{TextToModel, TripoClient, TEXT_TO_MODEL_CREDITS};
use crate::generation::{key_account, run_tripo_job, settings_path, within_budget, Polling};
use crate::library::Library;
use crate::unreal::discovery;
use crate::unreal::mcp_client::{endpoint_url, McpSession, ToolResult, DEFAULT_TIMEOUT};
use crate::unreal::UnrealError;

/// Versión del protocolo MCP que anunciamos si el cliente no pide otra.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Configuración del servidor (se calcula de los argumentos; los tests la montan a mano).
pub struct ServerConfig {
    pub unreal_url: String,
    pub library: Library,
    pub config_dir: PathBuf,
    /// Clave de un servicio (`tripo`), del almacén del sistema.
    pub key: Box<dyn Fn(&str) -> Option<String>>,
    /// URL de Tripo (cambia en los tests).
    pub tripo_base: Option<String>,
}

pub struct McpServer {
    config: ServerConfig,
    session: Option<McpSession>,
    spent: f64,
}

/// Lee `--flag valor`.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Configuración a partir de la línea de comandos y de las carpetas de la app.
pub fn config_from_args(args: &[String]) -> ServerConfig {
    let port = flag(args, "--port").and_then(|p| p.parse::<u16>().ok());
    let project_settings = flag(args, "--project").and_then(|p| PathBuf::from(p).parent().map(discovery::read_mcp_settings));
    let (port, path) = match (port, project_settings) {
        (Some(port), _) => (port, discovery::DEFAULT_MCP_PATH.to_string()),
        (None, Some(settings)) => (settings.port, settings.path),
        (None, None) => (discovery::DEFAULT_MCP_PORT, discovery::DEFAULT_MCP_PATH.to_string()),
    };
    let data = crate::paths::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let secrets = Secrets::default();
    ServerConfig {
        unreal_url: endpoint_url(port, &path),
        library: Library::new(crate::library::library_root(&data)),
        config_dir: crate::paths::config_dir().unwrap_or_else(|| PathBuf::from(".")),
        key: Box::new(move |service| secrets.get(&key_account(service))),
        tripo_base: None,
    }
}

const fn schema_object() -> &'static str {
    "object"
}

/// Herramientas que publica el servidor.
pub fn tool_specs() -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, required: Value| json!({ "name": name, "description": description, "inputSchema": { "type": schema_object(), "properties": properties, "required": required } });
    vec![
        tool("unreal_list_toolsets", "List the toolsets (groups of tools) of the connected Unreal Editor.", json!({}), json!([])),
        tool("unreal_describe_toolset", "Describe one Unreal Editor toolset: its tools and input schemas.", json!({ "toolset": { "type": "string" } }), json!(["toolset"])),
        tool(
            "unreal_call_tool",
            "Call a tool of an Unreal Editor toolset with its arguments.",
            json!({ "toolset": { "type": "string" }, "tool": { "type": "string" }, "arguments": { "type": "object" } }),
            json!(["toolset", "tool"]),
        ),
        tool(
            "unreal_run_python",
            "Run Python in the Unreal Editor (one undoable transaction). The code is first checked against the running engine API; unknown names are returned instead of running it.",
            json!({ "code": { "type": "string" } }),
            json!(["code"]),
        ),
        tool("unreal_describe_python_api", "Exact reference of an `unreal` Python API name in this engine version.", json!({ "name": { "type": "string" } }), json!(["name"])),
        tool("library_list", "List the generated assets of the DBV Unreal AI library.", json!({ "filter": { "type": "string" } }), json!([])),
        tool(
            "library_send_to_unreal",
            "Import a library asset into the Unreal project under /Game/DBV/Generated and place it in front of the editor camera.",
            json!({ "id": { "type": "string" }, "place": { "type": "boolean" } }),
            json!(["id"]),
        ),
        tool(
            "generate_3d_model",
            "Generate a 3D model with Tripo (costs about 30 credits). Ask the user first and pass confirm_cost=true only after they accept the cost.",
            json!({ "prompt": { "type": "string" }, "name": { "type": "string" }, "face_limit": { "type": "integer" }, "confirm_cost": { "type": "boolean" } }),
            json!(["prompt", "confirm_cost"]),
        ),
    ]
}

fn text_result(text: String, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

fn format(result: &ToolResult) -> String {
    result.value.as_ref().map(Value::to_string).unwrap_or_else(|| result.text.clone())
}

/// Nombre completo de un toolset: exacto o por su final si es único.
pub fn resolve_toolset(wanted: &str, names: &[String]) -> Option<String> {
    let lower = wanted.trim().to_lowercase();
    let exact = names.iter().find(|n| n.to_lowercase() == lower);
    let by_suffix: Vec<&String> = names.iter().filter(|n| n.to_lowercase().rsplit('.').next() == Some(lower.as_str())).collect();
    exact.cloned().or_else(|| if by_suffix.len() == 1 { Some(by_suffix[0].clone()) } else { None })
}

fn is_dbv(name: &str) -> bool {
    name.contains("DBVUnrealAIToolset") || name.contains("DBVUnrealAICppToolset")
}

impl McpServer {
    pub fn new(config: ServerConfig) -> Self {
        Self { config, session: None, spent: 0.0 }
    }

    fn session(&mut self) -> Result<McpSession, UnrealError> {
        let session = match self.session.clone() {
            Some(session) => session,
            None => McpSession::connect(&self.config.unreal_url)?,
        };
        self.session = Some(session.clone());
        Ok(session)
    }

    /// Llama al editor reabriendo la sesión una vez si caducó.
    fn editor<T>(&mut self, work: impl Fn(&McpSession) -> Result<T, UnrealError>) -> Result<T, UnrealError> {
        let first = work(&self.session()?);
        match first {
            Err(UnrealError::NotFound(m)) if m.contains("sesión MCP") => {
                self.session = None;
                work(&self.session()?)
            }
            other => other,
        }
    }

    fn toolset_names(&mut self) -> Result<Vec<String>, UnrealError> {
        Ok(self.editor(McpSession::list_toolsets)?.into_iter().map(|t| t.name).collect())
    }

    fn dbv_toolset(&mut self) -> Result<Option<String>, UnrealError> {
        Ok(self.toolset_names()?.into_iter().find(|n| is_dbv(n)))
    }

    /// Ejecuta una herramienta. `Err(texto)` vuelve como resultado con `isError`.
    fn call(&mut self, name: &str, args: &Value) -> Result<String, String> {
        let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let err = |e: UnrealError| e.to_string();
        let result = match name {
            "unreal_list_toolsets" => self.editor(McpSession::list_toolsets).map_err(err)?.iter().map(|t| format!("- {}: {}", t.name, t.description.lines().next().unwrap_or(""))).collect::<Vec<_>>().join("\n"),
            "unreal_describe_toolset" => {
                let names = self.toolset_names().map_err(err)?;
                let full = resolve_toolset(&text(args, "toolset"), &names).ok_or_else(|| format!("unknown toolset: {}", text(args, "toolset")))?;
                serde_json::to_string(&self.editor(|s| s.describe_toolset(&full)).map_err(err)?).map_err(|e| e.to_string())?
            }
            "unreal_call_tool" => {
                let names = self.toolset_names().map_err(err)?;
                let full = resolve_toolset(&text(args, "toolset"), &names).ok_or_else(|| format!("unknown toolset: {}", text(args, "toolset")))?;
                let tool = text(args, "tool");
                let arguments = args.get("arguments").cloned().unwrap_or_else(|| json!({}));
                let result = self.editor(|s| s.call_toolset_tool(&full, &tool, arguments.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
                if result.is_error {
                    return Err(format(&result));
                }
                format(&result)
            }
            "unreal_run_python" => self.run_python(&text(args, "code"))?,
            "unreal_describe_python_api" => {
                let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
                let target = text(args, "name");
                format(&self.editor(|s| s.call_toolset_tool(&dbv, "describe_python_api", json!({ "name": target }), DEFAULT_TIMEOUT)).map_err(err)?)
            }
            "library_list" => {
                let needle = text(args, "filter").to_lowercase();
                let entries: Vec<String> = self
                    .config
                    .library
                    .list()
                    .into_iter()
                    .filter(|e| needle.is_empty() || format!("{} {}", e.name, e.prompt).to_lowercase().contains(&needle))
                    .map(|e| format!("- {}: \"{}\" ({} {}, prompt: \"{}\")", e.id, e.name, e.provider, e.kind, e.prompt))
                    .collect();
                if entries.is_empty() {
                    "the library is empty or nothing matches".to_string()
                } else {
                    entries.join("\n")
                }
            }
            "library_send_to_unreal" => self.send_to_unreal(&text(args, "id"), args.get("place").and_then(Value::as_bool).unwrap_or(true))?,
            "generate_3d_model" => self.generate(args)?,
            other => return Err(format!("unknown tool: {other}")),
        };
        Ok(result)
    }

    fn run_python(&mut self, code: &str) -> Result<String, String> {
        if code.trim().is_empty() {
            return Err("empty code".into());
        }
        let err = |e: UnrealError| e.to_string();
        let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
        let code_value = json!({ "code": code });
        let report = self.editor(|s| s.call_toolset_tool(&dbv, "validate_python", code_value.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
        let blocking = report.value.as_ref().is_some_and(|v| v.get("ok") == Some(&json!(false)));
        if blocking {
            return Err(format!("NOT run: the code does not pass the check against this engine version. Fix it and retry: {}", format(&report)));
        }
        let ran = self.editor(|s| s.call_toolset_tool(&dbv, "run_python", code_value.clone(), std::time::Duration::from_secs(180))).map_err(err)?;
        Ok(format(&ran))
    }

    fn send_to_unreal(&mut self, id: &str, place: bool) -> Result<String, String> {
        let entry = self.config.library.get(id).ok_or_else(|| format!("no library asset with id {id}"))?;
        let file = entry.files.iter().find(|f| f.ends_with(".glb") || f.ends_with(".fbx") || f.ends_with(".obj")).ok_or("the asset has no model file")?;
        let path = self.config.library.file_path(id, file).map_err(|e| e.to_string())?;
        let err = |e: UnrealError| e.to_string();
        let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
        let name: String = entry.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let import_args = json!({ "source_file": path.to_string_lossy(), "folder_path": "/Game/DBV/Generated", "asset_name": name.trim_matches('_') });
        let imported = self.editor(|s| s.call_toolset_tool(&dbv, "import_model", import_args.clone(), std::time::Duration::from_secs(300))).map_err(err)?;
        let primary = imported.value.as_ref().and_then(|v| v.get("primary")).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("import failed: {}", format(&imported)))?;
        let mut message = format!("Imported as {primary}");
        if place {
            let place_args = json!({ "asset_path": primary, "distance": 300.0, "folder": "DBV/Generated" });
            let placed = self.editor(|s| s.call_toolset_tool(&dbv, "place_asset", place_args.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
            message.push_str(&format!(" and placed: {}", format(&placed)));
        }
        let project = self.editor(|s| s.call_toolset_tool(&dbv, "get_project_info", json!({}), DEFAULT_TIMEOUT)).ok().and_then(|r| r.value).and_then(|v| v.get("project_dir").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
        let _ = self.config.library.record_sent(id, &project, &primary);
        Ok(message)
    }

    fn generate(&mut self, args: &Value) -> Result<String, String> {
        if args.get("confirm_cost").and_then(Value::as_bool) != Some(true) {
            return Err(format!("not generated: this costs about {TEXT_TO_MODEL_CREDITS} Tripo credits. Ask the user and call again with confirm_cost=true only if they accept."));
        }
        let key = (self.config.key)("tripo").ok_or("there is no Tripo API key: add it in the DBV Unreal AI settings")?;
        let settings: GenerationSettings = std::fs::read_to_string(settings_path(&self.config.config_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if !within_budget(self.spent, TEXT_TO_MODEL_CREDITS, settings.budget_credits) {
            return Err(format!("not generated: it would exceed the session cap of {} credits", settings.budget_credits));
        }
        let prompt = args.get("prompt").and_then(Value::as_str).unwrap_or("").to_string();
        let request = TextToModel { prompt: prompt.clone(), model: None, texture_quality: None, face_limit: args.get("face_limit").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()), smart_low_poly: None, pbr: None };
        let name = args.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or_else(|| prompt.chars().take(40).collect());
        let client = match &self.config.tripo_base {
            Some(base) => TripoClient::with_base_url(&key, base),
            None => TripoClient::new(&key),
        };
        self.spent += TEXT_TO_MODEL_CREDITS;
        let entry = run_tripo_job(&client, &request, &name, &self.config.library, &AtomicBool::new(false), Polling::default(), |_, _| {}).map_err(|e| e.to_string())?;
        Ok(format!("Generated and saved in the library: {} (\"{}\"). Use library_send_to_unreal to import it.", entry.id, entry.name))
    }

    /// Respuesta a una línea JSON-RPC, o `None` si no toca responder (notificaciones, líneas vacías).
    pub fn handle(&mut self, line: &str) -> Option<String> {
        let message: Value = match serde_json::from_str(line.trim()) {
            Ok(value) => value,
            Err(_) if line.trim().is_empty() => return None,
            Err(error) => return Some(json!({ "jsonrpc": "2.0", "id": 0, "error": { "code": -32700, "message": format!("parse error: {error}") } }).to_string()),
        };
        // Sin `id` es una notificación: nunca se responde (JSON-RPC 2.0).
        let id = message.get("id").filter(|id| !id.is_null())?.clone();
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let outcome: Result<Value, (i64, String)> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL_VERSION),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "dbv-unreal-ai", "title": "DBV Unreal AI", "version": env!("CARGO_PKG_VERSION") }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_specs() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                Ok(match self.call(&name, &arguments) {
                    Ok(text) => text_result(text, false),
                    Err(text) => text_result(text, true),
                })
            }
            other => Err((-32601, format!("method not found: {other}"))),
        };
        let response = match outcome {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
        };
        Some(response.to_string())
    }
}

/// Bucle de stdio. Devuelve el código de salida del proceso.
pub fn run(args: &[String]) -> i32 {
    let mut server = McpServer::new(config_from_args(args));
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(response) = server.handle(&line) {
            // `to_string` de serde_json nunca mete saltos de línea: una respuesta por línea.
            if writeln!(stdout, "{response}").and_then(|_| stdout.flush()).is_err() {
                break;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryEntry;

    fn server(dir: &std::path::Path) -> McpServer {
        McpServer::new(ServerConfig {
            unreal_url: "http://127.0.0.1:1/mcp".into(),
            library: Library::new(dir.join("library")),
            config_dir: dir.to_path_buf(),
            key: Box::new(|_| None),
            tripo_base: None,
        })
    }

    fn rpc(server: &mut McpServer, method: &str, params: Value) -> Value {
        serde_json::from_str(&server.handle(&json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params }).to_string()).unwrap()).unwrap()
    }

    /// @spec REQ-20
    #[test]
    fn se_presenta_y_lista_sus_herramientas() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let init = rpc(&mut s, "initialize", json!({ "protocolVersion": "2025-03-26" }));
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(init["result"]["serverInfo"]["name"], "dbv-unreal-ai");
        let tools = rpc(&mut s, "tools/list", json!({}));
        let names: Vec<&str> = tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"unreal_run_python") && names.contains(&"generate_3d_model") && names.contains(&"library_send_to_unreal"));
    }

    /// @spec REQ-20
    #[test]
    fn respeta_las_reglas_de_stdio() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        assert_eq!(s.handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#), None, "no se responde a una notificación");
        assert_eq!(s.handle(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#), None, "nunca se responde con id null");
        assert_eq!(s.handle("   "), None);
        let response = s.handle(r#"{"jsonrpc":"2.0","id":"a","method":"ping"}"#).unwrap();
        assert!(!response.contains('\n'), "una respuesta por línea");
        assert_eq!(serde_json::from_str::<Value>(&response).unwrap()["id"], "a");
        let unknown = rpc(&mut s, "resources/list", json!({}));
        assert_eq!(unknown["error"]["code"], -32601);
    }

    /// @spec REQ-20, RNF-04
    #[test]
    fn generar_exige_confirmar_el_coste_y_una_clave() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let call = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre" } }));
        assert_eq!(call["result"]["isError"], true);
        assert!(call["result"]["content"][0]["text"].as_str().unwrap().contains("confirm_cost=true"));
        let no_key = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre", "confirm_cost": true } }));
        assert!(no_key["result"]["content"][0]["text"].as_str().unwrap().contains("no Tripo API key"));
    }

    /// @spec REQ-20, REQ-17
    #[test]
    fn lista_la_biblioteca_y_explica_un_editor_apagado() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.library.add(LibraryEntry { id: "cofre-1".into(), name: "Cofre".into(), provider: "tripo".into(), kind: "model".into(), prompt: "treasure chest".into(), parameters: Value::Null, task_id: "t".into(), created_at: 1, credits: None, files: vec!["model.glb".into()], thumbnail: None, sent_to: vec![] }).unwrap();
        let listed = rpc(&mut s, "tools/call", json!({ "name": "library_list", "arguments": { "filter": "chest" } }));
        assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains("cofre-1"));
        let offline = rpc(&mut s, "tools/call", json!({ "name": "unreal_list_toolsets", "arguments": {} }));
        assert_eq!(offline["result"]["isError"], true);
        assert!(offline["result"]["content"][0]["text"].as_str().unwrap().contains("no se pudo conectar"));
    }

    #[test]
    fn resuelve_toolsets_por_su_final() {
        let names = vec!["editor_toolset.toolsets.scene.SceneTools".to_string(), "EditorToolset.EditorAppToolset".to_string()];
        assert_eq!(resolve_toolset("SceneTools", &names).as_deref(), Some("editor_toolset.toolsets.scene.SceneTools"));
        assert_eq!(resolve_toolset("editortoolset.editorapptoolset", &names).as_deref(), Some("EditorToolset.EditorAppToolset"));
        assert_eq!(resolve_toolset("Nope", &names), None);
    }

    #[test]
    fn lee_los_argumentos_de_la_linea_de_comandos() {
        let args: Vec<String> = ["app", "--mcp", "--port", "8123"].iter().map(|s| s.to_string()).collect();
        assert_eq!(config_from_args(&args).unreal_url, "http://127.0.0.1:8123/mcp");
    }
}
