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

pub mod commands;
pub mod tripo;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;

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

/// Sondea una tarea hasta que termina bien; cancelación, fallo o tiempo agotado son errores.
fn wait_for_task(client: &TripoClient, task_id: &str, cancel: &AtomicBool, polling: Polling, report: &mut impl FnMut(&str, Option<f64>)) -> Result<tripo::TripoTask, UnrealError> {
    let started = Instant::now();
    let outcome = loop {
        if cancel.load(Ordering::Relaxed) {
            break Err(UnrealError::Protocol("cancelado por el usuario (Tripo puede cobrar la tarea igualmente)".into()));
        }
        let task = match client.task(task_id) {
            Ok(task) => task,
            Err(error) => break Err(error),
        };
        report("running", task.progress);
        if task.is_failed() {
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

/// Trabajo completo de texto → modelo con Tripo. Devuelve la entrada de la biblioteca.
#[allow(clippy::too_many_arguments)]
pub fn run_tripo_job(client: &TripoClient, request: &TextToModel, name: &str, library: &Library, cancel: &AtomicBool, polling: Polling, mut report: impl FnMut(&str, Option<f64>)) -> Result<LibraryEntry, UnrealError> {
    report("queued", None);
    let task_id = client.text_to_model(request)?;
    // A partir de aquí la tarea existe en Tripo y puede cobrarse: quien lleva el gasto ya no lo devuelve.
    report("submitted", None);
    let task = wait_for_task(client, &task_id, cancel, polling, &mut report)?;
    let model_url = task.model_url.clone().ok_or_else(|| UnrealError::Protocol(format!("la tarea {task_id} terminó sin modelo")))?;
    report("downloading", Some(100.0));
    let (id, dir) = library.new_entry_dir(name)?;
    tripo::download(&model_url, &dir.join("model.glb"))?;
    // La miniatura es opcional: si falla, el modelo ya está a salvo.
    let thumbnail = task.image_url.as_deref().and_then(|url| {
        let file = if url.contains(".png") { "thumbnail.png" } else if url.contains(".jpg") || url.contains(".jpeg") { "thumbnail.jpg" } else { "thumbnail.webp" };
        tripo::download(url, &dir.join(file)).ok().map(|_| file.to_string())
    });
    let entry = LibraryEntry {
        id,
        name: name.to_string(),
        provider: "tripo".into(),
        kind: "model".into(),
        prompt: request.prompt.trim().to_string(),
        parameters: json!(request.body()),
        task_id,
        created_at: now_secs(),
        credits: task.credits.or(Some(tripo::TEXT_TO_MODEL_CREDITS)),
        files: vec!["model.glb".into()],
        thumbnail,
        sent_to: vec![],
    };
    library.add(entry.clone())?;
    Ok(entry)
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
        let entry = run_tripo_job(&client, &request(), "Cofre del tesoro", &library, &AtomicBool::new(false), quick(), |stage, _| stages.push(stage.to_string())).unwrap();
        assert_eq!(stages, vec!["queued", "submitted", "running", "downloading"]);
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
        let failed = run_tripo_job(&client, &request(), "x", &library, &AtomicBool::new(false), quick(), |_, _| {});
        assert!(failed.unwrap_err().to_string().contains("failed"));
        let (api2, _handle2) = fake_server(vec![http("200 OK", "", r#"{"code":0,"data":{"task_id":"t-2"}}"#)]);
        let cancelled = run_tripo_job(&TripoClient::with_base_url("k", api2.trim_end_matches("/mcp")), &request(), "x", &library, &AtomicBool::new(true), quick(), |_, _| {});
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
