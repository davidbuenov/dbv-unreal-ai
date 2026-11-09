// =============================================================================
// DBV Unreal AI — Servidor MCP propio por stdio (`--mcp`)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-20, REQ-11, REQ-51, REQ-81, REQ-82
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
use crate::generation::worldlabs::WorldLabsClient;
use crate::generation::{key_account, service_keys, tools, within_budget, Polling};
use crate::project_memory::tools as memory_tools;
use crate::library::{now_secs, Library};
use crate::unreal::discovery;
use crate::unreal::compact;
use crate::unreal::mcp_client::{endpoint_url, McpSession, ToolResult, ToolsetSummary, DEFAULT_TIMEOUT};
use crate::unreal::UnrealError;

/// Cómo usar este servidor sin gastar contexto (REQ-51): lo recibe el cliente en `initialize`.
pub const INSTRUCTIONS: &str = "DBV Unreal AI bridges the Unreal Editor of one project. Every result you get stays in your context, so keep them small: \
use unreal_scene_summary with name_filter/class_filter/limit instead of listing every actor; unreal_describe_toolset gives one line per tool, and with tool=\"<name>\" the full schema of that one tool; \
prefer ONE unreal_run_python that does several things and sets `result` to a compact value over many small calls; capture the viewport only when you need to look (detail=\"high\" only to read small labels). \
Use only this server for Unreal even if another one (unreal-mcp) is configured: this one checks Python against the engine and goes through the user's approvals.";

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
    /// `list_toolsets` del editor, pedido una vez por proceso (REQ-51): se vuelve a pedir si la sesión caduca o si
    /// se nombra un toolset que no está.
    toolsets: Option<Vec<ToolsetSummary>>,
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
        // La misma carpeta que la ventana; `run` la crea y la migra (aquí no: los tests llaman a esta función).
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

/// Herramientas que solo consultan (REQ-12): se publican con `readOnlyHint` y, cuando un agente de la conversación
/// pide permiso para usarlas, la app lo concede sola. Lo que modifica, ejecuta Python o gasta créditos sigue
/// preguntando (pedido por el usuario el 2026-11-01: las aprobaciones de simples consultas eran muy pesadas).
pub const READ_ONLY_TOOLS: [&str; 18] = [
    "unreal_list_toolsets",
    "unreal_describe_toolset",
    "unreal_capture_viewport",
    "unreal_scene_summary",
    "unreal_describe_python_api",
    "unreal_list_metahumans",
    "library_list",
    "generation_jobs",
    "tripo_operations",
    "tripo_balance",
    "worldlabs_balance",
    "project_memory_read",
    "project_tasks_list",
    "level_zones_list",
    "recipes_list",
    "recipe_check",
    "search_unreal_guides",
    "convai_characters",
];

/// Las herramientas de solo lectura, para que la ventana apruebe sola sus permisos. @spec REQ-12
#[tauri::command]
pub fn app_mcp_read_only_tools() -> Vec<String> {
    READ_ONLY_TOOLS.iter().map(|name| name.to_string()).collect()
}

