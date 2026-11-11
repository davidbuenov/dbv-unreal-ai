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

use super::jobs::{entry_of, resume_orphans, start_jobs, Cancels, JobBoard, JobContext, JobRecord};
use super::tripo::{Balance, TripoClient};
use super::{key_account, service_keys, settings_path, tools, within_budget, Polling, Progress, SERVICES};
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
    /// Tope de créditos de Tripo por sesión (0 = sin tope).
    pub budget_credits: f64,
    /// Tope de créditos de World Labs por sesión (0 = sin tope): son otra cuenta, con otros precios.
    #[serde(default = "default_worldlabs_budget")]
    pub worldlabs_budget_credits: f64,
    /// Carpeta de la biblioteca elegida por el usuario; sin ella, `Documentos/DBV Unreal AI/Library` (REQ-30, REQ-53).
    #[serde(default)]
    pub library_dir: Option<String>,
    /// Ya se trajo (o no había nada que traer) la biblioteca de su ubicación antigua, `<datos>/library` (REQ-53).
    #[serde(default)]
    pub library_migrated: bool,
    /// Cuántos assets se trajeron, para la nota de Ajustes.
    #[serde(default)]
    pub library_migrated_count: usize,
}

impl Default for GenerationSettings {
    fn default() -> Self {
        Self { budget_credits: 300.0, worldlabs_budget_credits: default_worldlabs_budget(), library_dir: None, library_migrated: false, library_migrated_count: 0 }
    }
}

/// Cabe un mundo completo (`marble-1.1`, 1.580) pero no dos ni una malla HQ (3.500) sin subirlo.
fn default_worldlabs_budget() -> f64 {
    2000.0
}

impl GenerationSettings {
    /// Tope de un servicio (`tripo` o `worldlabs`).
    pub fn budget_for(&self, service: &str) -> f64 {
        if service == "worldlabs" { self.worldlabs_budget_credits } else { self.budget_credits }
    }
}

/// Biblioteca por defecto: en Documentos; sin carpeta Documentos (un servicio, un Linux mínimo), la de datos.
pub fn default_library_root(data_dir: &std::path::Path) -> PathBuf {
    crate::paths::documents_library().unwrap_or_else(|| library_root(data_dir))
}

/// Deja la biblioteca lista al arrancar (ventana y `--mcp`): crea su carpeta aunque esté vacía y, una sola vez y si
/// no se eligió otra en Ajustes, une a ella la de la ubicación antigua SIN borrar nada (R-21). Devuelve la carpeta.
/// @spec REQ-53
pub fn ensure_library(config_dir: &std::path::Path, data_dir: &std::path::Path) -> Result<PathBuf, UnrealError> {
    ensure_library_in(config_dir, data_dir, &default_library_root(data_dir))
}

/// `ensure_library` con la carpeta por defecto como dato (los tests no tocan la carpeta Documentos de verdad).
fn ensure_library_in(config_dir: &std::path::Path, data_dir: &std::path::Path, default_root: &std::path::Path) -> Result<PathBuf, UnrealError> {
    let mut settings = GenerationSettings::load(config_dir);
    let root = settings.library_dir.as_deref().filter(|d| !d.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| default_root.to_path_buf());
    std::fs::create_dir_all(&root).map_err(|error| io_error(&root, &error))?;
    let legacy = library_root(data_dir);
    if settings.library_dir.is_none() && !settings.library_migrated {
        // `merge_from` toma el cerrojo de la biblioteca y no repite ids: dos procesos a la vez no duplican nada.
        let added = if legacy != root && crate::library::is_library(&legacy) { Library::new(root.clone()).merge_from(&legacy)? } else { 0 };
        settings.library_migrated = true;
        settings.library_migrated_count = added;
        settings.save(config_dir)?;
    }
    Ok(root)
}

