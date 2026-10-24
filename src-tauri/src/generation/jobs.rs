// =============================================================================
// DBV Unreal AI — Trabajos de generación compartidos entre procesos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-27, REQ-29, RNF-04
//
// La ventana y el servidor MCP propio (`--mcp`, un proceso aparte que lanza el agente) generan con
// Tripo pero no comparten memoria. Cada trabajo se guarda como `<datos>/jobs/<id>.json` (escritura
// atómica): la ventana los lee para enseñar la consola y el progreso de lo que lanzó un agente, y el
// gasto de la sesión se calcula sumándolos, así que el tope (RNF-04) cuenta lo de los dos procesos.
// El fichero lleva la petición EXACTA enviada al proveedor, sin la clave (REQ-27).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::catalog::{self, Operation};
use super::tripo::TripoClient;
use super::{run_operation, JobEvent, OperationRequest, Polling};
use crate::library::{now_secs, Library, LibraryEntry};
use crate::unreal::{io_error, UnrealError};

/// Un trabajo activo que no se actualiza en estos segundos se da por perdido (el proceso que lo llevaba murió).
pub const STALE_SECS: u64 = 90;
/// Cuántos ficheros de trabajo se conservan.
const KEEP_JOBS: usize = 60;
/// Líneas máximas del registro de un trabajo.
const MAX_LOG: usize = 120;
/// Variantes máximas de una petición (REQ-29).
pub const MAX_VARIANTS: u32 = 4;

/// Estado de un trabajo, tal como se guarda y se enseña.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    /// `app` (la ventana) o `agent` (el servidor MCP lanzado por un agente).
    pub source: String,
    /// Operación del catálogo (`text_to_model`, `rig`, `retarget`…).
    pub kind: String,
    pub name: String,
    pub prompt: String,
    /// Punto de la API al que se envió la petición.
    #[serde(default)]
    pub endpoint: String,
    /// Cuerpo exacto enviado (sin la clave).
    #[serde(default)]
    pub request: Value,
    /// `queued`, `submitted`, `running`, `downloading`, `done`, `failed`, `cancelled`.
    pub stage: String,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub task_id: Option<String>,
    pub estimated_credits: f64,
    /// La tarea ya existe en el proveedor: puede cobrarse aunque falle después.
    #[serde(default)]
    pub submitted: bool,
    /// El proveedor la dio por fallida: Tripo devuelve lo reservado y no cobra (`/es/docs/billing`).
    #[serde(default)]
    pub refunded: bool,
    /// Asset de la biblioteca del que sale (rig → modelo, animación → rig).
    #[serde(default)]
    pub parent: Option<String>,
    /// Datos de la salida que no son ficheros (`riggable`, `rig_type`…): es todo el resultado de `rig_check`.
    #[serde(default)]
    pub output: Value,
    #[serde(default)]
    pub credits: Option<f64>,
    pub started_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub entry_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub log: Vec<String>,
}

impl JobRecord {
    pub fn is_active(&self) -> bool {
        !matches!(self.stage.as_str(), "done" | "failed" | "cancelled")
    }

    /// Créditos que cuentan para el tope: lo que está en curso o ya llegó a crear una tarea, salvo lo que el
    /// proveedor dio por fallido (no lo cobra).
    pub fn counted_credits(&self) -> f64 {
        let counts = (self.submitted || self.stage == "queued") && !self.refunded;
        if counts {
            self.credits.unwrap_or(self.estimated_credits)
        } else {
            0.0
        }
    }

    fn note(&mut self, text: &str) {
        let line = format!("+{}s {text}", now_secs().saturating_sub(self.started_at));
        self.log.push(line);
        if self.log.len() > MAX_LOG {
            self.log.remove(0);
        }
    }
}

/// Carpeta de trabajos dentro de la carpeta de datos de la app.
pub fn jobs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("jobs")
}

/// Los ficheros de trabajo y el inicio de la sesión.
#[derive(Clone)]
pub struct JobBoard {
    dir: PathBuf,
}

