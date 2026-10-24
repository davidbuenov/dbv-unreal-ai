// =============================================================================
// DBV Unreal AI — Generación 3D con servicios de terceros
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, REQ-17, RNF-04, RNF-IA.4
//
// Un trabajo de generación: lanzar la tarea, seguir su progreso, descargar el resultado al
// momento (Tripo caduca sus enlaces a los 5 minutos) y guardarlo en la biblioteca. Nada se
// lanza sin `confirm_cost` ni por encima del tope de gasto de la sesión (RNF-04). Las claves
// viven en el almacén del sistema con la cuenta `service:<proveedor>` (RNF-IA.4).

pub mod catalog;
pub mod commands;
pub mod jobs;
pub mod tools;
pub mod tripo;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use crate::library::{now_secs, Library, LibraryEntry};
use crate::unreal::UnrealError;
use tripo::{TextToModel, TripoClient};

/// Cuenta del almacén de credenciales para la clave de un servicio.
pub fn key_account(service: &str) -> String {
    format!("service:{service}")
}

/// Servicios de generación que conoce la app.
pub const SERVICES: [&str; 2] = ["tripo", "worldlabs"];

/// Progreso de un trabajo, tal como lo recibe la interfaz (evento `generation-progress`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub job_id: String,
    /// `queued`, `submitted` (la tarea ya existe en Tripo), `running`, `downloading`, `done`, `failed`, `cancelled`.
    pub stage: String,
    pub progress: Option<f64>,
    pub entry: Option<LibraryEntry>,
    pub error: Option<String>,
}

/// Cómo esperar a una tarea (cambia en los tests para no esperar de verdad).
#[derive(Debug, Clone, Copy)]
pub struct Polling {
    pub interval: Duration,
    pub max_wait: Duration,
}

impl Default for Polling {
    fn default() -> Self {
        Self { interval: Duration::from_secs(2), max_wait: Duration::from_secs(10 * 60) }
    }
}

