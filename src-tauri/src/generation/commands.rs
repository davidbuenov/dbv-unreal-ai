// =============================================================================
// DBV Unreal AI — Comandos Tauri de la generación 3D
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Reglas de la frontera:
//   · Ninguna clave vuelve al frontend: solo `hasKey` (RNF-IA.4).
//   · Nada se lanza sin `confirmCost: true` ni por encima del tope de la sesión (RNF-04).
//   · El trabajo corre en un hilo propio y avisa con eventos `generation-progress`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::jobs::{entry_of, start_jobs, Cancels, JobBoard, JobContext, JobRecord};
use super::tripo::{Balance, TripoClient};
use super::{key_account, settings_path, tools, within_budget, Polling, Progress, SERVICES};
use crate::ai::secrets::KeyStorage;
use crate::ai::AiState;
use crate::library::{library_root, Library};
use crate::unreal::{io_error, UnrealError};

/// Trabajos en curso (para cancelarlos) y el inicio de la sesión. El gasto NO vive aquí: se calcula de los
/// ficheros de trabajo, que también escribe el subproceso `--mcp` de un agente (REQ-27, RNF-04).
#[derive(Default)]
pub struct GenerationState {
    pub cancels: Cancels,
    pub session_start: AtomicU64,
}

/// Ajustes guardados de la generación.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSettings {
    /// Tope de créditos por sesión (0 = sin tope).
    pub budget_credits: f64,
    /// Carpeta de la biblioteca elegida por el usuario; sin ella, la de datos de la app (REQ-30).
    #[serde(default)]
    pub library_dir: Option<String>,
}

impl Default for GenerationSettings {
    fn default() -> Self {
        Self { budget_credits: 300.0, library_dir: None }
    }
}