impl JobBoard {
    pub fn new(data_dir: &Path) -> Self {
        Self { dir: jobs_dir(data_dir) }
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", id.replace(['/', '\\', '.'], "")))
    }

    pub fn write(&self, record: &JobRecord) -> Result<(), UnrealError> {
        std::fs::create_dir_all(&self.dir).map_err(|error| io_error(&self.dir, &error))?;
        let text = serde_json::to_string_pretty(record).map_err(|error| UnrealError::Format(error.to_string()))?;
        let path = self.path(&record.id);
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|error| io_error(&tmp, &error))?;
        std::fs::rename(&tmp, &path).map_err(|error| io_error(&path, &error))
    }

    /// Trabajos, el más reciente primero. Un activo sin latido se muestra como fallido («perdido»).
    pub fn list(&self) -> Vec<JobRecord> {
        let now = now_secs();
        let mut records: Vec<JobRecord> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|e| e == "json"))
            .filter_map(|f| std::fs::read_to_string(f.path()).ok())
            .filter_map(|text| serde_json::from_str::<JobRecord>(&text).ok())
            .map(|mut record| {
                if record.is_active() && now.saturating_sub(record.updated_at) > STALE_SECS {
                    record.stage = "failed".into();
                    record.error.get_or_insert_with(|| "el proceso que lo llevaba se detuvo (sin señales desde hace más de 90 s)".into());
                }
                record
            })
            .collect();
        records.sort_by_key(|r| std::cmp::Reverse((r.started_at, r.id.clone())));
        records
    }

    pub fn get(&self, id: &str) -> Option<JobRecord> {
        self.list().into_iter().find(|r| r.id == id)
    }

    /// Créditos de la sesión: los trabajos iniciados desde `since` (lo de los dos procesos).
    pub fn spent_since(&self, since: u64) -> f64 {
        self.list().iter().filter(|r| r.started_at >= since).map(JobRecord::counted_credits).sum()
    }

    /// Borra los ficheros más antiguos de los `KEEP_JOBS` últimos que ya no están activos.
    pub fn prune(&self) {
        for stale in self.list().into_iter().skip(KEEP_JOBS).filter(|r| !r.is_active()) {
            let _ = std::fs::remove_file(self.path(&stale.id));
        }
    }

    fn session_file(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    /// La ventana marca el inicio de la sesión al arrancar.
    pub fn start_session(&self) -> u64 {
        let now = now_secs();
        let _ = std::fs::create_dir_all(&self.dir).and_then(|_| std::fs::write(self.session_file(), format!("{{\"startedAt\":{now}}}")));
        now
    }

    /// Inicio de la sesión de la ventana; sin ventana abierta, `fallback` (el arranque del proceso que pregunta).
    pub fn session_start(&self, fallback: u64) -> u64 {
        std::fs::read_to_string(self.session_file()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.get("startedAt").and_then(Value::as_u64)).unwrap_or(fallback)
    }
}

/// Quién puede cancelar un trabajo: un indicador por trabajo.
pub type Cancels = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

/// Aviso a quien quiera enterarse de cada cambio (la ventana emite un evento; el MCP no hace nada).
pub type Hook = Arc<dyn Fn(&JobRecord) + Send + Sync>;

/// Todo lo que necesita un trabajo para correr en su hilo.
#[derive(Clone)]
pub struct JobContext {
    pub board: JobBoard,
    pub library: Library,
    pub key: String,
    /// URL de Tripo (cambia en los tests).
    pub base_url: Option<String>,
    pub polling: Polling,
    pub cancels: Cancels,
    pub hook: Hook,
    /// `app` o `agent`.
    pub source: String,
}

/// Nombre de la variante `index` (0 = el original): «Pirata», «Pirata 2», «Pirata 3».
pub fn variant_name(name: &str, index: u32) -> String {
    if index == 0 {
        name.to_string()
    } else {
        format!("{name} {}", index + 1)
    }
}

/// Id único también entre procesos: la ventana y el `--mcp` de un agente pueden lanzar en el mismo milisegundo.
fn new_id(index: u32) -> String {
    let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("job-{millis}-{}-{index}", std::process::id())
}