/// Lo que va ocurriendo en un trabajo, para la consola y el progreso (REQ-27).
#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent<'a> {
    /// La petición exacta que sale hacia el proveedor (sin la clave).
    Request { endpoint: &'a str, body: &'a Value },
    /// El proveedor creó la tarea.
    Task(&'a str),
    /// Cambio de etapa y progreso.
    Stage(&'a str, Option<f64>),
    /// Una línea para la consola (una subida, un aviso).
    Note(String),
    /// El proveedor dio la tarea por fallida: Tripo no cobra las tareas fallidas ni canceladas (`/es/docs/billing`).
    NotCharged,
}

/// Sondea una tarea hasta que termina bien; cancelación, fallo o tiempo agotado son errores.
fn wait_for_task(client: &TripoClient, task_id: &str, cancel: &AtomicBool, polling: Polling, report: &mut impl FnMut(JobEvent)) -> Result<tripo::TripoTask, UnrealError> {
    let started = Instant::now();
    let outcome = loop {
        if cancel.load(Ordering::Relaxed) {
            break Err(UnrealError::Protocol("cancelado por el usuario (Tripo puede cobrar la tarea igualmente)".into()));
        }
        let task = match client.task(task_id) {
            Ok(task) => task,
            Err(error) => break Err(error),
        };
        report(JobEvent::Stage("running", task.progress));
        if task.is_failed() {
            report(JobEvent::NotCharged);
            break Err(UnrealError::Protocol(format!("la tarea {task_id} de Tripo terminó como «{}»", task.status)));
        }
        if task.is_success() {
            break Ok(task);
        }
        if started.elapsed() > polling.max_wait {
            break Err(UnrealError::Timeout(format!("la tarea {task_id} de Tripo no terminó en {} s", polling.max_wait.as_secs())));
        }
        std::thread::sleep(polling.interval);
    };
    outcome
}

/// Una tarea de Tripo sigue siendo `input` válido de otra durante este tiempo; pasado, se sube el fichero.
const TASK_REUSE_SECS: u64 = 24 * 3600;

/// Traduce una fuente (`input`, `inputs`, `ref_image`) a lo que entiende Tripo: los `task_…`, `file_…` y enlaces
/// pasan tal cual; un id de la biblioteca se convierte en su tarea (un rig, siempre: retarget la exige) o se sube
/// su fichero; una ruta local se sube. Recorre listas y objetos (`inputs: [{"front": …}]`).
pub fn resolve_sources(value: &Value, client: &TripoClient, library: &Library, report: &mut impl FnMut(JobEvent)) -> Result<Value, UnrealError> {
    let resolved = match value {
        Value::Array(items) => Value::Array(items.iter().map(|item| resolve_sources(item, client, library, report)).collect::<Result<_, _>>()?),
        Value::Object(map) => Value::Object(map.iter().map(|(k, v)| resolve_sources(v, client, library, report).map(|r| (k.clone(), r))).collect::<Result<_, _>>()?),
        Value::String(text) => Value::String(resolve_source(text, client, library, report)?),
        other => other.clone(),
    };
    Ok(resolved)
}

fn resolve_source(text: &str, client: &TripoClient, library: &Library, report: &mut impl FnMut(JobEvent)) -> Result<String, UnrealError> {
    let text = text.trim();
    if text.starts_with("task_") || text.starts_with("file_") || text.starts_with("https://") || text.starts_with("http://") || text.is_empty() {
        return Ok(text.to_string());
    }
    let upload = |path: &Path, report: &mut dyn FnMut(JobEvent)| -> Result<String, UnrealError> {
        report(JobEvent::Note(format!("subiendo {} a Tripo…", path.display())));
        let token = client.upload(path)?;
        report(JobEvent::Note(format!("subido: {token}")));
        Ok(token)
    };
    let resolved = match library.get(text) {
        Some(entry) if !entry.task_id.is_empty() && entry.provider == "tripo" && (entry.kind == "rig" || now_secs().saturating_sub(entry.created_at) < TASK_REUSE_SECS) => entry.task_id,
        Some(entry) => {
            let file = entry.primary_file().ok_or_else(|| UnrealError::NotFound(format!("el asset {text} no tiene ningún fichero que subir")))?;
            upload(&library.file_path(&entry.id, &file)?, report)?
        }
        None if Path::new(text).is_file() => upload(Path::new(text), report)?,
        None => return Err(UnrealError::NotFound(format!("«{text}» no es un id de la biblioteca, ni un fichero, ni un enlace, ni un task_/file_ de Tripo"))),
    };
    Ok(resolved)
}

/// Nombre del fichero local de una salida (`model_url` → `model.glb`, `rendered_image_url` → `thumbnail.webp`).
fn output_file(key: &str, url: &str, produces: &str) -> String {
    let stem = match key.trim_end_matches("_url") {
        "rendered_image" | "image" => "thumbnail".to_string(),
        "pbr_model" | "base_model" => "model".to_string(),
        other => other.to_string(),
    };
    let ext = url.split('?').next().and_then(|path| path.rsplit('/').next()).and_then(|name| name.rsplit_once('.').map(|(_, e)| e.to_lowercase())).filter(|e| !e.is_empty() && e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()));
    let fallback = match (stem.as_str(), produces) {
        ("thumbnail", _) | (_, "image") | (_, "multiview") => "png",
        (_, "splat") => "splat",
        _ => "glb",
    };
    format!("{stem}.{}", ext.unwrap_or_else(|| fallback.to_string()))
}

/// Lo que deja una operación: la entrada nueva (si produce ficheros) y la salida de Tripo.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub entry: Option<LibraryEntry>,
    pub output: Value,
    pub task_id: String,
}

/// Qué pidió quien lanza una operación.
#[derive(Debug, Clone)]
pub struct OperationRequest {
    pub operation: &'static catalog::Operation,
    /// Parámetros tal como los dio la persona o la IA (las fuentes aún sin resolver).
    pub params: Value,
    pub name: String,
    /// Asset de la biblioteca del que sale este (rig → modelo; animación → rig).
    pub parent: Option<String>,
}

