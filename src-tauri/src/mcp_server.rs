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
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::ai::secrets::Secrets;
use crate::generation::commands::GenerationSettings;
use crate::generation::jobs::{describe, resume_orphans, start_jobs, Cancels, JobBoard, JobContext, JobRecord};
use crate::generation::tripo::TripoClient;
use crate::generation::{key_account, tools, within_budget, Polling};
use crate::project_memory::tools as memory_tools;
use crate::library::{now_secs, Library};
use crate::unreal::discovery;
use crate::unreal::mcp_client::{endpoint_url, McpSession, ToolResult, DEFAULT_TIMEOUT};
use crate::unreal::UnrealError;

/// Versión del protocolo MCP que anunciamos si el cliente no pide otra.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Clave de un servicio (`tripo`) del almacén del sistema.
pub type KeyLookup = Box<dyn Fn(&str) -> Option<String>>;

/// Configuración del servidor (se calcula de los argumentos; los tests la montan a mano).
pub struct ServerConfig {
    pub unreal_url: String,
    pub library: Library,
    pub config_dir: PathBuf,
    /// Carpeta de datos de la app: ahí viven los ficheros de trabajo que lee la ventana (REQ-27).
    pub data_dir: PathBuf,
    /// Cómo se espera a Tripo (cambia en los tests).
    pub polling: Polling,
    /// Cuánto espera como máximo una llamada a `generate_3d_model` / `generation_jobs` antes de devolver el estado.
    pub max_wait: Duration,
    /// Clave de un servicio (`tripo`), del almacén del sistema.
    pub key: KeyLookup,
    /// URL de Tripo (cambia en los tests).
    pub tripo_base: Option<String>,
    /// Carpeta del proyecto (`--project`): la de su memoria `dbv-unreal-ai/` (REQ-38).
    pub project_dir: Option<PathBuf>,
}

pub struct McpServer {
    config: ServerConfig,
    session: Option<McpSession>,
    board: JobBoard,
    cancels: Cancels,
    /// Arranque de este proceso: inicio de sesión si la ventana no está abierta.
    started_at: u64,
}

/// Espera por defecto de una llamada larga: por debajo del tiempo límite habitual de los clientes MCP.
pub const DEFAULT_MAX_WAIT: Duration = Duration::from_secs(45);

/// Lee `--flag valor`.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Configuración a partir de la línea de comandos y de las carpetas de la app.
pub fn config_from_args(args: &[String]) -> ServerConfig {
    let port = flag(args, "--port").and_then(|p| p.parse::<u16>().ok());
    let project_dir = flag(args, "--project").and_then(|p| PathBuf::from(p).parent().map(PathBuf::from));
    let project_settings = project_dir.as_deref().map(discovery::read_mcp_settings);
    let (port, path) = match (port, project_settings) {
        (Some(port), _) => (port, discovery::DEFAULT_MCP_PATH.to_string()),
        (None, Some(settings)) => (settings.port, settings.path),
        (None, None) => (discovery::DEFAULT_MCP_PORT, discovery::DEFAULT_MCP_PATH.to_string()),
    };
    let data = crate::paths::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let config_dir = crate::paths::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let secrets = Secrets::default();
    ServerConfig {
        unreal_url: endpoint_url(port, &path),
        library: Library::new(GenerationSettings::load(&config_dir).library_root(&data)),
        config_dir,
        data_dir: data,
        polling: Polling::default(),
        max_wait: DEFAULT_MAX_WAIT,
        key: Box::new(move |service| secrets.get(&key_account(service))),
        tripo_base: None,
        project_dir,
    }
}

const fn schema_object() -> &'static str {
    "object"
}