impl GenerationSettings {
    /// Lee los ajustes de `config_dir` (los mismos para la ventana y para `--mcp`).
    pub fn load(config_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(settings_path(config_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    /// Carpeta de la biblioteca: la elegida o la de siempre.
    pub fn library_root(&self, data_dir: &std::path::Path) -> PathBuf {
        self.library_dir.as_deref().filter(|d| !d.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| default_library_root(data_dir))
    }

    pub(crate) fn save(&self, config_dir: &std::path::Path) -> Result<(), UnrealError> {
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
    /// Lo de Tripo, como siempre (la píldora de gasto y su tarjeta de coste).
    pub spent: f64,
    pub budget: f64,
    /// Gasto y tope por servicio (`tripo`, `worldlabs`).
    pub spent_by_service: HashMap<String, f64>,
    pub budget_by_service: HashMap<String, f64>,
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
    if let (Ok(config), Ok(data)) = (config_dir(app), data_dir(app)) {
        let _ = ensure_library(&config, &data);
    }
    if let Ok(library) = library(app) {
        allow_assets(app, &library.root);
    }
}

/// La interfaz lee miniaturas y modelos con `convertFileSrc` (protocolo `asset`). `tauri.conf.json` solo abre la
/// biblioteca de siempre; una carpeta elegida por el usuario (REQ-30) se abre aquí, en tiempo de ejecución.
fn allow_assets(app: &AppHandle, dir: &std::path::Path) {
    let _ = app.asset_protocol_scope().allow_directory(dir, true);
}

/// Créditos de la sesión, sumando también lo que lanzaron los agentes.
fn session_spent(app: &AppHandle, state: &GenerationState, service: &str) -> f64 {
    board(app).map(|board| board.spent_since(state.session_start.load(Ordering::Relaxed), service)).unwrap_or(0.0)
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
    let settings = load_settings(&app);
    let spent_by_service: HashMap<String, f64> = SERVICES.iter().map(|s| (s.to_string(), session_spent(&app, &state, s))).collect();
    let budget_by_service: HashMap<String, f64> = SERVICES.iter().map(|s| (s.to_string(), settings.budget_for(s))).collect();
    GenerationStatus { spent: spent_by_service.get("tripo").copied().unwrap_or(0.0), budget: settings.budget_credits, spent_by_service, budget_by_service, has_key }
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

/// Cambia el tope de créditos por sesión de un servicio (`tripo` si no se dice). @spec RNF-04, REQ-18
#[tauri::command]
pub fn generation_set_budget(app: AppHandle, credits: f64, service: Option<String>) -> Result<(), UnrealError> {
    let dir = config_dir(&app)?;
    let current = GenerationSettings::load(&dir);
    let credits = credits.max(0.0);
    let updated = if service.as_deref() == Some("worldlabs") {
        GenerationSettings { worldlabs_budget_credits: credits, ..current }
    } else {
        GenerationSettings { budget_credits: credits, ..current }
    };
    updated.save(&dir)
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
    /// De qué servicio son los créditos (`tripo` o `worldlabs`).
    pub service: String,
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
        service: super::service_of(launch.operation.category).into(),
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
    let service = super::service_of(launch.operation.category);
    let name = super::service_name(service);
    let key = ai.secrets.get(&key_account(service)).ok_or_else(|| UnrealError::NotFound(format!("no hay clave de {name}: añádela en Ajustes")))?;
    let budget = load_settings(&app).budget_for(service);
    let spent = session_spent(&app, &state, service);
    if !within_budget(spent, total, budget) {
        return Err(UnrealError::Protocol(format!("superaría el tope de {budget} créditos de {name} de esta sesión (llevas {spent} y esto son {total}); súbelo en Ajustes → {name}")));
    }
    let context = job_context(&app, &state, key)?;
    let job_ids = start_jobs(&context, launch.operation, &launch.params, &launch.name, launch.variants, launch.parent.clone());
    Ok(job_ids)
}

/// Contexto de los trabajos de la ventana: cada cambio llega a la interfaz como evento.
fn job_context(app: &AppHandle, state: &GenerationState, key: String) -> Result<JobContext, UnrealError> {
    let emitter = app.clone();
    let context = JobContext {
        board: board(app)?,
        library: library(app)?,
        key,
        base_url: None,
        polling: Polling::default(),
        cancels: state.cancels.clone(),
        hook: Arc::new(move |record: &JobRecord| {
            let _ = emitter.emit("generation-progress", progress_of(&emitter, record));
        }),
        source: "app".into(),
    };
    Ok(context)
}

/// Catálogo de operaciones de Tripo (para `tripo_operations` del asistente integrado). @spec REQ-33
#[tauri::command]
pub fn generation_operations(operation: Option<String>) -> Result<String, UnrealError> {
    let text = tools::operations_text(&serde_json::json!({ "operation": operation.unwrap_or_default() })).map_err(UnrealError::NotFound)?;
    Ok(text)
}

/// Saldo de la cuenta de Tripo. @spec REQ-33
#[tauri::command]
pub async fn generation_balance(ai: State<'_, AiState>) -> Result<Balance, UnrealError> {
    let key = ai.secrets.get(&key_account("tripo")).ok_or_else(|| UnrealError::NotFound("no hay clave de Tripo: añádela en Ajustes".into()))?;
    let balance = tauri::async_runtime::spawn_blocking(move || TripoClient::new(&key).balance()).await.map_err(|error| UnrealError::Io(error.to_string()))??;
    Ok(balance)
}

/// Saldo de la cuenta de World Labs. @spec REQ-18
#[tauri::command]
pub async fn generation_worldlabs_balance(ai: State<'_, AiState>) -> Result<super::worldlabs::WorldLabsCredits, UnrealError> {
    let key = ai.secrets.get(&key_account("worldlabs")).ok_or_else(|| UnrealError::NotFound("no hay clave de World Labs: añádela en Ajustes".into()))?;
    let balance = tauri::async_runtime::spawn_blocking(move || super::worldlabs::WorldLabsClient::new(&key).credits()).await.map_err(|error| UnrealError::Io(error.to_string()))??;
    Ok(balance)
}

/// Saldo de la cuenta de ElevenLabs (caracteres restantes y límite de suscripción). @spec REQ-77
#[tauri::command]
pub async fn generation_elevenlabs_balance(ai: State<'_, AiState>) -> Result<super::audio::ElevenLabsBalance, UnrealError> {
    let key = ai.secrets.get(&key_account("elevenlabs")).ok_or_else(|| UnrealError::NotFound("no hay clave de ElevenLabs: añádela en Ajustes".into()))?;
    let balance = tauri::async_runtime::spawn_blocking(move || super::audio::get_elevenlabs_subscription(&key)).await.map_err(|error| UnrealError::Io(error.to_string()))??;
    Ok(balance)
}

/// El evento que recibe la interfaz a partir del fichero de trabajo.
fn progress_of(app: &AppHandle, record: &JobRecord) -> Progress {
    let entry = if record.stage == "done" { library(app).ok().and_then(|library| entry_of(&library, record)) } else { None };
    Progress { job_id: record.id.clone(), stage: record.stage.clone(), progress: record.progress, entry, error: record.error.clone() }
}

/// Trabajos de la sesión, también los lanzados por un agente, con su consola. De paso reanuda los huérfanos de
/// Tripo y de World Labs: un proceso que se cerró con una tarea en marcha no pierde lo que ya se cobró. @spec REQ-27, REQ-70
#[tauri::command]
pub fn generation_jobs(app: AppHandle, ai: State<'_, AiState>, state: State<'_, GenerationState>) -> Result<Vec<JobRecord>, UnrealError> {
    let keys = service_keys(|service| ai.secrets.get(&key_account(service)));
    if !keys.is_empty() {
        resume_orphans(&job_context(&app, &state, String::new())?, &keys);
    }
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
    let lib = library(&app)?;
    let _ = lib.deduplicate();
    Ok(lib.list())
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

/// Código Python fijo que coloca un mundo ya cargado (escala métrica y suelo en Z = 0), o `null` si la entrada no
/// trae esos metadatos. La ventana lo ejecuta con el `run_python` del toolset DBV, como el `--mcp`. @spec REQ-18
#[tauri::command]
pub fn library_world_place_code(app: AppHandle, id: String, splat_label: String, collider_asset: Option<String>, name: String) -> Result<Option<String>, UnrealError> {
    let entry = library(&app)?.get(&id).ok_or_else(|| UnrealError::NotFound(format!("no library asset with id {id}")))?;
    Ok(crate::library::world_place_code(&entry, &splat_label, collider_asset.as_deref(), &name))
}

/// Dónde está la biblioteca y cuál es la de siempre. @spec REQ-30
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryLocation {
    pub current: String,
    pub default: String,
    pub is_default: bool,
    /// Assets traídos de la ubicación antigua al pasar a Documentos (REQ-53).
    pub migrated: usize,
}

/// Dónde está la biblioteca. La carpeta se crea si falta, así «Abrir carpeta» nunca abre algo que no existe (REQ-53).
#[tauri::command]
pub fn library_location(app: AppHandle) -> Result<LibraryLocation, UnrealError> {
    let default = default_library_root(&data_dir(&app)?);
    let current = library(&app)?.root;
    std::fs::create_dir_all(&current).map_err(|error| io_error(&current, &error))?;
    let migrated = load_settings(&app).library_migrated_count;
    Ok(LibraryLocation { is_default: current == default, current: current.to_string_lossy().into_owned(), default: default.to_string_lossy().into_owned(), migrated })
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
    allow_assets(&app, &target.root);
    library_location(app)
}

/// Importa a la biblioteca modelos o imágenes sueltos, o carpetas de otra biblioteca. Devuelve lo añadido. @spec REQ-30, REQ-42
#[tauri::command]
pub async fn library_import(app: AppHandle, paths: Vec<String>) -> Result<Vec<crate::library::LibraryEntry>, UnrealError> {
    let library = library(&app)?;
    let emitter = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut targets: Vec<(PathBuf, bool)> = Vec::new();
        for path in paths.iter().map(PathBuf::from) {
            if crate::library::is_library(&path) {
                targets.push((path, true));
            } else if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry_res in entries.flatten() {
                        let sub_path = entry_res.path();
                        if sub_path.is_file() {
                            targets.push((sub_path, false));
                        }
                    }
                }
            } else {
                targets.push((path, false));
            }
        }

        let total = targets.len();
        let mut added = Vec::new();

        for (idx, (target, is_merge)) in targets.into_iter().enumerate() {
            let item_name = crate::library::name_from_file(&target);
            let _ = emitter.emit("library-import-progress", serde_json::json!({
                "current": idx + 1,
                "total": total,
                "name": item_name,
                "done": false
            }));

            if is_merge {
                let before: Vec<String> = library.list().into_iter().map(|e| e.id).collect();
                library.merge_from(&target)?;
                added.extend(library.list().into_iter().filter(|e| !before.contains(&e.id)));
            } else if let Ok(imported) = library.import_file(&target, None) {
                added.push(imported);
            }
        }

        let _ = emitter.emit("library-import-progress", serde_json::json!({
            "current": total,
            "total": total,
            "name": "",
            "done": true
        }));

        let outcome = if added.is_empty() && total > 0 {
            Err(UnrealError::NotFound("no se encontraron modelos, splats ni manifiestos válidos".into()))
        } else {
            Ok(added)
        };
        outcome
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?
}

/// Descarga un asset desde una URL y lo importa a la biblioteca. @spec REQ-30
#[tauri::command]
pub async fn library_import_url(app: AppHandle, url: String, name: Option<String>) -> Result<crate::library::LibraryEntry, UnrealError> {
    let library = library(&app)?;
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        library.import_url(&url, name.as_deref())
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?;
    outcome
}

/// Guarda una prenda ajustada exportada desde Unreal en la biblioteca de la app. @spec REQ-100
#[tauri::command]
pub async fn library_save_outfit_from_unreal(
    app: AppHandle,
    staging_dir: String,
    name: Option<String>,
) -> Result<crate::library::LibraryEntry, UnrealError> {
    let library = library(&app)?;
    let path = PathBuf::from(staging_dir);
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        library.import_tailored_outfit(&path, name.as_deref())
    })
    .await
    .map_err(|error| UnrealError::Io(error.to_string()))?;
    outcome
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

/// Deja ver en la interfaz una imagen de referencia elegida por el usuario (solo imágenes). @spec REQ-32
#[tauri::command]
pub fn generation_allow_reference(app: AppHandle, path: String) -> Result<(), UnrealError> {
    let file = PathBuf::from(&path);
    let ext = file.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !crate::library::IMAGE_EXTENSIONS.contains(&ext.as_str()) || !file.is_file() {
        return Err(UnrealError::Format(format!("{path} no es una imagen (png, jpg, webp)")));
    }
    app.asset_protocol_scope().allow_file(&file).map_err(|error| UnrealError::Io(error.to_string()))
}

/// Comprueba si el microservicio de audio local está activo. @spec REQ-71
#[tauri::command]
pub fn generation_check_local_audio(url: Option<String>) -> super::audio::LocalAudioStatus {
    super::audio::check_local_service(url.as_deref())
}

/// Genera un efecto de sonido (local o ElevenLabs) o busca en Freesound y lo guarda en la biblioteca. @spec REQ-71, REQ-76
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn generation_generate_sound(
    app: AppHandle,
    provider: String,
    prompt: String,
    name: Option<String>,
    duration: Option<f32>,
    r#loop: Option<bool>,
    local_url: Option<String>,
    negative_prompt: Option<String>,
    steps: Option<u32>,
    cfg_scale: Option<f32>,
    seed: Option<u64>,
) -> Result<crate::library::LibraryEntry, UnrealError> {
    let dur = duration.unwrap_or(5.0);
    let lib = library(&app)?;

    let (bytes, source) = match provider.as_str() {
        "local" => {
            let audio_bytes = super::audio::generate_local_sound(
                local_url.as_deref(),
                &prompt,
                negative_prompt.as_deref(),
                dur,
                steps,
                cfg_scale,
                seed,
            )?;
            (audio_bytes, "local")
        }
        "elevenlabs" => {
            let key = app
                .state::<AiState>()
                .secrets
                .get(&key_account("elevenlabs"))
                .ok_or_else(|| UnrealError::Io("no hay clave guardada para ElevenLabs en Ajustes".into()))?;
            let audio_bytes = super::audio::generate_elevenlabs_sfx(&key, &prompt, Some(dur), r#loop.unwrap_or(false))?;
            (audio_bytes, "elevenlabs")
        }
        "freesound" => {
            let token = app
                .state::<AiState>()
                .secrets
                .get(&key_account("freesound"))
                .ok_or_else(|| UnrealError::Io("no hay token guardado para Freesound en Ajustes".into()))?;
            let items = super::audio::search_freesound(&token, &prompt, 5)?;
            let first = items.into_iter().next().ok_or_else(|| UnrealError::NotFound("no se encontraron sonidos en Freesound para esa búsqueda".into()))?;
            let download_url = first.preview_url.or(first.download_url).ok_or_else(|| UnrealError::NotFound("el sonido no tiene URL de descarga disponible".into()))?;
            let agent = ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(30)))
                .build()
                .new_agent();
            let mut res = agent.get(&download_url).call().map_err(|e| UnrealError::Io(format!("error descargando audio de Freesound: {e}")))?;
            let mut audio_bytes = Vec::new();
            std::io::Read::read_to_end(&mut res.body_mut().as_reader(), &mut audio_bytes).map_err(|e| UnrealError::Io(format!("error leyendo bytes descargados de Freesound: {e}")))?;
            (audio_bytes, "freesound")
        }
        other => return Err(UnrealError::NotFound(format!("proveedor de audio desconocido: {other}"))),
    };

    let ext = if bytes.starts_with(b"RIFF") {
        "wav"
    } else if bytes.starts_with(b"ID3") || bytes.starts_with(&[0xff, 0xfb]) || bytes.starts_with(&[0xff, 0xf3]) || bytes.starts_with(&[0xff, 0xf2]) {
        "mp3"
    } else if bytes.starts_with(b"OggS") {
        "ogg"
    } else {
        "wav"
    };

    let default_name = format!("sound_{}", crate::library::now_secs());
    let entry_name = name.filter(|n| !n.trim().is_empty()).unwrap_or(default_name);
    let (id, dir) = lib.new_entry_dir(&entry_name)?;
    let sound_filename = format!("sound.{ext}");
    let final_sound = dir.join(&sound_filename);
    std::fs::write(&final_sound, &bytes).map_err(|e| io_error(&final_sound, &e))?;

    let entry = crate::library::LibraryEntry {
        id: id.clone(),
        name: entry_name,
        provider: source.to_string(),
        kind: "sound".to_string(),
        prompt: prompt.clone(),
        parameters: serde_json::json!({
            "duration": dur,
            "source": source,
            "loop": r#loop.unwrap_or(false),
            "negative_prompt": negative_prompt,
            "steps": steps,
            "cfg_scale": cfg_scale,
            "seed": seed,
        }),
        task_id: String::new(),
        created_at: crate::library::now_secs(),
        credits: None,
        files: vec![sound_filename],
        thumbnail: None,
        sent_to: Vec::new(),
        parent: None,
        output: serde_json::Value::Null,
    };

    lib.add(entry.clone())?;
    Ok(entry)
}

/// Busca efectos de sonido en Freesound. @spec REQ-71
#[tauri::command]
pub async fn generation_search_freesound(
    app: AppHandle,
    query: String,
    page_size: Option<usize>,
) -> Result<Vec<super::audio::FreesoundItem>, UnrealError> {
    let token = app
        .state::<AiState>()
        .secrets
        .get(&key_account("freesound"))
        .unwrap_or_default();
    if token.is_empty() {
        return Err(UnrealError::NotFound("no hay token guardado para Freesound en Ajustes".to_string()));
    }
    super::audio::search_freesound(&token, &query, page_size.unwrap_or(15))
}

/// Guarda un sonido descargado desde una URL (p.ej. preview o descarga de Freesound) directamente en la biblioteca. @spec REQ-71
#[tauri::command]
pub async fn generation_save_sound_url(
    app: AppHandle,
    url: String,
    name: String,
    prompt: Option<String>,
    duration: Option<f32>,
) -> Result<crate::library::LibraryEntry, UnrealError> {
    let lib = library(&app)?;
    let dur = duration.unwrap_or(0.0);

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .new_agent();

    let mut res = agent
        .get(&url)
        .call()
        .map_err(|e| UnrealError::Io(format!("error descargando audio: {e}")))?;

    let mut audio_bytes = Vec::new();
    std::io::Read::read_to_end(&mut res.body_mut().as_reader(), &mut audio_bytes)
        .map_err(|e| UnrealError::Io(format!("error leyendo bytes de audio: {e}")))?;

    let ext = if audio_bytes.starts_with(b"RIFF") {
        "wav"
    } else if audio_bytes.starts_with(b"ID3")
        || audio_bytes.starts_with(&[0xff, 0xfb])
        || audio_bytes.starts_with(&[0xff, 0xf3])
        || audio_bytes.starts_with(&[0xff, 0xf2])
    {
        "mp3"
    } else if audio_bytes.starts_with(b"OggS") {
        "ogg"
    } else {
        "mp3"
    };

    let default_name = format!("sound_{}", crate::library::now_secs());
    let entry_name = if name.trim().is_empty() { default_name } else { name };
    let (id, dir) = lib.new_entry_dir(&entry_name)?;
    let sound_filename = format!("sound.{ext}");
    let final_sound = dir.join(&sound_filename);
    std::fs::write(&final_sound, &audio_bytes).map_err(|e| io_error(&final_sound, &e))?;

    let p = prompt.unwrap_or_default();
    let entry = crate::library::LibraryEntry {
        id: id.clone(),
        name: entry_name,
        provider: "freesound".to_string(),
        kind: "sound".to_string(),
        prompt: p,
        parameters: serde_json::json!({
            "duration": dur,
            "source": "freesound",
            "url": url,
        }),
        task_id: String::new(),
        created_at: crate::library::now_secs(),
        credits: None,
        files: vec![sound_filename],
        thumbnail: None,
        sent_to: Vec::new(),
        parent: None,
        output: serde_json::Value::Null,
    };

    lib.add(entry.clone())?;
    Ok(entry)
}

/// Miniatura hecha por el visor 3D para un asset que no la traía (los importados). @spec REQ-31
#[tauri::command]
pub fn library_save_thumbnail(app: AppHandle, id: String, png: Vec<u8>) -> Result<crate::library::LibraryEntry, UnrealError> {
    library(&app)?.save_thumbnail(&id, &png)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryEntry;

    fn entry(id: &str) -> LibraryEntry {
        LibraryEntry { id: id.into(), name: id.into(), provider: "import".into(), kind: "world".into(), prompt: String::new(), parameters: serde_json::Value::Null, task_id: String::new(), created_at: 1, credits: None, files: vec!["scene.ply".into()], thumbnail: None, sent_to: vec![], parent: None, output: serde_json::Value::Null }
    }

    /// Primer arranque sin nada: la carpeta existe aunque esté vacía y queda anotado. @spec REQ-53
    #[test]
    fn crea_la_biblioteca_al_arrancar_aunque_este_vacia() {
        let dir = tempfile::tempdir().unwrap();
        let (config, data, docs) = (dir.path().join("config"), dir.path().join("data"), dir.path().join("Documents/DBV Unreal AI/Library"));
        assert_eq!(ensure_library_in(&config, &data, &docs).unwrap(), docs);
        assert!(docs.is_dir());
        let settings = GenerationSettings::load(&config);
        assert!(settings.library_migrated && settings.library_migrated_count == 0);
    }

    /// La biblioteca antigua se une a la nueva una vez, sin borrarla y sin duplicar al repetir. @spec REQ-53
    #[test]
    fn migra_la_biblioteca_antigua_una_vez_y_sin_borrarla() {
        let dir = tempfile::tempdir().unwrap();
        let (config, data, docs) = (dir.path().join("config"), dir.path().join("data"), dir.path().join("Documents/Library"));
        let legacy = Library::new(library_root(&data));
        let (id, folder) = legacy.new_entry_dir("Isla").unwrap();
        std::fs::write(folder.join("scene.ply"), b"ply
").unwrap();
        legacy.add(entry(&id)).unwrap();
        ensure_library_in(&config, &data, &docs).unwrap();
        let moved = Library::new(docs.clone());
        assert_eq!(moved.list().len(), 1);
        assert!(docs.join(&id).join("scene.ply").is_file(), "los ficheros viajan con la entrada");
        assert_eq!(legacy.list().len(), 1, "la ubicación antigua se conserva");
        assert_eq!(GenerationSettings::load(&config).library_migrated_count, 1);
        // Una segunda vez (o un `--mcp` a la vez) no duplica ni vuelve a copiar.
        legacy.add(entry("otra-2")).unwrap();
        ensure_library_in(&config, &data, &docs).unwrap();
        assert_eq!(moved.list().len(), 1);
    }

    /// Una carpeta elegida en Ajustes se respeta: ni se cambia ni se migra a ella. @spec REQ-53
    #[test]
    fn respeta_la_carpeta_elegida_a_mano() {
        let dir = tempfile::tempdir().unwrap();
        let (config, data, docs, mine) = (dir.path().join("config"), dir.path().join("data"), dir.path().join("Documents/Library"), dir.path().join("Mia"));
        let legacy = Library::new(library_root(&data));
        legacy.add(entry("vieja-1")).unwrap();
        GenerationSettings { library_dir: Some(mine.to_string_lossy().into()), ..GenerationSettings::default() }.save(&config).unwrap();
        assert_eq!(ensure_library_in(&config, &data, &docs).unwrap(), mine);
        assert!(Library::new(mine).list().is_empty() && !docs.exists());
    }
}