/// Trabajo completo de cualquier operación del catálogo: resolver fuentes, crear la tarea, esperarla y bajar
/// TODAS sus salidas (modelo, vista previa, vistas, …) a una entrada nueva de la biblioteca enlazada con su origen.
#[allow(clippy::too_many_arguments)]
pub fn run_operation(client: &TripoClient, request: &OperationRequest, library: &Library, cancel: &AtomicBool, polling: Polling, report: &mut impl FnMut(JobEvent)) -> Result<Outcome, UnrealError> {
    let op = request.operation;
    report(JobEvent::Stage("queued", None));
    catalog::validate(op, &request.params).map_err(UnrealError::Format)?;
    let mut body = request.params.clone();
    if let Value::Object(map) = &mut body {
        for field in catalog::SOURCE_FIELDS {
            if let Some(value) = map.get(field).cloned() {
                map.insert(field.to_string(), resolve_sources(&value, client, library, report)?);
            }
        }
    }
    report(JobEvent::Request { endpoint: op.endpoint, body: &body });
    let task_id = client.create(op.endpoint, &body)?;
    // A partir de aquí la tarea existe en Tripo y puede cobrarse.
    report(JobEvent::Task(&task_id));
    report(JobEvent::Stage("submitted", None));
    let task = wait_for_task(client, &task_id, cancel, polling, report)?;
    let links: Vec<(String, String)> = task
        .output
        .as_object()
        .into_iter()
        .flat_map(|map| map.iter())
        .filter_map(|(key, value)| value.as_str().filter(|v| v.starts_with("https://") || (cfg!(test) && v.starts_with("http://127.0.0.1"))).map(|url| (key.clone(), url.to_string())))
        .collect();
    let data: serde_json::Map<String, Value> = task.output.as_object().into_iter().flat_map(|map| map.iter()).filter(|(key, value)| !links.iter().any(|(k, _)| k == *key) && !value.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect();
    let entry = if op.produces == "none" || links.is_empty() {
        None
    } else {
        report(JobEvent::Stage("downloading", Some(100.0)));
        let (id, dir) = library.new_entry_dir(&request.name)?;
        let mut files = Vec::new();
        let mut thumbnail = None;
        for (key, url) in &links {
            let file = output_file(key, url, op.produces);
            if files.contains(&file) {
                continue;
            }
            match tripo::download(url, &dir.join(&file)) {
                Ok(_) if file.starts_with("thumbnail.") => thumbnail = Some(file),
                Ok(_) => files.push(file),
                // Una vista previa que no baja no estropea el resultado; el resultado sí.
                Err(error) if file.starts_with("thumbnail.") => report(JobEvent::Note(format!("sin vista previa: {error}"))),
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(error);
                }
            }
        }
        let entry = LibraryEntry {
            id,
            name: request.name.clone(),
            provider: "tripo".into(),
            kind: op.produces.into(),
            prompt: request.params.get("prompt").and_then(Value::as_str).unwrap_or("").trim().to_string(),
            parameters: json!({ "operation": op.name, "request": body }),
            task_id: task_id.clone(),
            created_at: now_secs(),
            credits: task.credits.or(Some(catalog::estimate(op, &request.params))),
            files,
            thumbnail,
            sent_to: vec![],
            parent: request.parent.clone(),
            output: Value::Object(data.clone()),
        };
        library.add(entry.clone())?;
        Some(entry)
    };
    Ok(Outcome { entry, output: Value::Object(data), task_id })
}

/// Texto → modelo (la operación de siempre) con los valores verificados de `TextToModel`.
pub fn run_tripo_job(client: &TripoClient, request: &TextToModel, name: &str, library: &Library, cancel: &AtomicBool, polling: Polling, report: &mut impl FnMut(JobEvent)) -> Result<LibraryEntry, UnrealError> {
    if request.prompt.trim().is_empty() {
        return Err(UnrealError::Format("el prompt está vacío".into()));
    }
    let operation = catalog::find("text_to_model").ok_or_else(|| UnrealError::NotFound("text_to_model".into()))?;
    let outcome = run_operation(client, &OperationRequest { operation, params: request.body(), name: name.to_string(), parent: None }, library, cancel, polling, report)?;
    outcome.entry.ok_or_else(|| UnrealError::Protocol(format!("la tarea {} terminó sin modelo", outcome.task_id)))
}

/// ¿Cabe un gasto más en el tope? `budget <= 0` significa «sin tope».
pub fn within_budget(spent: f64, cost: f64, budget: f64) -> bool {
    budget <= 0.0 || spent + cost <= budget
}