/// Herramientas que publica el servidor.
pub fn tool_specs() -> Vec<Value> {
    let tool = |name: &str, description: &str, properties: Value, required: Value| json!({ "name": name, "description": description, "inputSchema": { "type": schema_object(), "properties": properties, "required": required } });
    let mut specs = vec![
        tool("unreal_list_toolsets", "List the toolsets (groups of tools) of the connected Unreal Editor, one line each.", json!({}), json!([])),
        tool(
            "unreal_describe_toolset",
            "Describe one Unreal Editor toolset in a compact form: one line per tool with its parameters (* = required) and first sentence. Pass tool=\"<name>\" to get the full description and input schema of that one tool. Do not describe toolsets you do not need: some are huge.",
            json!({ "toolset": { "type": "string" }, "tool": { "type": "string", "description": "Only this tool, with its full input schema." } }),
            json!(["toolset"]),
        ),
        tool(
            "unreal_call_tool",
            "Call a tool of an Unreal Editor toolset with its arguments. Pass EVERY parameter of the tool: optional ones as null (the editor rejects a missing one with \"needs a default value\").",
            json!({ "toolset": { "type": "string" }, "tool": { "type": "string" }, "arguments": { "type": "object" } }),
            json!(["toolset", "tool"]),
        ),
        tool(
            "unreal_run_python",
            "Run Python in the Unreal Editor (one undoable transaction). The code is first checked against the running engine API; unknown names are returned instead of running it. Prefer ONE call that does several things and assigns a compact value to `result` (counts, names, a short dict) over many calls or printing large data.",
            json!({ "code": { "type": "string" } }),
            json!(["code"]),
        ),
        tool(
            "unreal_capture_viewport",
            "Look at the level viewport of the Unreal Editor: returns an image (and the camera) to check what you placed or changed. With annotate=true it draws a ground grid (2 m cells, coordinates in meters) and labels the nearest actors, useful to place things. The image is 1024 px wide by default; detail=\"high\" (1568 px) only to read small labels. Each image stays in your context: capture when you need to look. Read-only.",
            json!({ "annotate": { "type": "boolean" }, "detail": { "type": "string", "enum": ["normal", "high"] } }),
            json!([]),
        ),
        tool(
            "unreal_scene_summary",
            "Actors of the open level: name, class, location and mesh, compact. Filter with name_filter / class_filter and keep limit small (default 100). Use it instead of GetActors to look at the scene. Read-only.",
            json!({ "name_filter": { "type": "string" }, "class_filter": { "type": "string" }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } }),
            json!([]),
        ),
        tool("unreal_describe_python_api", "Exact reference of an `unreal` Python API name in this engine version.", json!({ "name": { "type": "string" } }), json!(["name"])),
        tool(
            "library_list",
            "List the assets of the DBV Unreal AI library (models, rigs, animations, images, and world scenarios with visual Gaussian Splat .spz and physical collider mesh .glb): id, name, kind, files and prompt. Use the id with rig_model, animate_model, tripo_run or library_send_to_unreal.",
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
            "unreal_import_sound",
            "Import an audio or sound effect file (.wav, .mp3, .ogg) into Unreal under /Game/DBV/Audio as a USoundWave asset.",
            json!({ "path": { "type": "string", "description": "Absolute path to the audio file on disk." }, "asset_name": { "type": "string", "description": "Optional asset name." } }),
            json!(["path"]),
        ),
        tool(
            "library_send_to_unreal",
            "Import a library asset into the Unreal project under /Game/DBV/Generated and place it in front of the editor camera. For a MetaHuman garment, pass `metahuman` (the label of the character to dress, from unreal_list_metahumans) to auto-rig and equip it; without it the garment is only imported and never equipped. For 3D models it imports the mesh; for World Labs scenarios it imports the collider mesh (Static Mesh with collision) for ground and physics.",
            json!({ "id": { "type": "string" }, "place": { "type": "boolean" }, "metahuman": { "type": "string", "description": "For a MetaHuman garment: label of the MetaHuman to dress (from unreal_list_metahumans). Never guess it: with several MetaHumans and no indication, ask the user." } }),
            json!(["id"]),
        ),
        tool(
            "unreal_list_metahumans",
            "List the MetaHuman characters in the open level: actor label, Body SkeletalMesh, build (e.g. f_med_nrw, m_tal_nrw) and current outfit slots (Torso, Legs, Feet). Read-only.",
            json!({}),
            json!([]),
        ),
        tool(
            "unreal_equip_metahuman_outfit",
            "Auto-rig a 3D garment (library asset id, local .glb/.fbx path, or /Game/... StaticMesh) onto a MetaHuman character in the open level using GeometryScript INPAINT_WEIGHTS and equip it in the Torso, Legs or Feet slot with set_leader_pose_component(Body).",
            json!({
                "actor_label": { "type": "string", "description": "Label of the MetaHuman to dress, from unreal_list_metahumans. Required: nothing is equipped without a target. If the user has not said which and there are several, ask them." },
                "source": { "type": "string", "description": "Library asset id, local .glb/.fbx path, or /Game/... StaticMesh path." },
                "slot": { "type": "string", "enum": ["Torso", "Legs", "Feet"], "description": "Outfit slot (default: Torso, or the library asset's garment_slot)." },
                "scale_multiplier": { "type": "number", "description": "Scale margin over the target body slot (default 1.05)." },
                "z_offset": { "type": "number", "description": "Vertical offset in cm (default 0.0)." }
            }),
            json!(["actor_label", "source"]),
        ),
        tool(
            "unreal_playtest",
            "Run an autonomous Play-In-Editor (PIE) playtest: starts PIE, drives the PlayerPawn across Slate ticks (forward/backward/left/right/jump), measures start/end location, distance moved, whether it fell out of the world, average FPS and recent Warning/Error log entries, captures the PIE viewport screenshot, and always stops PIE.",
            json!({
                "duration_seconds": { "type": "integer", "minimum": 1, "maximum": 15, "description": "Playtest duration in seconds (1-15, default 5)." },
                "actions": { "type": "array", "items": { "type": "string", "enum": ["forward", "backward", "left", "right", "jump"] }, "description": "Movement actions to apply during the playtest (default: [\"forward\", \"jump\"])." }
            }),
            json!([]),
        ),
        tool(
            crate::unreal::compile::MCP_TOOL,
            crate::unreal::compile::MCP_DESCRIPTION,
            json!({ "files": { "type": "array", "items": { "type": "string" }, "description": "Files you changed, relative to the project folder." } }),
            json!(["files"]),
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
    // Zonas del nivel (REQ-57): la misma definición que el asistente integrado.
    specs.extend(crate::zones::specs());
    // Recetas guiadas (REQ-23): la misma definición que el asistente integrado.
    specs.extend(crate::recipes::specs());
    // Guías de Unreal sin red (REQ-25).
    specs.push(crate::guides::spec());
    // Personajes de Convai registrados en Ajustes (REQ-58).
    specs.push(crate::convai_characters::spec());
    // Anotación estándar de MCP: algunos agentes ya no preguntan por lo que solo consulta.
    for spec in &mut specs {
        let read_only = spec["name"].as_str().is_some_and(|name| READ_ONLY_TOOLS.contains(&name));
        spec["annotations"] = json!({ "readOnlyHint": read_only });
    }
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
        Self { config, session: None, toolsets: None, board, cancels: Cancels::default(), started_at: now_secs() }
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
                // Un editor reiniciado puede traer otros toolsets.
                self.session = None;
                self.toolsets = None;
                work(&self.session()?)
            }
            other => other,
        }
    }

    /// Toolsets del editor, de la caché salvo que `fresh` pida volver a preguntar. @spec REQ-51
    fn toolset_list(&mut self, fresh: bool) -> Result<Vec<ToolsetSummary>, UnrealError> {
        let list = match self.toolsets.clone().filter(|_| !fresh) {
            Some(cached) => cached,
            None => self.editor(McpSession::list_toolsets)?,
        };
        self.toolsets = Some(list.clone());
        Ok(list)
    }

    fn toolset_names(&mut self) -> Result<Vec<String>, UnrealError> {
        Ok(self.toolset_list(false)?.into_iter().map(|t| t.name).collect())
    }

    /// Nombre completo de un toolset; si no está en la caché, se vuelve a preguntar al editor una vez.
    fn resolve(&mut self, wanted: &str) -> Result<String, String> {
        let err = |e: UnrealError| e.to_string();
        let cached = self.toolset_names().map_err(err)?;
        let found = match resolve_toolset(wanted, &cached) {
            Some(name) => Some(name),
            None => resolve_toolset(wanted, &self.toolset_list(true).map_err(err)?.into_iter().map(|t| t.name).collect::<Vec<_>>()),
        };
        found.ok_or_else(|| format!("unknown toolset: {wanted}"))
    }

    fn dbv_toolset(&mut self) -> Result<Option<String>, UnrealError> {
        Ok(self.toolset_names()?.into_iter().find(|n| is_dbv(n)))
    }

    /// Ejecuta una herramienta. `Err(texto)` vuelve como resultado con `isError`.
    fn call(&mut self, name: &str, args: &Value) -> Result<String, String> {
        let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let err = |e: UnrealError| e.to_string();
        let result = match name {
            "unreal_list_toolsets" => self.toolset_list(false).map_err(err)?.iter().map(|t| format!("- {}: {}", t.name, t.description.lines().next().unwrap_or(""))).collect::<Vec<_>>().join("\n"),
            "unreal_describe_toolset" => {
                let full = self.resolve(&text(args, "toolset"))?;
                let info = self.editor(|s| s.describe_toolset(&full)).map_err(err)?;
                let tool = text(args, "tool");
                if tool.is_empty() {
                    compact::compact_toolset(&info)
                } else {
                    compact::tool_detail(&info, &tool)?
                }
            }
            "unreal_scene_summary" => {
                let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100).clamp(1, 500);
                let scene_args = json!({ "name_filter": text(args, "name_filter"), "class_filter": text(args, "class_filter"), "limit": limit });
                format(&self.editor(|s| s.call_toolset_tool(&dbv, "get_scene_summary", scene_args.clone(), DEFAULT_TIMEOUT)).map_err(err)?)
            }
            "unreal_call_tool" => {
                let full = self.resolve(&text(args, "toolset"))?;
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
                    .map(|e| {
                        let details = if e.kind == "world" || e.provider == "worldlabs" {
                            let splat = e.parameters.get("splat_file").and_then(Value::as_str)
                                .or_else(|| e.files.iter().find(|f| f.ends_with(".spz") || f.ends_with(".ply")).map(String::as_str));
                            let collider = e.parameters.get("collider_file").and_then(Value::as_str)
                                .or_else(|| e.files.iter().find(|f| f.ends_with(".glb") || f.ends_with(".fbx")).map(String::as_str));
                            let mut parts = Vec::new();
                            if let Some(s) = splat { parts.push(format!("visual Gaussian Splat: {s}")); }
                            if let Some(c) = collider { parts.push(format!("physical collider mesh: {c}")); }
                            if let Some(scale) = e.parameters.get("metric_scale_factor").and_then(Value::as_f64) {
                                parts.push(format!("scale: {scale}"));
                            }
                            if parts.is_empty() { String::new() } else { format!(" [{}]", parts.join(", ")) }
                        } else {
                            String::new()
                        };
                        format!("- {}: \"{}\" ({} {}{}{}, files: {}, prompt: \"{}\")", e.id, e.name, e.provider, e.kind, e.parent.as_ref().map(|p| format!(" from {p}")).unwrap_or_default(), details, e.files.join(", "), e.prompt)
                    })
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
            "library_send_to_unreal" => self.send_to_unreal(&text(args, "id"), args.get("place").and_then(Value::as_bool).unwrap_or(true), text(args, "metahuman").trim())?,
            "unreal_list_metahumans" => self.list_metahumans()?,
            "unreal_equip_metahuman_outfit" => self.equip_metahuman_outfit(args)?,
            "unreal_import_sound" => {
                let path = text(args, "path");
                let asset_name = text(args, "asset_name");
                let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
                let res = self.editor(|s| s.call_toolset_tool(&dbv, "import_sound", json!({ "source_file": path, "folder_path": "/Game/DBV/Audio", "asset_name": asset_name }), std::time::Duration::from_secs(60))).map_err(err)?;
                res.value.map(|v| v.to_string()).unwrap_or(res.text)
            }
            "generation_jobs" => self.jobs(args)?,
            // REQ-22: el agente edita con sus herramientas; aquí solo se compila (con su permiso, que pone el cliente).
            crate::unreal::compile::MCP_TOOL => {
                let files: Vec<String> = args.get("files").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
                let dir = self.project_dir()?;
                crate::unreal::compile::compile_for_agent(&dir, &files, || self.editor(crate::unreal::compile::live_coding)).map_err(err)?
            }
            "tripo_operations" => tools::operations_text(args)?,
            "tripo_balance" => self.balance()?,
            "worldlabs_balance" => self.worldlabs_balance()?,
            launcher if tools::LAUNCHERS.contains(&launcher) => self.launch(launcher, args)?,
            "project_memory_read" => memory_tools::read_text(&self.project_dir()?, args)?,
            "project_memory_propose" => memory_tools::propose_and_save(&self.project_dir()?, args)?,
            "project_tasks_list" => memory_tools::tasks_text(&self.project_dir()?)?,
            "project_tasks_propose" => memory_tools::propose_plan_and_save(&self.project_dir()?, args)?,
            // En el MCP la aprobación la pone el cliente (permiso por herramienta, D6); el código es fijo de la app.
            "level_zones_propose" => {
                let zones = crate::zones::validate(args)?;
                crate::zones::report_text(&self.run_fixed(&crate::zones::apply_code(&zones))?)
            }
            "level_zones_list" => crate::zones::report_text(&self.run_fixed(&crate::zones::list_code())?),
            "search_unreal_guides" => crate::guides::search_text(&self.config.config_dir, &text(args, "query"), args.get("limit").and_then(Value::as_u64).unwrap_or(3) as usize),
            "recipes_list" | "recipe_check" => crate::recipes::run_tool(name, args, |code| self.run_fixed(code))?,
            "convai_characters" => crate::convai_characters::text(&crate::convai_characters::load(&self.config.config_dir)),
            "project_task_update" => {
                let dir = self.project_dir()?;
                let done = args.get("done").and_then(Value::as_bool).unwrap_or(true);
                crate::project_memory::checks::update_task(&dir, &text(args, "task"), done, true, |code| self.check_in_editor(code))?.message
            }
            other => Err(format!("unknown tool: {other}"))?,
        };
        Ok(result)
    }

    /// Ejecuta en el editor una comprobación de tarea (código FIJO de la app, solo lectura: REQ-37) y devuelve lo
    /// que dejó en `result`. No pasa por `validate_python`: no lo escribió la IA.
    fn check_in_editor(&mut self, code: &str) -> Result<Value, String> {
        let err = |e: UnrealError| e.to_string();
        let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
        let ran = self.editor(|s| s.call_toolset_tool(&dbv, "run_python", json!({ "code": code }), DEFAULT_TIMEOUT)).map_err(err)?;
        ran.value.ok_or_else(|| format!("unexpected answer from the editor: {}", ran.text))
    }

    /// Anota cuánto devolvió una llamada, para el contador de la ventana (REQ-51). Solo tamaños, nunca contenido.
    fn record_usage(&self, tool: &str, result: &Value) {
        let content = result.get("content").and_then(Value::as_array).cloned().unwrap_or_default();
        let texts: Vec<&str> = content.iter().filter_map(|c| c.get("text").and_then(Value::as_str)).collect();
        let entry = crate::tool_usage::UsageRecord {
            at: crate::tool_usage::now_ms(),
            pid: std::process::id(),
            project: crate::tool_usage::project_key(&self.config.project_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()),
            tool: tool.to_string(),
            text: texts.iter().map(|t| t.chars().count()).sum(),
            images: content.iter().filter(|c| c.get("type").and_then(Value::as_str) == Some("image")).count(),
            clipped: texts.iter().any(|t| t.contains("… (truncated: ")),
        };
        crate::tool_usage::record(&self.config.data_dir, &entry);
    }

    /// Ejecuta en el editor código FIJO de la app (zonas, REQ-57): no lo escribió la IA, así que no pasa por
    /// `validate_python`. Devuelve el informe de `run_python`.
    fn run_fixed(&mut self, code: &str) -> Result<Value, String> {
        let err = |e: UnrealError| e.to_string();
        let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
        let ran = self.editor(|s| s.call_toolset_tool(&dbv, "run_python", json!({ "code": code }), std::time::Duration::from_secs(120))).map_err(err)?;
        ran.value.ok_or_else(|| format!("unexpected answer from the editor: {}", ran.text))
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

    /// «Enviar a Unreal» desde un agente, con la misma elección de ficheros que la ventana (D25). `metahuman` es el
    /// personaje que se viste si la entrada es una prenda (vacío: solo se importa). @spec REQ-19, REQ-54, REQ-83
    fn send_to_unreal(&mut self, id: &str, place: bool, metahuman: &str) -> Result<String, String> {
        let entry = self.config.library.get(id).ok_or_else(|| format!("no library asset with id {id}"))?;
        let err = |e: UnrealError| e.to_string();
        let dbv = self.dbv_toolset().map_err(err)?.ok_or("the DBV Unreal AI toolset is not installed: prepare the project in the app")?;
        let name: String = entry.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let name = name.trim_matches('_').to_string();
        let result = match crate::library::send_plan(&entry) {
            crate::library::SendPlan::Splat { splat, collider } => self.send_splat(&entry, &dbv, &name, &splat, collider.as_deref(), place),
            crate::library::SendPlan::Model { model } => self.send_model(&entry, &dbv, &name, &model, place, metahuman),
            crate::library::SendPlan::Sound { sound } => self.send_sound(&entry, &dbv, &name, &sound),
            crate::library::SendPlan::Nothing => Err("the asset has no model (.glb, .gltf, .fbx, .obj), audio (.wav) or Gaussian Splat (.spz, .ply) file".to_string()),
        };
        result
    }

    /// Importa un sonido a /Game/DBV/Audio como USoundWave. @spec REQ-72
    fn send_sound(&mut self, entry: &crate::library::LibraryEntry, dbv: &str, name: &str, sound: &str) -> Result<String, String> {
        let sound_path = self.config.library.file_path(&entry.id, sound).map_err(|e| e.to_string())?;
        let args = json!({
            "source_file": sound_path.to_string_lossy(),
            "folder_path": "/Game/DBV/Audio",
            "asset_name": name,
        });
        let imported = self.editor(|s| s.call_toolset_tool(dbv, "import_sound", args.clone(), std::time::Duration::from_secs(60))).map_err(|e| e.to_string())?;
        let report = imported.value.clone().unwrap_or(Value::Null);
        if imported.is_error || report.get("success") != Some(&json!(true)) {
            Err(format!("the sound was not imported: {}", format(&imported)))
        } else {
            let primary = report.get("primary").and_then(Value::as_str).unwrap_or(name);
            if let Some(dir) = &self.config.project_dir {
                let _ = self.config.library.record_sent(&entry.id, &dir.to_string_lossy(), primary);
            }
            Ok(format!("Sound imported as USoundWave at {primary}"))
        }
    }

    /// Espera para cargar un Gaussian Splat: un .ply de 3DGS pasa con facilidad de 2 GB. @spec REQ-54
    const SPLAT_TIMEOUT_SECS: u64 = 600;

    /// Un Gaussian Splat con el componente del toolset DBV y, si la hay, su malla de colisión. @spec REQ-54
    fn send_splat(&mut self, entry: &crate::library::LibraryEntry, dbv: &str, name: &str, splat: &str, collider: Option<&str>, place: bool) -> Result<String, String> {
        let splat_path = self.config.library.file_path(&entry.id, splat).map_err(|e| e.to_string())?;
        let label = format!("{name}_Splat");
        let splat_args = json!({ "source_file": splat_path.to_string_lossy(), "actor_label": label });
        // Un .ply de varios GB tarda en leerse; si aun así se agota la espera, el editor sigue cargándolo.
        let loaded = self.editor(|s| s.call_toolset_tool(dbv, "load_gaussian_splat", splat_args.clone(), std::time::Duration::from_secs(Self::SPLAT_TIMEOUT_SECS))).map_err(|e| match e {
            UnrealError::Timeout(_) => "the editor is still loading the Gaussian Splat (large file): check with unreal_scene_summary in a minute before sending it again".to_string(),
            other => other.to_string(),
        })?;
        let report = loaded.value.clone().unwrap_or(Value::Null);
        let result = if loaded.is_error || report.get("success") != Some(&json!(true)) {
            let missing = loaded.text.contains("load_gaussian_splat") && loaded.text.to_lowercase().contains("not found");
            Err(if missing { "the DBV toolset of this project cannot load Gaussian Splats yet: ask the user to prepare the project again in the app".to_string() } else { format!("the splat was not loaded: {}", format(&loaded)) })
        } else {
            let message = format!("Gaussian Splat loaded as actor {}", report.get("actor").and_then(Value::as_str).unwrap_or(&label));
            // Un mundo de World Labs con escala y suelo se coloca con `place_world` (REQ-18): su colisión se importa sin
            // colocarla delante de la cámara, y luego se alinea con el splat.
            let is_placed_world = place && crate::library::world_metadata(entry).is_some();
            let (collider_note, collider_asset) = match collider {
                Some(collider) => self.send_collider(entry, dbv, name, collider, place && !is_placed_world)?,
                None => (String::new(), None),
            };
            let world_note = match crate::library::world_place_code(entry, &label, collider_asset.as_deref(), name).filter(|_| place) {
                Some(code) => self.run_fixed(&code).map(|placed| format!("; world placed at its metric scale with the ground at Z = 0: {placed}"))?,
                None => String::new(),
            };
            self.record_sent(&entry.id, dbv, &label);
            Ok(format!("{message}{collider_note}{world_note}"))
        };
        result
    }

    /// La malla de colisión de un mundo, importada y, si se pide, colocada. Devuelve la nota para el mensaje y el asset.
    fn send_collider(&mut self, entry: &crate::library::LibraryEntry, dbv: &str, name: &str, collider: &str, place: bool) -> Result<(String, Option<String>), String> {
        let err = |e: UnrealError| e.to_string();
        let mesh = self.config.library.file_path(&entry.id, collider).map_err(|e| e.to_string())?;
        let import_args = json!({ "source_file": mesh.to_string_lossy(), "folder_path": "/Game/DBV/Generated", "asset_name": format!("{name}_Collider") });
        let imported = self.editor(|s| s.call_toolset_tool(dbv, "import_model", import_args.clone(), std::time::Duration::from_secs(300))).map_err(err)?;
        let primary = imported.value.as_ref().and_then(|v| v.get("primary")).and_then(Value::as_str).map(str::to_string);
        let note = match (primary.clone(), place) {
            (Some(primary), true) => {
                let place_args = json!({ "asset_path": primary, "distance": 0.0, "folder": "DBV/Generated" });
                let placed = self.editor(|s| s.call_toolset_tool(dbv, "place_asset", place_args.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
                format!(", with its collider mesh {primary} placed: {}", format(&placed))
            }
            (Some(primary), false) => format!(", with its collider mesh imported as {primary}"),
            (None, _) => format!("; the collider mesh was not imported: {}", format(&imported)),
        };
        Ok((note, primary))
    }

    /// Anota en la biblioteca a qué proyecto se envió.
    fn record_sent(&mut self, id: &str, dbv: &str, asset: &str) {
        let project = self.editor(|s| s.call_toolset_tool(dbv, "get_project_info", json!({}), DEFAULT_TIMEOUT)).ok().and_then(|r| r.value).and_then(|v| v.get("project_dir").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
        let _ = self.config.library.record_sent(id, &project, asset);
    }

    /// Un modelo (Static o Skeletal Mesh) importado y colocado delante de la cámara; una prenda solo se equipa en el
    /// MetaHuman indicado y, sin él, se importa y se avisa (nunca se elige «el primero»). @spec REQ-19, REQ-81, REQ-83
    fn send_model(&mut self, entry: &crate::library::LibraryEntry, dbv: &str, name: &str, file: &str, place: bool, metahuman: &str) -> Result<String, String> {
        let id = entry.id.as_str();
        let path = self.config.library.file_path(id, file).map_err(|e| e.to_string())?;
        let err = |e: UnrealError| e.to_string();
        // Mismas carpetas que `sendToUnreal.js`: una prenda va con el `SK_` que crea `equip_metahuman_outfit`.
        let folder = if entry.parameters.get("garment_slot").is_some_and(Value::is_string) { "/Game/DBV/MetaHumanOutfits" } else { "/Game/DBV/Generated" };
        let import_args = json!({ "source_file": path.to_string_lossy(), "folder_path": folder, "asset_name": name });
        let imported = self.editor(|s| s.call_toolset_tool(dbv, "import_model", import_args.clone(), std::time::Duration::from_secs(300))).map_err(err)?;
        let primary = imported.value.as_ref().and_then(|v| v.get("primary")).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("import failed: {}", format(&imported)))?;
        let report = imported.value.clone().unwrap_or(Value::Null);
        let animations: Vec<String> = report.get("animations").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
        let mut message = format!("Imported as {primary}");
        let garment_slot = entry.parameters.get("garment_slot").and_then(Value::as_str);
        if place && garment_slot.is_some() {
            let slot = garment_slot.unwrap_or("Torso");
            if metahuman.is_empty() {
                message.push_str(". It is a MetaHuman garment and no target was given, so it was only imported and NOT equipped: call unreal_list_metahumans, ask the user which character to dress (or use the only one and say so) and call unreal_equip_metahuman_outfit");
            } else {
                let ran = self.run_fixed(&Self::equip_metahuman_code(metahuman, &primary, slot, 1.05, 0.0))?;
                let res = ran.get("result").cloned().unwrap_or(Value::Null);
                if res.get("success") != Some(&json!(true)) {
                    let reason = res.get("error").and_then(Value::as_str).unwrap_or("the garment could not be equipped");
                    return Err(format!("imported as {primary}, but not equipped: {reason}"));
                }
                let actor = res.get("actor").and_then(Value::as_str).unwrap_or(metahuman);
                let slot = res.get("slot").and_then(Value::as_str).unwrap_or(slot);
                let rigged = res.get("rigged_asset").and_then(Value::as_str).unwrap_or(&primary);
                message.push_str(&format!(" and auto-rigged + equipped on MetaHuman {actor} ({slot}) as {rigged}"));
            }
        } else if place {
            let place_args = json!({ "asset_path": primary, "distance": 300.0, "folder": "DBV/Generated" });
            let placed = self.editor(|s| s.call_toolset_tool(dbv, "place_asset", place_args.clone(), DEFAULT_TIMEOUT)).map_err(err)?;
            message.push_str(&format!(" and placed: {}", format(&placed)));
            // Un personaje de Tripo con animaciones se pone a reproducir la primera en el viewport (toolset 0.2.0).
            let actor = placed.value.as_ref().and_then(|v| v.get("actor")).and_then(Value::as_str).map(str::to_string);
            if let (Some(actor), Some(first), true) = (actor, animations.first(), report.get("kind").and_then(Value::as_str) == Some("skeletal")) {
                let play_args = json!({ "actor_label": actor, "animation_path": first, "looping": true });
                let played = self.editor(|s| s.call_toolset_tool(dbv, "play_animation", play_args.clone(), DEFAULT_TIMEOUT));
                message.push_str(&match played {
                    Ok(result) if !result.is_error => format!(". Skeletal mesh with {} animation(s); playing {first}", animations.len()),
                    _ => format!(". Skeletal mesh with {} animation(s); the project's DBV toolset is outdated, so it is not playing: ask the user to prepare the project again in the app", animations.len()),
                });
            }
        }
        self.record_sent(id, dbv, &primary);
        Ok(message)
    }

    fn hex_json(payload: &Value) -> String {
        serde_json::to_string(payload).unwrap_or_else(|_| "{}".into()).bytes().map(|b| format!("{b:02x}")).collect()
    }

    /// Código fijo para riggear y equipar una prenda en un MetaHuman del nivel. @spec REQ-81
    pub fn equip_metahuman_code(actor_label: &str, garment_source: &str, slot: &str, scale_multiplier: f64, z_offset: f64) -> String {
        let hex = Self::hex_json(&json!({
            "actor_label": actor_label,
            "garment_source": garment_source,
            "slot": slot,
            "scale_multiplier": scale_multiplier,
            "z_offset": z_offset,
        }));
        format!(
            "import json\nimport dbv_unreal_ai.editor as _e\n_p = json.loads(bytes.fromhex(\"{hex}\").decode(\"utf-8\"))\nresult = _e.equip_metahuman_outfit(actor_label=_p[\"actor_label\"], garment_source=_p[\"garment_source\"], slot=_p[\"slot\"], scale_multiplier=float(_p[\"scale_multiplier\"]), z_offset=float(_p[\"z_offset\"]))\n"
        )
    }

    /// Lista los actores MetaHuman del nivel abierto. @spec REQ-81
    fn list_metahumans(&mut self) -> Result<String, String> {
        let ran = self.run_fixed("import dbv_unreal_ai.editor as _e\nresult = _e.list_metahumans()\n")?;
        if ran.get("success") == Some(&json!(true)) {
            Ok(ran.get("result").cloned().unwrap_or(Value::Null).to_string())
        } else {
            Err(ran.get("error").and_then(Value::as_str).unwrap_or("failed to list MetaHumans").to_string())
        }
    }

    /// Riggea y equipa una prenda en un MetaHuman del nivel. @spec REQ-81
    fn equip_metahuman_outfit(&mut self, args: &Value) -> Result<String, String> {
        let raw_source = args.get("source").and_then(Value::as_str).unwrap_or("").trim();
        if raw_source.is_empty() {
            return Err("`source` is required (library asset id, local .glb/.fbx path, or /Game/... StaticMesh path)".into());
        }
        // REQ-83: sin destino no se hace nada; el editor enumera los MetaHumans disponibles en el error.
        let actor_label = args.get("actor_label").and_then(Value::as_str).unwrap_or("").trim();
        let explicit_slot = args.get("slot").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
        let (garment_source, inferred_slot) = match self.config.library.get(raw_source) {
            Some(entry) => {
                let model = entry
                    .files
                    .iter()
                    .find(|f| f.ends_with(".glb") || f.ends_with(".gltf") || f.ends_with(".fbx") || f.ends_with(".obj"))
                    .ok_or_else(|| format!("library asset {raw_source} has no 3D mesh file"))?;
                let path = self.config.library.file_path(&entry.id, model).map_err(|e| e.to_string())?;
                let slot = entry.parameters.get("garment_slot").and_then(Value::as_str).unwrap_or("Torso").to_string();
                (path.to_string_lossy().to_string(), slot)
            }
            None => (raw_source.to_string(), "Torso".to_string()),
        };
        let slot = explicit_slot.unwrap_or(&inferred_slot);
        let scale_multiplier = args.get("scale_multiplier").and_then(Value::as_f64).unwrap_or(1.05).clamp(0.5, 2.0);
        let z_offset = args.get("z_offset").and_then(Value::as_f64).unwrap_or(0.0).clamp(-200.0, 200.0);
        let ran = self.run_fixed(&Self::equip_metahuman_code(actor_label, &garment_source, slot, scale_multiplier, z_offset))?;
        let res = ran.get("result").cloned().unwrap_or(Value::Null);
        if ran.get("success") == Some(&json!(true)) && res.get("success") != Some(&json!(false)) {
            Ok(res.to_string())
        } else {
            let msg = res.get("error").and_then(Value::as_str).or_else(|| ran.get("error").and_then(Value::as_str)).unwrap_or("failed to equip MetaHuman outfit");
            Err(msg.to_string())
        }
    }

    /// Ejecuta un playtest autónomo en PIE y garantiza detener siempre la sesión (`StopPIE`). @spec REQ-82
    fn playtest(&mut self, args: &Value) -> Value {
        let duration_secs = args.get("duration_seconds").and_then(Value::as_u64).unwrap_or(5).clamp(1, 15);
        let actions: Vec<String> = args
            .get("actions")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .filter(|a: &Vec<String>| !a.is_empty())
            .unwrap_or_else(|| vec!["forward".into(), "jump".into()]);
        let fast_test = self.config.polling.interval < Duration::from_millis(100);
        let start_res = self.editor(|s| s.call_toolset_tool("EditorToolset.EditorAppToolset", "StartPIE", json!({ "bSimulateInEditor": false }), DEFAULT_TIMEOUT));
        if let Err(err) = start_res {
            return text_result(format!("Could not start PIE: {err}"), true);
        }
        std::thread::sleep(if fast_test { self.config.polling.interval } else { Duration::from_millis(400) });
        let hex = Self::hex_json(&json!({ "duration_seconds": duration_secs, "actions": actions }));
        let start_code = format!(
            "import json\nimport dbv_unreal_ai.editor as _e\n_p = json.loads(bytes.fromhex(\"{hex}\").decode(\"utf-8\"))\nresult = _e.start_pie_playtest_tracker(float(_p[\"duration_seconds\"]), _p[\"actions\"])\n"
        );
        let _ = self.run_fixed(&start_code);
        std::thread::sleep(if fast_test {
            self.config.polling.interval
        } else {
            Duration::from_millis(duration_secs * 1000 + 250)
        });
        let telemetry = self
            .run_fixed("import dbv_unreal_ai.editor as _e\nresult = _e.read_pie_playtest_telemetry()\n")
            .ok()
            .and_then(|ran| ran.get("result").cloned())
            .unwrap_or_else(|| json!({ "pawn_spawned": false }));
        let logs = self
            .resolve("LogsToolset")
            .ok()
            .and_then(|toolset| {
                self.editor(|s| {
                    s.call_toolset_tool(
                        &toolset,
                        "GetLogEntries",
                        json!({ "categories": null, "verbosity": "Warning", "searchString": null, "tailLines": 50 }),
                        DEFAULT_TIMEOUT,
                    )
                })
                .ok()
            })
            .and_then(|r| r.value)
            .unwrap_or_else(|| json!([]));
        let capture = self.editor(|s| crate::unreal::capture::capture(s, false, false)).ok();
        let stop_res = self.editor(|s| s.call_toolset_tool("EditorToolset.EditorAppToolset", "StopPIE", json!({}), DEFAULT_TIMEOUT));
        let stopped_pie = stop_res.is_ok();
        let report = json!({
            "telemetry": telemetry,
            "log_entries": logs,
            "stopped_pie": stopped_pie,
        });
        let text = compact::clip(&report.to_string());
        match capture {
            Some(cap) => json!({
                "content": [
                    { "type": "image", "data": cap.base64, "mimeType": cap.mime },
                    { "type": "text", "text": text }
                ],
                "isError": false
            }),
            None => text_result(text, false),
        }
    }

    fn session_spent(&self, service: &str) -> f64 {
        self.board.spent_since(self.board.session_start(self.started_at), service)
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
        Ok(format!("Tripo balance: {} credits available ({} reserved by running tasks). This session has used about {} Tripo credits.", balance.balance, balance.frozen, self.session_spent("tripo")))
    }

    /// Saldo de World Labs, con su clave: es otra cuenta y otros créditos que los de Tripo. @spec REQ-18
    fn worldlabs_balance(&self) -> Result<String, String> {
        let key = (self.config.key)("worldlabs").ok_or("there is no World Labs API key: add it in DBV Unreal AI → Settings → World Labs")?;
        let client = match &self.config.tripo_base {
            Some(base) => WorldLabsClient::with_base_url(&key, base),
            None => WorldLabsClient::new(&key),
        };
        let credits = client.credits().map_err(|e| e.to_string())?;
        Ok(format!("World Labs balance: {} credits available. This session has used about {} World Labs credits.", credits.balance, self.session_spent("worldlabs")))
    }

    /// Cualquier herramienta que gasta (Tripo o World Labs): traduce, exige confirmar el coste TOTAL, respeta el tope
    /// de SU servicio y lanza con SU clave (antes World Labs usaba la de Tripo: «No API key found», 2026-11-01).
    fn launch(&mut self, tool: &str, args: &Value) -> Result<String, String> {
        let launch = tools::build(tool, args, &self.config.library)?;
        let total = launch.total();
        let free = total <= 0.0;
        let service = crate::generation::service_of(launch.operation.category);
        let name = crate::generation::service_name(service);
        if !free && args.get("confirm_cost").and_then(Value::as_bool) != Some(true) {
            let each = if launch.variants > 1 { format!(" ({} x {})", launch.variants, total / f64::from(launch.variants)) } else { String::new() };
            return Err(format!("not run: {} costs about {total} {name} credits{each}. Tell the user and call again with confirm_cost=true only if they accept.", launch.operation.name));
        }
        let client_key = (self.config.key)(service).ok_or_else(|| format!("there is no {name} API key: add it in DBV Unreal AI → Settings → {name}"))?;
        let settings = GenerationSettings::load(&self.config.config_dir);
        let budget = settings.budget_for(service);
        let spent = self.session_spent(service);
        if !within_budget(spent, total, budget) {
            return Err(format!("not run: it would exceed the {name} session cap of {budget} credits ({spent} already used). The user can raise it in DBV Unreal AI → Settings → {name}. Tripo and World Labs have separate caps and credits."));
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
        // Un trabajo cuyo proceso murió (otra sesión del agente) se reanuda: su tarea sigue en Tripo o World Labs (REQ-27, REQ-70).
        let keys = service_keys(|service| (self.config.key)(service));
        if !keys.is_empty() {
            resume_orphans(&self.job_context(String::new()), &keys);
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
                "serverInfo": { "name": "dbv-unreal-ai", "title": "DBV Unreal AI", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_specs() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                // La captura es la única que devuelve una imagen (contenido `image` de MCP, REQ-13).
                let result = if name == "unreal_capture_viewport" {
                    let annotate = arguments.get("annotate").and_then(Value::as_bool).unwrap_or(false);
                    let high = arguments.get("detail").and_then(Value::as_str) == Some("high");
                    match self.editor(|s| crate::unreal::capture::capture(s, annotate, high)) {
                        Ok(capture) => json!({ "content": [{ "type": "image", "data": capture.base64, "mimeType": capture.mime }, { "type": "text", "text": crate::unreal::capture::describe(&capture) }], "isError": false }),
                        Err(error) => text_result(error.to_string(), true),
                    }
                } else if name == "unreal_playtest" {
                    self.playtest(&arguments)
                } else {
                    // Ningún resultado pasa del tope, sea cual sea la herramienta (REQ-51).
                    match self.call(&name, &arguments) {
                        Ok(text) => text_result(compact::clip(&text), false),
                        Err(text) => text_result(compact::clip(&text), true),
                    }
                };
                self.record_usage(&name, &result);
                Ok(result)
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
    if let Some(data) = crate::paths::data_dir() {
        crate::logging::init(&data, "mcp");
        // Biblioteca creada y migrada antes de abrirla, como hace la ventana al arrancar (REQ-53).
        if let Some(config) = crate::paths::config_dir() {
            let _ = crate::generation::commands::ensure_library(&config, &data);
        }
    }
    let mut server = McpServer::new(config_from_args(args));
    crate::logging::info(&format!("start v{} packaged={} library={} project={}", env!("CARGO_PKG_VERSION"), crate::logging::is_packaged(), server.config.library.root.display(), server.config.project_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default()));
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

    /// Las de solo lectura existen y van anotadas; las que modifican, no. @spec REQ-12
    #[test]
    fn las_consultas_van_anotadas_como_solo_lectura() {
        let specs = tool_specs();
        for name in READ_ONLY_TOOLS {
            let spec = specs.iter().find(|s| s["name"] == name).unwrap_or_else(|| panic!("{name} no existe"));
            assert_eq!(spec["annotations"]["readOnlyHint"], true, "{name}");
        }
        for name in ["unreal_call_tool", "unreal_run_python", "library_send_to_unreal", "generate_3d_model", "compile_project_code", "project_memory_propose"] {
            let spec = specs.iter().find(|s| s["name"] == name).unwrap();
            assert_eq!(spec["annotations"]["readOnlyHint"], false, "{name}");
        }
    }

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
        for expected in ["unreal_run_python", "generate_3d_model", "image_to_3d_model", "rig_model", "animate_model", "tripo_run", "tripo_operations", "tripo_balance", "worldlabs_balance", "generation_jobs", "library_send_to_unreal"] {
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
    /// Regresión (2026-11-01, la vio el usuario con Antigravity): World Labs se frenaba con el tope de Tripo y, al
    /// subirlo, recibía la clave de Tripo («No API key found»). Cada servicio, su clave, su tope y sus créditos.
    /// @spec REQ-18, RNF-04
    #[test]
    fn world_labs_usa_su_clave_y_su_tope_no_los_de_tripo() {
        use crate::unreal::mcp_client::tests::{fake_server, http};
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.key = Box::new(|service| Some(if service == "worldlabs" { "wl-key".into() } else { "tripo-key".into() }));
        let ask = rpc(&mut s, "tools/call", json!({ "name": "generate_world", "arguments": { "prompt": "ciudad", "name": "Ciudad" } }));
        assert!(ask["result"]["content"][0]["text"].as_str().unwrap().contains("1580 World Labs credits"), "{ask}");

        // El tope de Tripo por defecto es 300 y un mundo cuesta 1.580: no lo frena (World Labs tiene el suyo, 2.000).
        let (api, handle) = fake_server(vec![http("401 Unauthorized", "", r#"{"error":{"message":"No API key found in request"}}"#)]);
        s.config.tripo_base = Some(api.trim_end_matches("/mcp").to_string());
        rpc(&mut s, "tools/call", json!({ "name": "generate_world", "arguments": { "prompt": "ciudad", "name": "Ciudad", "confirm_cost": true } }));
        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("POST /marble/v1/worlds:generate"), "{}", seen[0]);
        assert!(seen[0].contains("wlt-api-key: wl-key") && !seen[0].contains("tripo-key"), "la clave de World Labs, con su cabecera, no la de Tripo");

        // Con su tope por debajo del coste, se frena y lo dice con su nombre.
        GenerationSettings { worldlabs_budget_credits: 1000.0, ..GenerationSettings::default() }.save(dir.path()).unwrap();
        let capped = rpc(&mut s, "tools/call", json!({ "name": "generate_world", "arguments": { "prompt": "ciudad", "name": "Ciudad", "confirm_cost": true } }));
        assert!(capped["result"]["content"][0]["text"].as_str().unwrap().contains("World Labs session cap of 1000"), "{capped}");
    }

    /// El agente consulta el saldo de World Labs con su clave, no el de Tripo (2026-11-01: Antigravity llamaba a
    /// tripo_balance antes de generar un mundo porque no había otra). @spec REQ-18, REQ-12
    #[test]
    fn worldlabs_balance_con_su_clave_y_sin_permiso() {
        use crate::unreal::mcp_client::tests::{fake_server, http};
        assert!(READ_ONLY_TOOLS.contains(&"worldlabs_balance"));
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let none = rpc(&mut s, "tools/call", json!({ "name": "worldlabs_balance", "arguments": {} }));
        assert!(none["result"]["content"][0]["text"].as_str().unwrap().contains("no World Labs API key"), "{none}");

        s.config.key = Box::new(|service| Some(if service == "worldlabs" { "wl-key".into() } else { "tripo-key".into() }));
        let (api, handle) = fake_server(vec![http("200 OK", "", r#"{"remaining_credits":4200}"#)]);
        s.config.tripo_base = Some(api.trim_end_matches("/mcp").to_string());
        let ok = rpc(&mut s, "tools/call", json!({ "name": "worldlabs_balance", "arguments": {} }));
        assert!(ok["result"]["content"][0]["text"].as_str().unwrap().contains("World Labs balance: 4200 credits available"), "{ok}");
        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("GET /marble/v1/credits") && seen[0].contains("wlt-api-key: wl-key"), "{}", seen[0]);
    }

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
        assert_eq!(s.session_spent("tripo"), 0.0);
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
        assert_eq!(s.session_spent("tripo"), 40.0, "2 × 20 (texto con textura, tabla oficial)");
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
        assert_eq!(s.session_spent("tripo"), 20.0 + 25.0 + 20.0);
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

    const RECORDED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/unreal-mcp/ue-5.8.4");

    /// Una respuesta grabada del editor real (JSON-RPC entero).
    fn recorded(file: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(format!("{RECORDED}/{file}")).unwrap()).unwrap()
    }

    /// Texto de una respuesta `tools/call` grabada.
    fn recorded_text(file: &str) -> String {
        recorded(file)["result"]["content"][0]["text"].as_str().unwrap().to_string()
    }

    /// PNG del tamaño de una captura real del editor (1944×617).
    fn capture_png() -> String {
        let image = image::RgbImage::from_fn(1944, 617, |x, y| image::Rgb([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image).write_to(&mut out, image::ImageFormat::Png).unwrap();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, out.into_inner())
    }

    /// Editor de Unreal simulado que responde POR CONTENIDO con las respuestas grabadas del editor real: lista y
    /// descripción de toolsets, resumen de escena, captura y `validate_python` / `run_python` del toolset DBV.
    /// Llamadas que recibió el editor simulado: herramienta y argumentos.
    type SeenCalls = std::sync::Arc<std::sync::Mutex<Vec<(String, Value)>>>;

    fn fake_editor() -> (String, SeenCalls) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/mcp", listener.local_addr().unwrap());
        let png = capture_png();
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = calls.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut socket) = stream else { break };
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
                let body: Value = request.split_once("\r\n\r\n").and_then(|(_, b)| serde_json::from_str(b).ok()).unwrap_or(Value::Null);
                let id = body.get("id").cloned().unwrap_or(Value::Null);
                let params = &body["params"];
                let reply = |text: String| json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{ "type": "text", "text": text }] } });
                let grabbed = |file: &str| {
                    let mut value = recorded(file);
                    value["id"] = id.clone();
                    value
                };
                let (status, headers, payload) = match body["method"].as_str().unwrap_or("") {
                    "initialize" => ("200 OK", "Mcp-Session-Id: s1\r\n", json!({ "jsonrpc": "2.0", "id": id, "result": { "protocolVersion": "2025-06-18", "capabilities": { "tools": {} } } }).to_string()),
                    "notifications/initialized" => ("202 Accepted", "", String::new()),
                    _ => {
                        let args = &params["arguments"];
                        seen.lock().unwrap().push((args["tool_name"].as_str().unwrap_or(params["name"].as_str().unwrap_or("")).to_string(), args["arguments"].clone()));
                        let response = match (params["name"].as_str().unwrap_or(""), args["tool_name"].as_str().unwrap_or("")) {
                            ("list_toolsets", _) => grabbed("list_toolsets.response.json"),
                            ("describe_toolset", _) => grabbed(&format!("describe_toolset.{}.response.json", args["toolset_name"].as_str().unwrap_or(""))),
                            (_, "get_scene_summary") => grabbed("call_tool.get_scene_summary.response.json"),
                            (_, "validate_python") => reply(json!({ "returnValue": json!({ "ok": true, "findings": [], "syntax_error": null }).to_string() }).to_string()),
                            (_, "run_python") => {
                                let code = args["arguments"]["code"].as_str().unwrap_or("");
                                let result_val = if code.contains("list_metahumans") {
                                    json!({ "count": 2, "metahumans": [{ "label": "BP_Victoria_med_nrw_55", "build": "f_med_nrw", "slots": { "Torso": "/Game/MetaHumans/Common/Shirt" } }] })
                                } else if code.contains("equip_metahuman_outfit") {
                                    json!({ "success": true, "actor": "BP_Victoria_med_nrw_55", "slot": "Torso", "rigged_asset": "/Game/DBV/MetaHumanOutfits/Chaqueta_Rigged_f_med_nrw", "has_bone_weights": true })
                                } else if code.contains("read_pie_playtest_telemetry") {
                                    json!({ "pawn_spawned": true, "pawn_class": "BP_ThirdPersonCharacter_C", "start_location": [0.0, 0.0, 91.7], "end_location": [618.0, 0.0, 101.6], "distance_moved_cm": 618.0, "fell_out_of_world": false, "avg_fps": 60.0, "frames": 300, "actions_executed": ["forward", "jump"] })
                                } else {
                                    json!({ "placed": 12, "folder": "DBV/Torches" })
                                };
                                reply(json!({ "returnValue": json!({ "success": true, "stdout": "", "stderr": "", "warnings": [], "error": null, "result": result_val }).to_string() }).to_string())
                            }
                            (_, "load_gaussian_splat") => reply(json!({ "returnValue": json!({ "success": true, "actor": args["arguments"]["actor_label"] }).to_string() }).to_string()),
                            (_, "import_model") => reply(json!({ "returnValue": json!({ "success": true, "primary": "/Game/DBV/Generated/Isla_Collider", "kind": "static", "animations": [] }).to_string() }).to_string()),
                            (_, "place_asset") => reply(json!({ "returnValue": json!({ "success": true, "actor": "Isla_Collider" }).to_string() }).to_string()),
                            (_, "get_project_info") => reply(json!({ "returnValue": json!({ "project_dir": "D:/P/Demo/" }).to_string() }).to_string()),
                            (_, "StartPIE") => reply(json!({ "success": true }).to_string()),
                            (_, "StopPIE") => reply(json!({ "success": true }).to_string()),
                            (_, "CaptureViewport") => reply(json!({ "image": { "mimeType": "image/png", "data": png }, "cameraLocation": { "x": 0.0, "y": -500.0, "z": 300.0 }, "cameraFOV": 90 }).to_string()),
                            _ => reply("{}".into()),
                        };
                        ("200 OK", "", response.to_string())
                    }
                };
                let _ = write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len());
            }
        });
        (url, calls)
    }

    /// Caracteres que vuelven al modelo en un resultado MCP: texto más el base64 de las imágenes.
    fn returned(result: &Value) -> usize {
        result["content"].as_array().unwrap().iter().map(|c| c["text"].as_str().or(c["data"].as_str()).map_or(0, |t| t.chars().count())).sum()
    }

    /// El guion fijo de REQ-51 contra el editor simulado con respuestas reales: la v0.3.0 devuelve como mucho la mitad
    /// que la v0.2.4 (calculada con su mismo formato: JSON entero de cada toolset, sin tope y captura a 1568 px),
    /// cada llamada sigue funcionando y todas quedan anotadas para el contador. @spec REQ-51
    #[test]
    fn el_guion_fijo_devuelve_al_menos_la_mitad_menos_que_la_v0_2_4() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        s.config.unreal_url = fake_editor().0;
        s.config.project_dir = Some(dir.path().join("Demo"));
        let describes = ["EditorToolset.EditorAppToolset", "ConvaiToolset.ConvaiSetupToolset", "NiagaraToolsets.NiagaraToolset_System"];
        let mut script: Vec<(&str, Value)> = vec![("unreal_list_toolsets", json!({}))];
        script.extend(describes.iter().map(|t| ("unreal_describe_toolset", json!({ "toolset": t.rsplit('.').next().unwrap() }))));
        script.push(("unreal_scene_summary", json!({})));
        script.push(("unreal_capture_viewport", json!({})));
        script.extend((0..5).map(|i| ("unreal_run_python", json!({ "code": format!("import unreal\nresult = {i}") }))));

        let mut now = 0;
        let mut now_text = 0;
        let mut list_text = 0;
        let mut python_text = 0;
        let mut capture_text = 0;
        for (name, args) in &script {
            let response = rpc(&mut s, "tools/call", json!({ "name": name, "arguments": args }));
            let result = &response["result"];
            assert_eq!(result["isError"], false, "{name}: {result}");
            let size = returned(result);
            now += size;
            now_text += result["content"].as_array().unwrap().iter().filter_map(|c| c["text"].as_str()).map(|t| t.chars().count()).sum::<usize>();
            match *name {
                "unreal_list_toolsets" => list_text = size,
                "unreal_run_python" => python_text += size,
                "unreal_capture_viewport" => capture_text = result["content"][1]["text"].as_str().unwrap().chars().count(),
                _ => {}
            }
        }

        // La v0.2.4: mismo listado y mismo Python; cada toolset en JSON entero; la escena por `unreal_call_tool` con el
        // valor entero; la captura a 1568 px y calidad 85.
        let described: usize = describes.iter().map(|t| serde_json::to_string(&crate::unreal::mcp_client::parse_toolset_info(&recorded_text(&format!("describe_toolset.{t}.response.json"))).unwrap()).unwrap().chars().count()).sum();
        let scene = crate::unreal::mcp_client::unwrap_value(serde_json::from_str(&recorded_text("call_tool.get_scene_summary.response.json")).unwrap()).to_string().chars().count();
        let png = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, capture_png()).unwrap();
        let (jpeg, _, _) = crate::unreal::capture::shrink(&png, true).unwrap();
        let old_capture = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, jpeg).len() + capture_text;
        let baseline = list_text + described + scene + old_capture + python_text;

        // La cifra de la prueba de consumo, visible con `cargo test el_guion_fijo -- --nocapture`.
        let baseline_text = list_text + described + scene + capture_text + python_text;
        println!("REQ-51 · guion fijo: v0.2.4 = {baseline} caracteres, v0.3.0 = {now} ({} % menos); solo texto: {baseline_text} → {now_text} ({} % menos)", 100 - now * 100 / baseline, 100 - now_text * 100 / baseline_text);
        assert!(now * 2 <= baseline, "v0.3.0 = {now} caracteres, v0.2.4 = {baseline}: no baja la mitad");
        // Sin contar la imagen (se paga por píxeles, no por caracteres): el texto del guion entero, ~10.000 tokens.
        assert!(now_text < 40_000, "texto del guion: {now_text} caracteres");
        let usage = crate::tool_usage::since(dir.path(), &dir.path().join("Demo").to_string_lossy(), 0);
        assert_eq!(usage.calls, script.len());
        assert_eq!(usage.images, 1);
        assert_eq!(usage.clipped, 0, "el formato compacto evita el recorte");
    }

    /// `initialize` le dice al cliente cómo gastar poco contexto. @spec REQ-51
    #[test]
    fn al_inicializar_explica_como_gastar_poco_contexto() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let init = rpc(&mut s, "initialize", json!({}));
        let instructions = init["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("unreal_scene_summary") && instructions.contains("ONE unreal_run_python") && instructions.contains("unreal-mcp"));
    }

    /// Un `.ply` suelto y un mundo con colisión se envían desde un agente como desde la ventana (antes: «the asset has
    /// no model file»). @spec REQ-54
    #[test]
    fn envia_gaussian_splats_desde_el_mcp() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let (url, calls) = fake_editor();
        s.config.unreal_url = url;
        let add = |s: &mut McpServer, name: &str, files: &[&str]| {
            let (id, folder) = s.config.library.new_entry_dir(name).unwrap();
            for f in files {
                std::fs::write(folder.join(f), b"data").unwrap();
            }
            s.config.library.add(LibraryEntry { id: id.clone(), name: name.into(), provider: "import".into(), kind: "world".into(), prompt: String::new(), parameters: Value::Null, task_id: String::new(), created_at: 1, credits: None, files: files.iter().map(|f| f.to_string()).collect(), thumbnail: None, sent_to: vec![], parent: None, output: Value::Null }).unwrap();
            id
        };
        let ply = add(&mut s, "Isla", &["scene.ply"]);
        let sent = rpc(&mut s, "tools/call", json!({ "name": "library_send_to_unreal", "arguments": { "id": ply } }));
        assert_eq!(sent["result"]["isError"], false, "{}", sent["result"]);
        assert!(sent["result"]["content"][0]["text"].as_str().unwrap().contains("Isla_Splat"));
        let tools: Vec<String> = calls.lock().unwrap().iter().map(|(t, _)| t.clone()).collect();
        assert!(tools.contains(&"load_gaussian_splat".to_string()) && !tools.contains(&"import_model".to_string()), "{tools:?}");
        let (_, splat_args) = calls.lock().unwrap().iter().find(|(t, _)| t == "load_gaussian_splat").cloned().unwrap();
        assert!(splat_args["source_file"].as_str().unwrap().ends_with("scene.ply"));

        calls.lock().unwrap().clear();
        let world = add(&mut s, "Mundo", &["world.spz", "collider.glb"]);
        let sent = rpc(&mut s, "tools/call", json!({ "name": "library_send_to_unreal", "arguments": { "id": world } }));
        assert!(sent["result"]["content"][0]["text"].as_str().unwrap().contains("collider mesh"), "{}", sent["result"]);
        let tools: Vec<String> = calls.lock().unwrap().iter().map(|(t, _)| t.clone()).collect();
        let position = |name: &str| tools.iter().position(|t| t == name).unwrap();
        assert!(position("load_gaussian_splat") < position("import_model") && position("import_model") < position("place_asset"), "{tools:?}");
        assert_eq!(s.config.library.get(&world).unwrap().sent_to.len(), 1);
    }

    /// Las zonas llegan al editor como código fijo con los datos en hexadecimal, sin pasar por `validate_python`, y una
    /// propuesta inválida no llega al editor. @spec REQ-57
    #[test]
    fn crea_zonas_con_codigo_fijo_de_la_app() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let (url, calls) = fake_editor();
        s.config.unreal_url = url;
        let names: Vec<String> = tool_specs().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        assert!(names.contains(&"level_zones_propose".to_string()) && names.contains(&"level_zones_list".to_string()));
        let bad = rpc(&mut s, "tools/call", json!({ "name": "level_zones_propose", "arguments": { "zones": [{ "name": "", "x": 0, "y": 0 }] } }));
        assert_eq!(bad["result"]["isError"], true);
        assert!(calls.lock().unwrap().iter().all(|(t, _)| t != "run_python"), "una propuesta inválida no llega al editor");
        let ok = rpc(&mut s, "tools/call", json!({ "name": "level_zones_propose", "arguments": { "zones": [{ "name": "Cocina", "description": "El horno", "x": 3, "y": 1 }] } }));
        assert_eq!(ok["result"]["isError"], false, "{}", ok["result"]);
        let ran: Vec<(String, Value)> = calls.lock().unwrap().iter().filter(|(t, _)| t == "run_python" || t == "validate_python").cloned().collect();
        assert_eq!(ran.len(), 1, "solo run_python, sin validate_python: {ran:?}");
        let code = ran[0].1["code"].as_str().unwrap();
        assert!(code.contains("bytes.fromhex(\"") && !code.contains("El horno"));
    }

    /// El agente ve los personajes registrados en Ajustes, la misma lista que el asistente integrado. @spec REQ-58
    #[test]
    fn el_agente_ve_los_personajes_de_convai() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let empty = rpc(&mut s, "tools/call", json!({ "name": "convai_characters", "arguments": {} }));
        assert!(empty["result"]["content"][0]["text"].as_str().unwrap().contains("No Convai characters"));
        crate::convai_characters::save(dir.path(), &[crate::convai_characters::ConvaiCharacter { name: "Victoria".into(), character_id: "c1fca5ca-03b4-11ef-ae52-42010a7be00e".into(), notes: String::new() }]).unwrap();
        let listed = rpc(&mut s, "tools/call", json!({ "name": "convai_characters", "arguments": {} }));
        assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains("- Victoria: c1fca5ca-03b4-11ef-ae52-42010a7be00e"));
    }

    /// Lista MetaHumans del nivel y equipa una prenda de la biblioteca o al enviarla con `library_send_to_unreal`. @spec REQ-81
    #[test]
    fn lista_y_equipa_prendas_en_metahumans() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let (url, calls) = fake_editor();
        s.config.unreal_url = url;
        let listed = rpc(&mut s, "tools/call", json!({ "name": "unreal_list_metahumans", "arguments": {} }));
        assert_eq!(listed["result"]["isError"], false, "{listed}");
        assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains("BP_Victoria_med_nrw_55"));

        let (id, folder) = s.config.library.new_entry_dir("Chaqueta").unwrap();
        std::fs::write(folder.join("model.glb"), b"glb").unwrap();
        s.config.library.add(LibraryEntry {
            id: id.clone(),
            name: "Chaqueta".into(),
            provider: "tripo".into(),
            kind: "model".into(),
            prompt: "jacket".into(),
            parameters: json!({ "garment_slot": "Torso", "quad_remeshed": true }),
            task_id: "t-1".into(),
            created_at: 1,
            credits: Some(30.0),
            files: vec!["model.glb".into()],
            thumbnail: None,
            sent_to: vec![],
            parent: None,
            output: Value::Null,
        }).unwrap();

        // REQ-83: sin destino la prenda solo se importa; nunca se equipa en «el primero» del nivel.
        calls.lock().unwrap().clear();
        let sent = rpc(&mut s, "tools/call", json!({ "name": "library_send_to_unreal", "arguments": { "id": id } }));
        assert_eq!(sent["result"]["isError"], false, "{sent}");
        let sent_text = sent["result"]["content"][0]["text"].as_str().unwrap();
        assert!(sent_text.contains("NOT equipped") && !sent_text.contains("auto-rigged"), "{sent_text}");
        let seen: Vec<(String, Value)> = calls.lock().unwrap().clone();
        assert!(!seen.iter().any(|(_, args)| args["code"].as_str().is_some_and(|c| c.contains("equip_metahuman_outfit"))), "sin destino no se ejecuta el equipado: {seen:?}");
        assert!(!seen.iter().any(|(tool, _)| tool == "place_asset"), "una prenda no se coloca delante de la cámara");

        calls.lock().unwrap().clear();
        let sent = rpc(&mut s, "tools/call", json!({ "name": "library_send_to_unreal", "arguments": { "id": id, "metahuman": "BP_Victoria_med_nrw_55" } }));
        assert_eq!(sent["result"]["isError"], false, "{sent}");
        let sent_text = sent["result"]["content"][0]["text"].as_str().unwrap();
        assert!(sent_text.contains("auto-rigged + equipped on MetaHuman BP_Victoria_med_nrw_55 (Torso)"), "{sent_text}");
        let seen: Vec<(String, Value)> = calls.lock().unwrap().clone();
        let equip_code = seen.iter().find_map(|(_, args)| args["code"].as_str().filter(|c| c.contains("equip_metahuman_outfit"))).expect("ejecuta el equipado");
        let hex: String = "BP_Victoria_med_nrw_55".bytes().map(|b| format!("{b:02x}")).collect();
        assert!(equip_code.contains(&hex), "el destino viaja en el código fijo");

        let equipped = rpc(&mut s, "tools/call", json!({ "name": "unreal_equip_metahuman_outfit", "arguments": { "source": id, "actor_label": "BP_Victoria_med_nrw_55" } }));
        assert_eq!(equipped["result"]["isError"], false, "{equipped}");
        assert!(equipped["result"]["content"][0]["text"].as_str().unwrap().contains("Chaqueta_Rigged_f_med_nrw"));
    }

    /// `unreal_playtest` arranca PIE, registra movimiento por ticks, captura el viewport y detiene siempre PIE. @spec REQ-82
    #[test]
    fn playtest_autonomo_arranca_mide_captura_y_detiene_pie() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = server(dir.path());
        let (url, calls) = fake_editor();
        s.config.unreal_url = url;
        let res = rpc(&mut s, "tools/call", json!({ "name": "unreal_playtest", "arguments": { "duration_seconds": 3, "actions": ["forward", "jump"] } }));
        assert_eq!(res["result"]["isError"], false, "{res}");
        let content = res["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2, "devuelve captura más telemetría JSON");
        assert_eq!(content[0]["type"], "image");
        let text = content[1]["text"].as_str().unwrap();
        assert!(text.contains("\"distance_moved_cm\":618.0") && text.contains("\"stopped_pie\":true"), "{text}");
        let seen: Vec<String> = calls.lock().unwrap().iter().map(|(t, _)| t.clone()).collect();
        assert!(seen.contains(&"StartPIE".to_string()) && seen.contains(&"StopPIE".to_string()), "{seen:?}");
    }
}