impl GenerationSettings {
    /// Lee los ajustes de `config_dir` (los mismos para la ventana y para `--mcp`).
    pub fn load(config_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(settings_path(config_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    /// Carpeta de la biblioteca: la elegida o la de siempre.
    pub fn library_root(&self, data_dir: &std::path::Path) -> PathBuf {
        self.library_dir.as_deref().filter(|d| !d.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| library_root(data_dir))
    }

    fn save(&self, config_dir: &std::path::Path) -> Result<(), UnrealError> {
        std::fs::create_dir_all(config_dir).map_err(|error| io_error(config_dir, &error))?;
        let text = serde_json::to_string_pretty(self).map_err(|error| UnrealError::Format(error.to_string()))?;
        let path = settings_path(config_dir);
        std::fs::write(&path, text).map_err(|error| io_error(&path, &error))
    }
}

/// Lo que la interfaz necesita para la tarjeta de coste.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationStatus {
    pub spent: f64,
    pub budget: f64,
    pub has_key: HashMap<String, bool>,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, UnrealError> {
    app.path().app_config_dir().map_err(|error| UnrealError::Io(error.to_string()))
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, UnrealError> {
    app.path().app_data_dir().map_err(|error| UnrealError::Io(error.to_string()))
}

fn library(app: &AppHandle) -> Result<Library, UnrealError> {
    Ok(Library::new(load_settings(app).library_root(&data_dir(app)?)))
}

/// Los ficheros de trabajo y el inicio de la sesión (hay que arrancarla con `begin_session`).
fn board(app: &AppHandle) -> Result<JobBoard, UnrealError> {
    Ok(JobBoard::new(&data_dir(app)?))
}

/// Marca el inicio de la sesión y limpia los trabajos antiguos. Lo llama el arranque de la ventana.
pub fn begin_session(app: &AppHandle) {
    if let Ok(board) = board(app) {
        let started = board.start_session();
        board.prune();
        app.state::<GenerationState>().session_start.store(started, Ordering::Relaxed);
    }
}

/// Créditos de la sesión, sumando también lo que lanzaron los agentes.
fn session_spent(app: &AppHandle, state: &GenerationState) -> f64 {
    board(app).map(|board| board.spent_since(state.session_start.load(Ordering::Relaxed))).unwrap_or(0.0)
}

fn load_settings(app: &AppHandle) -> GenerationSettings {
    config_dir(app).map(|dir| GenerationSettings::load(&dir)).unwrap_or_default()
}

fn known_service(service: &str) -> Result<(), UnrealError> {
    if !SERVICES.contains(&service) {
        return Err(UnrealError::NotFound(format!("servicio desconocido: {service}")));
    }
    Ok(())
}

#[tauri::command]
pub fn generation_status(app: AppHandle, ai: State<'_, AiState>, state: State<'_, GenerationState>) -> GenerationStatus {
    let has_key = SERVICES.iter().map(|s| (s.to_string(), ai.secrets.get(&key_account(s)).is_some())).collect();
    let spent = session_spent(&app, &state);
    GenerationStatus { spent, budget: load_settings(&app).budget_credits, has_key }
}

/// Guarda la clave de un servicio en el almacén del sistema (solo de ida). @spec RNF-IA.4
#[tauri::command]
pub fn generation_set_key(ai: State<'_, AiState>, service: String, key: String) -> Result<KeyStorage, UnrealError> {
    known_service(&service)?;
    if key.trim().is_empty() {
        return Err(UnrealError::Format("la clave está vacía".into()));
    }
    Ok(ai.secrets.set(&key_account(&service), key.trim()))
}

#[tauri::command]
pub fn generation_delete_key(ai: State<'_, AiState>, service: String) -> Result<(), UnrealError> {
    known_service(&service)?;
    ai.secrets.delete(&key_account(&service)).map_err(|error| UnrealError::Io(error.to_string()))
}

/// Cambia el tope de créditos por sesión. @spec RNF-04
#[tauri::command]
pub fn generation_set_budget(app: AppHandle, credits: f64) -> Result<(), UnrealError> {
    let dir = config_dir(&app)?;
    GenerationSettings { budget_credits: credits.max(0.0), ..GenerationSettings::load(&dir) }.save(&dir)
}

/// Lo que hará una herramienta de generación, sin gastar nada: para la tarjeta de coste. @spec RNF-04, REQ-27
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prepared {
    pub operation: String,
    pub endpoint: String,
    pub name: String,
    pub variants: u32,
    /// Parámetros que se enviarán (las fuentes aún sin resolver: un id de la biblioteca, una ruta…).
    pub params: serde_json::Value,
    /// Créditos estimados en total (todas las variantes).
    pub total: f64,
    pub pricing: String,
}

/// Herramientas de generación para el asistente integrado: las mismas que ve un agente por MCP (D10). @spec REQ-33
#[tauri::command]
pub fn generation_tool_specs() -> Vec<serde_json::Value> {
    tools::specs(false)
}

/// Traduce una llamada a una herramienta (o una petición de la vista Generar) y calcula su coste. @spec RNF-04
#[tauri::command]
pub fn generation_prepare(app: AppHandle, tool: String, args: serde_json::Value) -> Result<Prepared, UnrealError> {
    let launch = tools::build(&tool, &args, &library(&app)?).map_err(UnrealError::Format)?;
    Ok(Prepared {
        operation: launch.operation.name.into(),
        endpoint: launch.operation.endpoint.into(),
        name: launch.name.clone(),
        variants: launch.variants,
        params: launch.params.clone(),
        total: launch.total(),
        pricing: launch.operation.pricing.into(),
    })
}

/// Lanza una herramienta de generación ya confirmada y devuelve los ids de sus trabajos. @spec REQ-15, REQ-16, REQ-29, REQ-33, RNF-04
#[tauri::command]
pub fn generation_launch(app: AppHandle, ai: State<'_, AiState>, state: State<'_, GenerationState>, tool: String, args: serde_json::Value, confirm_cost: bool) -> Result<Vec<String>, UnrealError> {
    let launch = tools::build(&tool, &args, &library(&app)?).map_err(UnrealError::Format)?;
    let total = launch.total();
    if total > 0.0 && !confirm_cost {
        return Err(UnrealError::Protocol("falta confirmar el coste antes de generar".into()));
    }
    let key = ai.secrets.get(&key_account("tripo")).ok_or_else(|| UnrealError::NotFound("no hay clave de Tripo: añádela en Ajustes".into()))?;
    let budget = load_settings(&app).budget_credits;
    let spent = session_spent(&app, &state);
    if !within_budget(spent, total, budget) {
        return Err(UnrealError::Protocol(format!("superaría el tope de {budget} créditos de esta sesión (llevas {spent} y esto son {total})")));
    }
    let emitter = app.clone();
    let context = JobContext {
        board: board(&app)?,
        library: library(&app)?,
        key,
        base_url: None,
        polling: Polling::default(),
        cancels: state.cancels.clone(),
        hook: Arc::new(move |record: &JobRecord| {
            let _ = emitter.emit("generation-progress", progress_of(&emitter, record));
        }),
        source: "app".into(),
    };
    Ok(start_jobs(&context, launch.operation, &launch.params, &launch.name, launch.variants, launch.parent.clone()))
}

/// Catálogo de operaciones de Tripo (para `tripo_operations` del asistente integrado). @spec REQ-33
#[tauri::command]
pub fn generation_operations(operation: Option<String>) -> Result<String, UnrealError> {
    tools::operations_text(&serde_json::json!({ "operation": operation.unwrap_or_default() })).map_err(UnrealError::NotFound)
}

/// Saldo de la cuenta de Tripo. @spec REQ-33
#[tauri::command]
pub async fn generation_balance(ai: State<'_, AiState>) -> Result<Balance, UnrealError> {
    let key = ai.secrets.get(&key_account("tripo")).ok_or_else(|| UnrealError::NotFound("no hay clave de Tripo: añádela en Ajustes".into()))?;
    tauri::async_runtime::spawn_blocking(move || TripoClient::new(&key).balance()).await.map_err(|error| UnrealError::Io(error.to_string()))?
}

/// El evento que recibe la interfaz a partir del fichero de trabajo.
fn progress_of(app: &AppHandle, record: &JobRecord) -> Progress {
    let entry = if record.stage == "done" { library(app).ok().and_then(|library| entry_of(&library, record)) } else { None };
    Progress { job_id: record.id.clone(), stage: record.stage.clone(), progress: record.progress, entry, error: record.error.clone() }
}

/// Trabajos de la sesión, también los lanzados por un agente, con su consola. @spec REQ-27
#[tauri::command]
pub fn generation_jobs(app: AppHandle, state: State<'_, GenerationState>) -> Result<Vec<JobRecord>, UnrealError> {
    let since = state.session_start.load(Ordering::Relaxed);
    Ok(board(&app)?.list().into_iter().filter(|r| r.started_at >= since || r.is_active()).take(30).collect())
}

#[tauri::command]
pub fn generation_cancel(state: State<'_, GenerationState>, job_id: String) -> bool {
    let flag = state.cancels.lock().ok().and_then(|jobs| jobs.get(&job_id).cloned());
    flag.map(|f| f.store(true, Ordering::Relaxed)).is_some()
}

// ─── Biblioteca ──────────────────────────────────────────────────────

/// @spec REQ-17
#[tauri::command]
pub fn library_list(app: AppHandle) -> Result<Vec<crate::library::LibraryEntry>, UnrealError> {
    Ok(library(&app)?.list())
}

/// @spec REQ-17
#[tauri::command]
pub fn library_remove(app: AppHandle, id: String) -> Result<(), UnrealError> {
    library(&app)?.remove(&id)
}

/// Ruta absoluta de un fichero de un asset (para importarlo en Unreal o abrir su carpeta). @spec REQ-17, REQ-19
#[tauri::command]
pub fn library_file_path(app: AppHandle, id: String, file: String) -> Result<String, UnrealError> {
    let path = library(&app)?.file_path(&id, &file)?;
    if !path.exists() {
        return Err(UnrealError::NotFound(format!("{} no existe", path.display())));
    }
    Ok(path.to_string_lossy().into_owned())
}

/// @spec REQ-17, REQ-19
#[tauri::command]
pub fn library_record_sent(app: AppHandle, id: String, project: String, asset_path: String) -> Result<crate::library::LibraryEntry, UnrealError> {
    library(&app)?.record_sent(&id, &project, &asset_path)
}

/// Dónde está la biblioteca y cuál es la de siempre. @spec REQ-30
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryLocation {
    pub current: String,
    pub default: String,
    pub is_default: bool,
}

#[tauri::command]
pub fn library_location(app: AppHandle) -> Result<LibraryLocation, UnrealError> {
    let default = library_root(&data_dir(&app)?);
    let current = library(&app)?.root;
    Ok(LibraryLocation { is_default: current == default, current: current.to_string_lossy().into_owned(), default: default.to_string_lossy().into_owned() })
}

/// Cambia la carpeta de la biblioteca (`None` = la de siempre); con `copyExisting` copia lo que ya había. @spec REQ-30
#[tauri::command]
pub fn library_set_location(app: AppHandle, path: Option<String>, copy_existing: bool) -> Result<LibraryLocation, UnrealError> {
    let previous = library(&app)?;
    let dir = config_dir(&app)?;
    let settings = GenerationSettings { library_dir: path.filter(|p| !p.trim().is_empty()), ..GenerationSettings::load(&dir) };
    let target = Library::new(settings.library_root(&data_dir(&app)?));
    std::fs::create_dir_all(&target.root).map_err(|error| io_error(&target.root, &error))?;
    // Comprobar que se puede escribir ANTES de cambiar: si no, la biblioteca quedaría apuntando a un sitio inútil.
    let probe = target.root.join(".dbv-write-test");
    std::fs::write(&probe, b"ok").map_err(|error| io_error(&target.root, &error))?;
    let _ = std::fs::remove_file(&probe);
    if copy_existing {
        target.merge_from(&previous.root)?;
    }
    settings.save(&dir)?;
    library_location(app)
}

/// Importa a la biblioteca modelos o imágenes sueltos, o carpetas de otra biblioteca. Devuelve lo añadido. @spec REQ-30
#[tauri::command]
pub async fn library_import(app: AppHandle, paths: Vec<String>) -> Result<Vec<crate::library::LibraryEntry>, UnrealError> {
    let library = library(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut added = Vec::new();
        for path in paths.iter().map(PathBuf::from) {
            if crate::library::is_library(&path) {
                let before: Vec<String> = library.list().into_iter().map(|e| e.id).collect();
                library.merge_from(&path)?;
                added.extend(library.list().into_iter().filter(|e| !before.contains(&e.id)));
            } else {
                added.push(library.import_file(&path, None)?);
            }
        }
        Ok(added)
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?
}

/// @spec REQ-28
#[tauri::command]
pub fn library_rename(app: AppHandle, id: String, name: String) -> Result<crate::library::LibraryEntry, UnrealError> {
    library(&app)?.rename(&id, &name)
}

/// Carpeta de un asset, para abrirla en el explorador. @spec REQ-30
#[tauri::command]
pub fn library_entry_dir(app: AppHandle, id: String) -> Result<String, UnrealError> {
    let dir = crate::unreal::prepare::inside(&library(&app)?.root, &id)?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Nombre corto deducido del prompt cuando no hay IA que lo proponga. @spec REQ-28
#[tauri::command]
pub fn generation_suggest_name(prompt: String) -> String {
    crate::library::suggest_name(&prompt)
}

/// Animaciones de Tripo por tipo de rig, para elegirlas en la Biblioteca: las 101 del rig v1.0 por grupos y las
/// 16 de los rigs v2/v3. @spec REQ-16
#[tauri::command]
pub fn generation_presets() -> serde_json::Value {
    use super::catalog::{BIPED_PRESETS, GENERIC_PRESETS};
    serde_json::json!({
        "biped": BIPED_PRESETS.iter().map(|(group, list)| serde_json::json!({ "group": group, "names": list.split(' ').collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "generic": GENERIC_PRESETS,
    })
}