/// Ruta del fichero de ajustes de generación.
pub fn settings_path(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join("generation.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::mcp_client::tests::{fake_server, http};

    fn request() -> TextToModel {
        TextToModel { prompt: "cofre del tesoro".into(), model: None, texture_quality: None, face_limit: None, smart_low_poly: None, pbr: None }
    }

    fn quick() -> Polling {
        Polling { interval: Duration::from_millis(1), max_wait: Duration::from_secs(5) }
    }

    /// @spec REQ-15, REQ-17
    #[test]
    fn un_trabajo_completo_guarda_el_glb_y_la_entrada() {
        // Dos servidores: uno hace de Tripo y otro de CDN, para que la URL firmada sea conocida al montar las respuestas.
        let (cdn, cdn_handle) = fake_server(vec![http("200 OK", "", "glTF-binario")]);
        let model_url = cdn.replace("/mcp", "/m.glb");
        let (api, api_handle) = fake_server(vec![
            http("200 OK", "", r#"{"code":0,"data":{"task_id":"t-9"}}"#),
            http("200 OK", "", &format!(r#"{{"code":0,"data":{{"status":"success","progress":100,"consumed_credit":30,"output":{{"model_url":"{model_url}"}}}}}}"#)),
        ]);
        let client = TripoClient::with_base_url("k", api.trim_end_matches("/mcp"));
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("library"));
        let mut stages = Vec::new();
        let mut sent = Vec::new();
        let entry = run_tripo_job(&client, &request(), "Cofre del tesoro", &library, &AtomicBool::new(false), quick(), &mut |event| match event {
            JobEvent::Stage(stage, _) => stages.push(stage.to_string()),
            JobEvent::Request { endpoint, body } => sent.push((endpoint.to_string(), body.clone())),
            _ => {}
        })
        .unwrap();
        assert_eq!(stages, vec!["queued", "submitted", "running", "downloading"]);
        // La consola enseña la petición exacta (REQ-27): endpoint y cuerpo, sin clave.
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "/generation/text-to-model");
        assert_eq!(sent[0].1["prompt"], "cofre del tesoro");
        assert!(!sent[0].1.to_string().contains("Bearer"));
        assert_eq!(entry.task_id, "t-9");
        assert_eq!(entry.credits, Some(30.0));
        assert_eq!(std::fs::read_to_string(library.file_path(&entry.id, "model.glb").unwrap()).unwrap(), "glTF-binario");
        assert_eq!(library.list(), vec![entry]);
        api_handle.join().unwrap();
        cdn_handle.join().unwrap();
    }

    /// @spec REQ-15
    #[test]
    fn una_tarea_fallida_o_cancelada_no_deja_nada() {
        let (api, _handle) = fake_server(vec![http("200 OK", "", r#"{"code":0,"data":{"task_id":"t-1"}}"#), http("200 OK", "", r#"{"code":0,"data":{"status":"failed"}}"#)]);
        let client = TripoClient::with_base_url("k", api.trim_end_matches("/mcp"));
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("library"));
        let failed = run_tripo_job(&client, &request(), "x", &library, &AtomicBool::new(false), quick(), &mut |_| {});
        assert!(failed.unwrap_err().to_string().contains("failed"));
        let (api2, _handle2) = fake_server(vec![http("200 OK", "", r#"{"code":0,"data":{"task_id":"t-2"}}"#)]);
        let cancelled = run_tripo_job(&TripoClient::with_base_url("k", api2.trim_end_matches("/mcp")), &request(), "x", &library, &AtomicBool::new(true), quick(), &mut |_| {});
        assert!(cancelled.unwrap_err().to_string().contains("cancelado"));
        assert!(library.list().is_empty());
    }

    /// @spec RNF-04
    #[test]
    fn el_tope_de_gasto_se_respeta() {
        assert!(within_budget(0.0, 30.0, 300.0));
        assert!(within_budget(270.0, 30.0, 300.0));
        assert!(!within_budget(271.0, 30.0, 300.0));
        assert!(within_budget(10_000.0, 30.0, 0.0), "0 = sin tope");
    }
}
