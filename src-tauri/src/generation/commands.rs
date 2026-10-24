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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::tripo::{TextToModel, TripoClient, TEXT_TO_MODEL_CREDITS};
use super::{key_account, run_tripo_job, settings_path, within_budget, Polling, Progress, SERVICES};
use crate::ai::secrets::KeyStorage;
use crate::ai::AiState;
use crate::library::{library_root, Library};
use crate::unreal::{io_error, UnrealError};

/// Gasto de la sesión y trabajos en curso.
#[derive(Default)]
pub struct GenerationState {
    pub spent: Mutex<f64>,
    pub jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// Ajustes guardados de la generación.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationSettings {
    /// Tope de créditos por sesión (0 = sin tope).
    pub budget_credits: f64,
}

impl Default for GenerationSettings {
    fn default() -> Self {
        Self { budget_credits: 300.0 }
    }
}

/// Lo que la interfaz necesita para la tarjeta de coste.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationStatus {
    pub spent: f64,
    pub budget: f64,
    pub has_key: HashMap<String, bool>,
    /// Créditos estimados de un texto → modelo de Tripo.
    pub tripo_text_to_model: f64,
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, UnrealError> {
    app.path().app_config_dir().map_err(|error| UnrealError::Io(error.to_string()))
}

fn library(app: &AppHandle) -> Result<Library, UnrealError> {
    let data = app.path().app_data_dir().map_err(|error| UnrealError::Io(error.to_string()))?;
    Ok(Library::new(library_root(&data)))
}

fn load_settings(app: &AppHandle) -> GenerationSettings {
    config_dir(app).ok().and_then(|dir| std::fs::read_to_string(settings_path(&dir)).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
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
    let spent = state.spent.lock().map(|v| *v).unwrap_or(0.0);
    GenerationStatus { spent, budget: load_settings(&app).budget_credits, has_key, tripo_text_to_model: TEXT_TO_MODEL_CREDITS }
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
    std::fs::create_dir_all(&dir).map_err(|error| io_error(&dir, &error))?;
    let text = serde_json::to_string_pretty(&GenerationSettings { budget_credits: credits.max(0.0) }).map_err(|error| UnrealError::Format(error.to_string()))?;
    let path = settings_path(&dir);
    std::fs::write(&path, text).map_err(|error| io_error(&path, &error))
}

/// Petición de texto → modelo desde la interfaz o desde la IA.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripoStart {
    #[serde(flatten)]
    pub request: TextToModel,
    /// Nombre del asset en la biblioteca (por defecto, el prompt).
    #[serde(default)]
    pub name: Option<String>,
    /// El usuario vio el coste y lo aceptó.
    pub confirm_cost: bool,
}

/// Lanza un texto → modelo de Tripo y devuelve el id del trabajo. @spec REQ-15, RNF-04
#[tauri::command]
pub fn generation_tripo_start(app: AppHandle, ai: State<'_, AiState>, state: State<'_, GenerationState>, start: TripoStart) -> Result<String, UnrealError> {
    if !start.confirm_cost {
        return Err(UnrealError::Protocol("falta confirmar el coste antes de generar".into()));
    }
    let key = ai.secrets.get(&key_account("tripo")).ok_or_else(|| UnrealError::NotFound("no hay clave de Tripo: añádela en Ajustes".into()))?;
    let budget = load_settings(&app).budget_credits;
    {
        let mut spent = state.spent.lock().map_err(|_| UnrealError::Io("estado bloqueado".into()))?;
        if !within_budget(*spent, TEXT_TO_MODEL_CREDITS, budget) {
            return Err(UnrealError::Protocol(format!("superaría el tope de {budget} créditos de esta sesión (llevas {spent})")));
        }
        *spent += TEXT_TO_MODEL_CREDITS;
    }
    let library = library(&app)?;
    let job_id = format!("job-{}", crate::library::now_secs());
    let job_id = format!("{job_id}-{}", state.jobs.lock().map(|j| j.len()).unwrap_or(0));
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut jobs) = state.jobs.lock() {
        jobs.insert(job_id.clone(), cancel.clone());
    }
    let name = start.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| start.request.prompt.chars().take(40).collect());
    let id = job_id.clone();
    std::thread::spawn(move || {
        let emit = |progress: Progress| {
            let _ = app.emit("generation-progress", progress);
        };
        let client = TripoClient::new(&key);
        let mut submitted = false;
        let result = run_tripo_job(&client, &start.request, &name, &library, &cancel, Polling::default(), |stage, progress| {
            submitted |= stage == "submitted";
            emit(Progress { job_id: id.clone(), stage: stage.to_string(), progress, entry: None, error: None });
        });
        // Si Tripo rechazó la petición (clave, red), no se creó tarea ni se gastó nada: se devuelve al tope.
        if result.is_err() && !submitted {
            if let Ok(mut spent) = app.state::<GenerationState>().spent.lock() {
                *spent = (*spent - TEXT_TO_MODEL_CREDITS).max(0.0);
            }
        }
        let cancelled = cancel.load(Ordering::Relaxed);
        match result {
            Ok(entry) => emit(Progress { job_id: id.clone(), stage: "done".into(), progress: Some(100.0), entry: Some(entry), error: None }),
            Err(error) => emit(Progress { job_id: id.clone(), stage: if cancelled { "cancelled".into() } else { "failed".into() }, progress: None, entry: None, error: Some(error.to_string()) }),
        }
        if let Ok(mut jobs) = app.state::<GenerationState>().jobs.lock() {
            jobs.remove(&id);
        }
    });
    Ok(job_id)
}

#[tauri::command]
pub fn generation_cancel(state: State<'_, GenerationState>, job_id: String) -> bool {
    let flag = state.jobs.lock().ok().and_then(|jobs| jobs.get(&job_id).cloned());
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