fn publish(context: &JobContext, record: &JobRecord) {
    let _ = context.board.write(record);
    (context.hook)(record);
}

/// Lanza `variants` trabajos de una operación con los mismos parámetros, cada uno en su hilo. Devuelve sus ids.
/// El llamador ya comprobó la confirmación del coste y el tope sobre el total (`estimate × variants`).
pub fn start_jobs(context: &JobContext, operation: &'static Operation, params: &Value, name: &str, variants: u32, parent: Option<String>) -> Vec<String> {
    let count = variants.clamp(1, MAX_VARIANTS);
    let mut ids = Vec::new();
    for index in 0..count {
        let mut record = JobRecord {
            id: new_id(index),
            source: context.source.clone(),
            kind: operation.name.into(),
            name: variant_name(name, index),
            prompt: params.get("prompt").and_then(Value::as_str).unwrap_or("").trim().to_string(),
            endpoint: operation.endpoint.into(),
            request: params.clone(),
            stage: "queued".into(),
            progress: None,
            task_id: None,
            estimated_credits: catalog::estimate(operation, params),
            submitted: false,
            refunded: false,
            parent: parent.clone(),
            output: Value::Null,
            credits: None,
            started_at: now_secs(),
            updated_at: now_secs(),
            entry_id: None,
            error: None,
            log: vec![],
        };
        record.note("en cola");
        publish(context, &record);
        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut cancels) = context.cancels.lock() {
            cancels.insert(record.id.clone(), cancel.clone());
        }
        ids.push(record.id.clone());
        let request = OperationRequest { operation, params: params.clone(), name: record.name.clone(), parent: parent.clone() };
        let context = context.clone();
        std::thread::spawn(move || run_job(&context, record, &request, &cancel));
    }
    ids
}

/// El cuerpo del trabajo: corre la operación y va dejando el estado en el fichero.
fn run_job(context: &JobContext, mut record: JobRecord, request: &OperationRequest, cancel: &AtomicBool) {
    let client = match &context.base_url {
        Some(base) => TripoClient::with_base_url(&context.key, base),
        None => TripoClient::new(&context.key),
    };
    let mut last_progress: Option<i64> = None;
    let result = run_operation(&client, request, &context.library, cancel, context.polling, &mut |event| {
        match event {
            JobEvent::Request { endpoint, body } => {
                record.endpoint = endpoint.to_string();
                record.request = body.clone();
                record.note(&format!("POST {endpoint} {body}"));
            }
            JobEvent::Task(task_id) => {
                record.task_id = Some(task_id.to_string());
                record.note(&format!("tarea creada: {task_id}"));
            }
            JobEvent::Stage(stage, progress) => {
                record.submitted |= stage == "submitted";
                let rounded = progress.map(|p| p.round() as i64);
                if record.stage != stage || (stage == "running" && rounded != last_progress) {
                    record.note(&format!("{stage}{}", progress.map(|p| format!(" {}%", p.round())).unwrap_or_default()));
                }
                last_progress = rounded;
                record.stage = stage.to_string();
                record.progress = progress;
            }
            JobEvent::Note(text) => record.note(&text),
            JobEvent::NotCharged => {
                record.refunded = true;
                record.note("Tripo no cobra las tareas fallidas");
            }
        }
        record.updated_at = now_secs();
        publish(context, &record);
    });
    match result {
        Ok(outcome) => {
            record.stage = "done".into();
            record.progress = Some(100.0);
            record.output = outcome.output;
            record.credits = outcome.entry.as_ref().and_then(|e| e.credits).or(Some(record.estimated_credits));
            record.entry_id = outcome.entry.as_ref().map(|e| e.id.clone());
            let saved = outcome.entry.map(|e| format!("guardado en la biblioteca: {}", e.id)).unwrap_or_else(|| format!("resultado: {}", record.output));
            record.note(&saved);
        }
        Err(error) => {
            record.stage = if cancel.load(Ordering::Relaxed) { "cancelled".into() } else { "failed".into() };
            record.error = Some(error.to_string());
            record.note(&format!("{}: {error}", record.stage));
        }
    }
    record.updated_at = now_secs();
    publish(context, &record);
    if let Ok(mut cancels) = context.cancels.lock() {
        cancels.remove(&record.id);
    }
}