/// Herramientas que publica el servidor.
pub fn tool_specs() -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, required: Value| json!({ "name": name, "description": description, "inputSchema": { "type": schema_object(), "properties": properties, "required": required } });
    let mut specs = vec![
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
        tool(
            "library_list",
            "List the assets of the DBV Unreal AI library (generated or imported): id, name, kind (model, rig, animation, image…), source asset, files and prompt. Use the id with rig_model, animate_model, tripo_run or library_send_to_unreal.",
            json!({ "filter": { "type": "string" } }),
            json!([]),
        ),
        tool(
            "library_rename",
            "Give a library asset a short, meaningful name in the user's language.",
            json!({ "id": { "type": "string" }, "name": { "type": "string" } }),
            json!(["id", "name"]),
        ),
        tool(
            "library_import",
            "Copy into the library a model (.glb, .gltf, .fbx, .obj, .usdz) or an image (.png, .jpg, .webp) from a local path, with the files it needs. Use it for models the user already has, then library_send_to_unreal or rig_model.",
            json!({ "path": { "type": "string" }, "name": { "type": "string" } }),
            json!(["path"]),
        ),
        tool(
            "library_send_to_unreal",
            "Import a library asset into the Unreal project under /Game/DBV/Generated and place it in front of the editor camera.",
            json!({ "id": { "type": "string" }, "place": { "type": "boolean" } }),
            json!(["id"]),
        ),
        tool(
            "generation_jobs",
            "Status and progress of the generation jobs of this session (also those started from the app). With wait_seconds it waits for the running ones to finish (up to 45 s per call) and returns their result. Use it after generate_3d_model returned running jobs.",
            json!({ "job_id": { "type": "string", "description": "Only this job." }, "wait_seconds": { "type": "integer", "minimum": 0, "maximum": 45 } }),
            json!([]),
        ),
    ];
    // Las de Tripo y las de la memoria vienen de una única definición, compartida con el asistente integrado (D10).
    specs.extend(tools::specs(true));
    specs.extend(memory_tools::specs());
    specs
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
        let board = JobBoard::new(&config.data_dir);
        Self { config, session: None, board, cancels: Cancels::default(), started_at: now_secs() }
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
            Err(UnrealError::Expired(_)) => {
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
                    Err(format(&result))?
                } else {
                    format(&result)
                }
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
                    .map(|e| format!("- {}: \"{}\" ({} {}{}, files: {}, prompt: \"{}\")", e.id, e.name, e.provider, e.kind, e.parent.as_ref().map(|p| format!(" from {p}")).unwrap_or_default(), e.files.join(", "), e.prompt))
                    .collect();
                if entries.is_empty() {
                    "the library is empty or nothing matches".to_string()
                } else {
                    entries.join("\n")
                }
            }
            "library_rename" => {
                let entry = self.config.library.rename(&text(args, "id"), &text(args, "name")).map_err(err)?;
                format!("Renamed {} to \"{}\"", entry.id, entry.name)
            }
            "library_import" => {
                let name = text(args, "name");
                let entry = self.config.library.import_file(std::path::Path::new(&text(args, "path")), Some(name.as_str()).filter(|n| !n.is_empty())).map_err(err)?;
                format!("Imported as {} (\"{}\", {}: {})", entry.id, entry.name, entry.kind, entry.files.join(", "))
            }
            "library_send_to_unreal" => self.send_to_unreal(&text(args, "id"), args.get("place").and_then(Value::as_bool).unwrap_or(true))?,
            "generation_jobs" => self.jobs(args)?,
            "tripo_operations" => tools::operations_text(args)?,
            "tripo_balance" => self.balance()?,
            launcher if tools::LAUNCHERS.contains(&launcher) => self.launch(launcher, args)?,
            "project_memory_read" => memory_tools::read_text(&self.project_dir()?, args)?,
            "project_memory_propose" => memory_tools::propose_and_save(&self.project_dir()?, args)?,
            "project_tasks_list" => memory_tools::tasks_text(&self.project_dir()?)?,
            "project_tasks_propose" => memory_tools::propose_plan_and_save(&self.project_dir()?, args)?,
            other => Err(format!("unknown tool: {other}"))?,
        };
        Ok(result)
    }

    /// Carpeta del proyecto, que el agente recibe con `--project` (REQ-38).
    fn project_dir(&self) -> Result<PathBuf, String> {
        self.config.project_dir.clone().ok_or_else(|| "no project: this server was started without --project <.uproject>".to_string())
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
        let result = if blocking {
            Err(format!("NOT run: the code does not pass the check against this engine version. Fix it and retry: {}", format(&report)))
        } else {
            self.editor(|s| s.call_toolset_tool(&dbv, "run_python", code_value.clone(), std::time::Duration::from_secs(180))).map(|ran| format(&ran)).map_err(err)
        };
        result
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
        let report = imported.value.clone().unwrap_or(Value::Null);
        let animations: Vec<String> = report.get("animations").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
        let mut message = format!("Imported as {primary}");
        if place {
            let place_args = json!({ "asset_path": primary, "distance": 300.0, "folder": "DBV/Generated" });
            let placed = self.editor(|s| s.call_toolset_tool(&dbv, "place_asset", place_args.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
            message.push_str(&format!(" and placed: {}", format(&placed)));
            // Un personaje de Tripo con animaciones se pone a reproducir la primera en el viewport (toolset 0.2.0).
            let actor = placed.value.as_ref().and_then(|v| v.get("actor")).and_then(Value::as_str).map(str::to_string);
            if let (Some(actor), Some(first), true) = (actor, animations.first(), report.get("kind").and_then(Value::as_str) == Some("skeletal")) {
                let play_args = json!({ "actor_label": actor, "animation_path": first, "looping": true });
                let played = self.editor(|s| s.call_toolset_tool(&dbv, "play_animation", play_args.clone(), DEFAULT_TIMEOUT));
                message.push_str(&match played {
                    Ok(result) if !result.is_error => format!(". Skeletal mesh with {} animation(s); playing {first}", animations.len()),
                    _ => format!(". Skeletal mesh with {} animation(s); the project's DBV toolset is outdated, so it is not playing: ask the user to prepare the project again in the app", animations.len()),
                });
            }
        }
        let project = self.editor(|s| s.call_toolset_tool(&dbv, "get_project_info", json!({}), DEFAULT_TIMEOUT)).ok().and_then(|r| r.value).and_then(|v| v.get("project_dir").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
        let _ = self.config.library.record_sent(id, &project, &primary);
        Ok(message)
    }

    fn session_spent(&self) -> f64 {
        self.board.spent_since(self.board.session_start(self.started_at))
    }

    /// Espera a que los trabajos `ids` terminen (o se acabe `wait`) y devuelve su estado.
    fn wait_for(&self, ids: &[String], wait: Duration) -> Vec<JobRecord> {
        let started = Instant::now();
        loop {
            let records: Vec<JobRecord> = ids.iter().filter_map(|id| self.board.get(id)).collect();
            if records.iter().all(|r| !r.is_active()) || started.elapsed() >= wait {
                break records;
            }
            std::thread::sleep(Duration::from_millis(400));
        }
    }

    /// Texto para el modelo: el estado de cada trabajo y qué hacer a continuación.
    fn report(&self, records: &[JobRecord]) -> Result<String, String> {
        let lines: Vec<String> = records.iter().map(|r| describe(r, &self.config.library)).collect();
        let running = records.iter().any(JobRecord::is_active);
        let done = records.iter().any(|r| r.stage == "done");
        let text = if running {
            format!("Still running (the user sees the progress in the app):\n{}\nCall generation_jobs with wait_seconds=45 to keep waiting.", lines.join("\n"))
        } else if done {
            format!("{}\nThe models are in the library. Show them to the user before bringing one into Unreal with library_send_to_unreal.", lines.join("\n"))
        } else {
            lines.join("\n")
        };
        if running || done { Ok(text) } else { Err(text) }
    }

    fn tripo_key(&self) -> Result<String, String> {
        (self.config.key)("tripo").ok_or_else(|| "there is no Tripo API key: add it in the DBV Unreal AI settings".to_string())
    }

    fn client(&self) -> Result<TripoClient, String> {
        let key = self.tripo_key()?;
        Ok(match &self.config.tripo_base {
            Some(base) => TripoClient::with_base_url(&key, base),
            None => TripoClient::new(&key),
        })
    }

    fn balance(&self) -> Result<String, String> {
        let balance = self.client()?.balance().map_err(|e| e.to_string())?;
        Ok(format!("Tripo balance: {} credits available ({} reserved by running tasks). This session has used about {} credits.", balance.balance, balance.frozen, self.session_spent()))
    }

    /// Cualquier herramienta de Tripo que gasta: traduce, exige confirmar el coste TOTAL, respeta el tope y lanza.
    fn launch(&mut self, tool: &str, args: &Value) -> Result<String, String> {
        let launch = tools::build(tool, args, &self.config.library)?;
        let total = launch.total();
        let free = total <= 0.0;
        if !free && args.get("confirm_cost").and_then(Value::as_bool) != Some(true) {
            let each = if launch.variants > 1 { format!(" ({} x {})", launch.variants, total / f64::from(launch.variants)) } else { String::new() };
            return Err(format!("not run: {} costs about {total} Tripo credits{each}. Tell the user and call again with confirm_cost=true only if they accept.", launch.operation.name));
        }
        let client_key = self.tripo_key()?;
        let settings = GenerationSettings::load(&self.config.config_dir);
        let spent = self.session_spent();
        if !within_budget(spent, total, settings.budget_credits) {
            return Err(format!("not run: it would exceed the session cap of {} credits ({spent} already used)", settings.budget_credits));
        }
        let context = self.job_context(client_key);
        let ids = start_jobs(&context, launch.operation, &launch.params, &launch.name, launch.variants, launch.parent.clone());
        self.report(&self.wait_for(&ids, self.config.max_wait))
    }

    fn job_context(&self, key: String) -> JobContext {
        JobContext {
            board: self.board.clone(),
            library: self.config.library.clone(),
            key,
            base_url: self.config.tripo_base.clone(),
            polling: self.config.polling,
            cancels: self.cancels.clone(),
            hook: Arc::new(|_| {}),
            source: "agent".into(),
        }
    }

    fn jobs(&mut self, args: &Value) -> Result<String, String> {
        // Un trabajo cuyo proceso murió (otra sesión del agente) se reanuda: su tarea sigue en Tripo.
        if let Ok(key) = self.tripo_key() {
            resume_orphans(&self.job_context(key));
        }
        let wait = Duration::from_secs(args.get("wait_seconds").and_then(Value::as_u64).unwrap_or(0).min(self.config.max_wait.as_secs()));
        let since = self.board.session_start(self.started_at);
        let ids: Vec<String> = match args.get("job_id").and_then(Value::as_str) {
            Some(id) => vec![id.to_string()],
            None => self.board.list().into_iter().filter(|r| r.started_at >= since).map(|r| r.id).take(20).collect(),
        };
        if ids.is_empty() {
            return Ok("no generation jobs in this session".into());
        }
        let records = self.wait_for(&ids, wait);
        if records.is_empty() {
            return Err(format!("no job {}", ids.join(", ")));
        }
        // Aquí un fallo no es un error de la herramienta: es el estado que se pidió.
        self.report(&records).or_else(Ok)
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
            data_dir: dir.to_path_buf(),
            polling: Polling { interval: Duration::from_millis(1), max_wait: Duration::from_secs(5) },
            max_wait: Duration::from_secs(5),
            key: Box::new(|_| None),
            tripo_base: None,
            project_dir: Some(dir.join("project")),
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
        for expected in ["unreal_run_python", "generate_3d_model", "image_to_3d_model", "rig_model", "animate_model", "tripo_run", "tripo_operations", "tripo_balance", "generation_jobs", "library_send_to_unreal"] {
            assert!(names.contains(&expected), "{expected}");
        }
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
        let call = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre", "name": "Cofre", "variants": 3 } }));
        assert_eq!(call["result"]["isError"], true);
        let text = call["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("confirm_cost=true") && text.contains("60") && text.contains("3 x 20"), "avisa del coste TOTAL de las variantes: {text}");
        let no_key = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre", "name": "Cofre", "confirm_cost": true } }));
        assert!(no_key["result"]["content"][0]["text"].as_str().unwrap().contains("no Tripo API key"));
        // REQ-28: sin nombre no se genera; el modelo debe dar uno corto, no el prompt.
        let nameless = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre", "confirm_cost": true } }));
        assert!(nameless["result"]["content"][0]["text"].as_str().unwrap().contains("`name` is required"));
    }

    /// Un agente lee y guarda la memoria del proyecto que le dio `--project`; el asistente integrado ve las
    /// mismas herramientas (una sola definición). @spec REQ-38, REQ-35
    #[test]
    fn el_agente_lee_y_guarda_la_memoria_del_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let listed = rpc(&mut s, "tools/list", json!({}));
        let names: Vec<String> = listed["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        for spec in memory_tools::specs() {
            assert!(names.contains(&spec["name"].as_str().unwrap().to_string()), "{}", spec["name"]);
        }
        let none = rpc(&mut s, "tools/call", json!({ "name": "project_memory_read", "arguments": {} }));
        assert!(none["result"]["content"][0]["text"].as_str().unwrap().contains("has no memory yet"));
        std::fs::create_dir_all(dir.path().join("project")).unwrap();
        crate::project_memory::activate(&dir.path().join("project")).unwrap();
        let saved = rpc(&mut s, "tools/call", json!({ "name": "project_memory_propose", "arguments": { "section": "Conventions", "entry": "Props go in /Game/DBV/Props" } }));
        assert_eq!(saved["result"]["isError"], false, "{saved}");
        let read = rpc(&mut s, "tools/call", json!({ "name": "project_memory_read", "arguments": { "file": "memory" } }));
        assert!(read["result"]["content"][0]["text"].as_str().unwrap().contains("- Props go in /Game/DBV/Props"));
    }

    /// @spec REQ-20, REQ-17
    #[test]
    fn lista_la_biblioteca_y_explica_un_editor_apagado() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.library.add(LibraryEntry { id: "cofre-1".into(), name: "Cofre".into(), provider: "tripo".into(), kind: "model".into(), prompt: "treasure chest".into(), parameters: Value::Null, task_id: "t".into(), created_at: 1, credits: None, files: vec!["model.glb".into()], thumbnail: None, sent_to: vec![], parent: None, output: Value::Null }).unwrap();
        let listed = rpc(&mut s, "tools/call", json!({ "name": "library_list", "arguments": { "filter": "chest" } }));
        assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains("cofre-1"));
        let offline = rpc(&mut s, "tools/call", json!({ "name": "unreal_list_toolsets", "arguments": {} }));
        assert_eq!(offline["result"]["isError"], true);
        assert!(offline["result"]["content"][0]["text"].as_str().unwrap().contains("no se pudo conectar"));
    }

    /// Si Tripo rechaza la petición no se crea tarea: el tope de la sesión no se consume (hallazgo de `/code-simplify`).
    /// @spec RNF-04
    #[test]
    fn un_rechazo_de_tripo_no_consume_el_tope() {
        use crate::unreal::mcp_client::tests::{fake_server, http};
        let (api, handle) = fake_server(vec![http("401 Unauthorized", "", r#"{"code":1002,"message":"invalid api key"}"#)]);
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.key = Box::new(|_| Some("sk-mala".into()));
        s.config.tripo_base = Some(api.trim_end_matches("/mcp").to_string());
        let call = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "cofre", "name": "Cofre", "confirm_cost": true } }));
        assert_eq!(call["result"]["isError"], true);
        assert!(call["result"]["content"][0]["text"].as_str().unwrap().contains("invalid api key"));
        assert_eq!(s.session_spent(), 0.0);
        handle.join().unwrap();
    }

    /// El agente lanza variantes: los trabajos quedan en ficheros con la petición exacta (la ventana los muestra),
    /// la herramienta devuelve el estado y `generation_jobs` sigue esperando. @spec REQ-27, REQ-29, REQ-33
    /// Tripo simulado que responde según la petición y no por orden de llegada: con variantes, los hilos
    /// consultan en cualquier orden. `POST` crea una tarea; `GET /tasks/…` la da por terminada; `GET /m.glb` sirve el modelo.
    fn fake_tripo(connections: usize) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let model_url = format!("{base}/m.glb");
        let handle = std::thread::spawn(move || {
            for n in 0..connections {
                let (mut socket, _) = listener.accept().unwrap();
                // La petición entera (cabeceras + cuerpo): responder antes de leerla cierra la conexión a medias.
                let mut received = Vec::new();
                let mut buffer = [0u8; 8192];
                loop {
                    let read = socket.read(&mut buffer).unwrap_or(0);
                    received.extend_from_slice(&buffer[..read]);
                    let text = String::from_utf8_lossy(&received).to_string();
                    let complete = text.find("\r\n\r\n").is_some_and(|end| {
                        let length = text[..end].lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                        received.len() >= end + 4 + length
                    });
                    if complete || read == 0 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&received).to_string();
                let body = if request.starts_with("POST") {
                    format!(r#"{{"code":0,"data":{{"task_id":"t-{n}"}}}}"#)
                } else if request.starts_with("GET /m.glb") {
                    "glb".to_string()
                } else {
                    format!(r#"{{"code":0,"data":{{"status":"success","progress":100,"output":{{"model_url":"{model_url}"}}}}}}"#)
                };
                let _ = write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        (base, handle)
    }

    #[test]
    fn genera_variantes_en_segundo_plano_y_deja_la_consola_en_ficheros() {
        // Dos variantes: dos POST, dos GET de estado y dos descargas.
        let (api, api_handle) = fake_tripo(6);
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.key = Box::new(|_| Some("sk-ok".into()));
        s.config.tripo_base = Some(api);
        let call = rpc(&mut s, "tools/call", json!({ "name": "generate_3d_model", "arguments": { "prompt": "stylized pirate in T-pose", "name": "Pirata", "variants": 2, "confirm_cost": true } }));
        let text = call["result"]["content"][0]["text"].as_str().unwrap().to_string();
        assert_eq!(call["result"]["isError"], false, "{text}");
        assert!(text.contains("\"Pirata\"") && text.contains("\"Pirata 2\""), "{text}");
        assert_eq!(s.config.library.list().len(), 2);
        let jobs = s.board.list();
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().all(|j| j.source == "agent" && j.stage == "done" && j.request["prompt"] == "stylized pirate in T-pose" && j.log.iter().any(|l| l.contains("POST /generation/text-to-model"))));
        assert!(!serde_json::to_string(&jobs).unwrap().contains("sk-ok"), "la clave nunca llega a los ficheros de trabajo");
        assert_eq!(s.session_spent(), 40.0, "2 × 20 (texto con textura, tabla oficial)");
        let listed = rpc(&mut s, "tools/call", json!({ "name": "generation_jobs", "arguments": {} }));
        assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains("[done"));
        api_handle.join().unwrap();
    }

    /// Lo que el agente no podía hacer: riggear el pirata y ponerlo a bailar. Cada paso usa la tarea de Tripo de su
    /// origen, cuesta lo de la tabla y deja un asset enlazado al anterior. @spec REQ-16, REQ-33
    #[test]
    fn riggea_y_anima_un_modelo_de_la_biblioteca() {
        // Tres operaciones: POST + GET de estado + descarga en cada una.
        let (api, api_handle) = fake_tripo(9);
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.key = Box::new(|_| Some("sk-ok".into()));
        s.config.tripo_base = Some(api);
        let call = |s: &mut McpServer, name: &str, args: Value| {
            let response = rpc(s, "tools/call", json!({ "name": name, "arguments": args }));
            (response["result"]["isError"] == true, response["result"]["content"][0]["text"].as_str().unwrap_or("").to_string())
        };
        let (_, made) = call(&mut s, "generate_3d_model", json!({ "prompt": "pirate in T-pose", "name": "Pirata", "confirm_cost": true }));
        let model = s.config.library.list()[0].clone();
        assert!(made.contains(&model.id));

        let (refused, price) = call(&mut s, "rig_model", json!({ "id": model.id }));
        assert!(refused && price.contains("25"), "sin confirmar no gasta y dice el precio: {price}");
        let (failed, rigged) = call(&mut s, "rig_model", json!({ "id": model.id, "confirm_cost": true }));
        assert!(!failed, "{rigged}");
        let rig = s.config.library.list().into_iter().find(|e| e.kind == "rig").unwrap();
        assert_eq!((rig.name.as_str(), rig.parent.as_deref()), ("Pirata (rig)", Some(model.id.as_str())));

        let (failed, danced) = call(&mut s, "animate_model", json!({ "id": rig.id, "animations": ["dance_01", "wave_goodbye_01"], "confirm_cost": true }));
        assert!(!failed, "{danced}");
        let animation = s.config.library.list().into_iter().find(|e| e.kind == "animation").unwrap();
        assert_eq!(animation.parent.as_deref(), Some(rig.id.as_str()));
        let jobs = s.board.list();
        let rig_job = jobs.iter().find(|j| j.kind == "rig").unwrap();
        assert_eq!(rig_job.request["input"], model.task_id, "el rig usa la tarea del modelo");
        assert_eq!(rig_job.request["spec"], "tripo");
        let retarget = jobs.iter().find(|j| j.kind == "retarget").unwrap();
        assert_eq!(retarget.request["input"], rig.task_id, "las animaciones usan la tarea del rig");
        assert_eq!(retarget.request["animations"], json!(["preset:biped:dance_01", "preset:biped:wave_goodbye_01"]));
        assert_eq!(s.session_spent(), 20.0 + 25.0 + 20.0);
        api_handle.join().unwrap();
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