/// La entrada de la biblioteca de un trabajo terminado.
pub fn entry_of(library: &Library, record: &JobRecord) -> Option<LibraryEntry> {
    record.entry_id.as_deref().and_then(|id| library.get(id))
}

/// Resumen de un trabajo para el modelo (herramientas `generate_3d_model` y `generation_jobs`).
pub fn describe(record: &JobRecord, library: &Library) -> String {
    let progress = record.progress.map(|p| format!(" {}%", p.round())).unwrap_or_default();
    let tail = match record.stage.as_str() {
        "done" => entry_of(library, record).map(|e| format!(" → library id {} (\"{}\", {} {})", e.id, e.name, e.kind, e.files.join(", "))).unwrap_or_else(|| format!(" → {}", record.output)),
        "failed" | "cancelled" => format!(": {}", record.error.clone().unwrap_or_default()),
        _ => String::new(),
    };
    format!("- {} \"{}\" [{}{progress}]{tail}", record.id, record.name, record.stage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, stage: &str, submitted: bool, started: u64) -> JobRecord {
        JobRecord {
            id: id.into(),
            source: "agent".into(),
            kind: "text".into(),
            name: id.into(),
            prompt: "p".into(),
            endpoint: String::new(),
            request: Value::Null,
            stage: stage.into(),
            progress: None,
            task_id: None,
            estimated_credits: 30.0,
            submitted,
            refunded: false,
            parent: None,
            output: Value::Null,
            credits: None,
            started_at: started,
            updated_at: now_secs(),
            entry_id: None,
            error: None,
            log: vec![],
        }
    }

    /// @spec REQ-27, RNF-04
    #[test]
    fn el_gasto_suma_lo_de_los_dos_procesos_y_no_cuenta_lo_rechazado() {
        let dir = tempfile::tempdir().unwrap();
        let board = JobBoard::new(dir.path());
        let session = now_secs();
        board.write(&record("a", "running", true, session)).unwrap();
        board.write(&record("b", "queued", false, session)).unwrap();
        board.write(&record("c", "failed", false, session)).unwrap();
        board.write(&record("d", "done", true, session - 100)).unwrap();
        let mut refunded = record("e", "failed", true, session);
        refunded.refunded = true;
        board.write(&refunded).unwrap();
        assert_eq!(board.spent_since(session), 60.0, "en curso + en cola; el fallido antes de crear tarea no cuenta; el que Tripo dio por fallido no se cobra; el de una sesión anterior tampoco");
    }

    /// @spec REQ-27
    #[test]
    fn un_trabajo_sin_latido_se_da_por_perdido_y_la_sesion_se_comparte() {
        let dir = tempfile::tempdir().unwrap();
        let board = JobBoard::new(dir.path());
        let mut old = record("viejo", "running", true, 1);
        old.updated_at = now_secs() - STALE_SECS - 5;
        board.write(&old).unwrap();
        let listed = board.get("viejo").unwrap();
        assert_eq!(listed.stage, "failed");
        assert!(listed.error.unwrap().contains("se detuvo"));
        assert_eq!(board.session_start(7), 7, "sin ventana, el inicio es el del proceso que pregunta");
        let started = board.start_session();
        assert_eq!(board.session_start(7), started);
    }

    /// @spec REQ-29
    #[test]
    fn las_variantes_se_llaman_pirata_pirata_2_pirata_3() {
        assert_eq!(variant_name("Pirata", 0), "Pirata");
        assert_eq!(variant_name("Pirata", 2), "Pirata 3");
    }

    /// @spec REQ-27
    #[test]
    fn el_nombre_del_fichero_no_puede_salirse_de_la_carpeta() {
        let dir = tempfile::tempdir().unwrap();
        let board = JobBoard::new(dir.path());
        let path = board.path("../../fuera");
        assert!(path.starts_with(dir.path().join("jobs")));
    }
}
