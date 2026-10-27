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
use super::{finish_task, run_operation, JobEvent, OperationRequest, Outcome, Polling};
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

/// Activo y sin latido: el proceso que lo llevaba murió.
fn is_stale(record: &JobRecord, now: u64) -> bool {
    record.is_active() && now.saturating_sub(record.updated_at) > STALE_SECS
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

    /// Los ficheros de trabajo tal cual están en disco.
    fn raw(&self) -> Vec<JobRecord> {
        std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|e| e == "json"))
            .filter_map(|f| std::fs::read_to_string(f.path()).ok())
            .filter_map(|text| serde_json::from_str::<JobRecord>(&text).ok())
            .collect()
    }

    /// Trabajos, el más reciente primero. Un activo sin latido se muestra como fallido («perdido»).
    pub fn list(&self) -> Vec<JobRecord> {
        let now = now_secs();
        let mut records: Vec<JobRecord> = self
            .raw()
            .into_iter()
            .map(|mut record| {
                if is_stale(&record, now) {
                    record.stage = "failed".into();
                    record.error.get_or_insert_with(|| "el proceso que lo llevaba se detuvo (sin señales desde hace más de 90 s)".into());
                }
                record
            })
            .collect();
        records.sort_by_key(|r| std::cmp::Reverse((r.started_at, r.id.clone())));
        records
    }

    fn claim_path(&self, id: &str) -> PathBuf {
        self.path(id).with_extension("claim")
    }

    /// Trabajos huérfanos que se pueden reanudar: su proceso murió con la tarea YA creada en Tripo (se cobra
    /// igual). Cada uno se reclama creando `<id>.claim` de forma atómica, así que si la ventana y un `--mcp` lo
    /// intentan a la vez, solo uno lo consigue. Una reclamación sin latido (su proceso también murió) se renueva.
    pub fn claim_orphans(&self) -> Vec<JobRecord> {
        let now = now_secs();
        let mut claimed = Vec::new();
        for record in self.raw().into_iter().filter(|r| is_stale(r, now) && r.submitted && r.task_id.is_some()) {
            let claim = self.claim_path(&record.id);
            let old_claim = std::fs::metadata(&claim).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_secs() > STALE_SECS);
            if old_claim {
                let _ = std::fs::remove_file(&claim);
            }
            if std::fs::OpenOptions::new().write(true).create_new(true).open(&claim).is_ok() {
                claimed.push(record);
            }
        }
        claimed
    }

    fn release(&self, id: &str) {
        let _ = std::fs::remove_file(self.claim_path(id));
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

/// Reanuda los trabajos huérfanos (ver `JobBoard::claim_orphans`): sigue esperando su tarea y la descarga a la
/// biblioteca como si nada. Devuelve sus ids. Lo llaman la ventana (al consultar los trabajos) y el `--mcp`.
pub fn resume_orphans(context: &JobContext) -> Vec<String> {
    let mut ids = Vec::new();
    for mut record in context.board.claim_orphans() {
        let Some(operation) = catalog::find(&record.kind) else {
            context.board.release(&record.id);
            continue;
        };
        let task_id = record.task_id.clone().unwrap_or_default();
        record.note(&format!("reanudado por {}: el proceso anterior se detuvo y la tarea {task_id} sigue en Tripo", context.source));
        record.updated_at = now_secs();
        publish(context, &record);
        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut cancels) = context.cancels.lock() {
            cancels.insert(record.id.clone(), cancel.clone());
        }
        ids.push(record.id.clone());
        // El cuerpo guardado es el que se envió (fuentes ya resueltas): sirve tal cual para la entrada de la biblioteca.
        let request = OperationRequest { operation, params: record.request.clone(), name: record.name.clone(), parent: record.parent.clone() };
        let context = context.clone();
        std::thread::spawn(move || {
            let body = request.params.clone();
            let client = match &context.base_url {
                Some(base) => TripoClient::with_base_url(&context.key, base),
                None => TripoClient::new(&context.key),
            };
            drive(&context, record, &cancel, |mut report| finish_task(&client, &request, body, task_id, &context.library, &cancel, context.polling, &mut report));
        });
    }
    ids
}

/// El cuerpo del trabajo: corre la operación y va dejando el estado en el fichero.
fn run_job(context: &JobContext, record: JobRecord, request: &OperationRequest, cancel: &AtomicBool) {
    if request.operation.category == "worldlabs" {
        drive(context, record, cancel, |mut report| run_worldlabs_operation(context, request, cancel, &mut report));
    } else {
        let client = match &context.base_url {
            Some(base) => TripoClient::with_base_url(&context.key, base),
            None => TripoClient::new(&context.key),
        };
        drive(context, record, cancel, |mut report| run_operation(&client, request, &context.library, cancel, context.polling, &mut report));
    }
}

fn run_worldlabs_operation(context: &JobContext, request: &OperationRequest, cancel: &AtomicBool, report: &mut impl FnMut(JobEvent)) -> Result<Outcome, UnrealError> {
    let client = match &context.base_url {
        Some(base) => super::worldlabs::WorldLabsClient::with_base_url(&context.key, base),
        None => super::worldlabs::WorldLabsClient::new(&context.key),
    };

    report(JobEvent::Request { endpoint: request.operation.endpoint, body: &request.params });
    report(JobEvent::Stage("submitted", None));

    let started = std::time::Instant::now();
    let outcome = match request.operation.name {
        "generate_world" => {
            let prompt = request.params.get("prompt").and_then(Value::as_str).unwrap_or("");
            let model = request.params.get("model").and_then(Value::as_str);
            let image_base64 = request.params.get("image").and_then(Value::as_str).and_then(|p| {
                let path = Path::new(p);
                if path.is_file() {
                    std::fs::read(path).ok().map(|b| {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(&b)
                    })
                } else {
                    None
                }
            });

            let op_id = client.generate_world(prompt, model, image_base64.as_deref())?;
            report(JobEvent::Task(&op_id));
            report(JobEvent::Stage("running", None));

            let world_id = loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(UnrealError::Protocol("cancelado por el usuario".into()));
                }
                if started.elapsed() > context.polling.max_wait {
                    return Err(UnrealError::Timeout(format!("la operación {op_id} no terminó a tiempo")));
                }
                let op = client.operation(&op_id)?;
                if op.done {
                    if let Some(err) = op.error {
                        return Err(UnrealError::Protocol(format!("World Labs falló: {err}")));
                    }
                    break op.world_id.ok_or_else(|| UnrealError::Protocol("World Labs terminó sin world_id".into()))?;
                }
                report(JobEvent::Stage("running", op.progress));
                std::thread::sleep(context.polling.interval);
            };

            report(JobEvent::Stage("downloading", None));
            let world_json = client.world(&world_id)?;

            let temp_dir = std::env::temp_dir().join(format!("dbv_worldlabs_{}_{}", crate::library::now_secs(), world_id));
            std::fs::create_dir_all(&temp_dir).map_err(|e| crate::unreal::io_error(&temp_dir, &e))?;
            let temp_manifest = temp_dir.join("manifest.json");
            let manifest_value = serde_json::json!({
                "id": world_id,
                "prompt": prompt,
                "rawResponse": world_json
            });
            std::fs::write(&temp_manifest, serde_json::to_string_pretty(&manifest_value).unwrap_or_default())
                .map_err(|e| crate::unreal::io_error(&temp_manifest, &e))?;

            let entry_result = context.library.import_worldlabs_manifest(&temp_manifest, Some(&request.name));
            let _ = std::fs::remove_dir_all(&temp_dir);
            let entry = entry_result?;
            Outcome { entry: Some(entry), output: world_json, task_id: op_id }
        }
        "export_world_mesh" => {
            let world_id = request.params.get("world_id")
                .or_else(|| request.params.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let op_id = client.export_mesh(world_id)?;
            report(JobEvent::Task(&op_id));
            report(JobEvent::Stage("running", None));

            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err(UnrealError::Protocol("cancelado por el usuario".into()));
                }
                if started.elapsed() > context.polling.max_wait {
                    return Err(UnrealError::Timeout(format!("la exportación {op_id} no terminó a tiempo")));
                }
                let op = client.operation(&op_id)?;
                if op.done {
                    if let Some(err) = op.error {
                        return Err(UnrealError::Protocol(format!("World Labs falló: {err}")));
                    }
                    break;
                }
                report(JobEvent::Stage("running", op.progress));
                std::thread::sleep(context.polling.interval);
            }

            report(JobEvent::Stage("downloading", None));
            let world_json = client.world(world_id)?;
            Outcome { entry: None, output: world_json, task_id: op_id }
        }
        other => return Err(UnrealError::NotFound(format!("operación de World Labs no implementada: {other}"))),
    };
    Ok(outcome)
}

/// Lleva un trabajo (nuevo o reanudado): traduce los eventos al fichero de trabajo y deja el resultado.
fn drive(context: &JobContext, mut record: JobRecord, cancel: &AtomicBool, work: impl FnOnce(&mut dyn FnMut(JobEvent)) -> Result<Outcome, UnrealError>) {

    let mut last_progress: Option<i64> = None;
    let result = work(&mut |event| {
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
    context.board.release(&record.id);
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

    /// Hallazgo de S18 con Tripo real: el `--mcp` de un agente se cerró con la tarea al 31 % y el trabajo quedó
    /// «perdido» aunque Tripo lo terminaba (y lo cobraba). Otro proceso lo reclama UNA vez y lo termina.
    /// @spec REQ-27
    #[test]
    fn un_huerfano_con_tarea_se_reanuda_una_sola_vez_y_acaba_en_la_biblioteca() {
        use crate::unreal::mcp_client::tests::{fake_server, http};
        let (cdn, _cdn) = fake_server(vec![http("200 OK", "", "glTF-binario")]);
        let model_url = cdn.replace("/mcp", "/m.glb");
        let (api, _api) = fake_server(vec![http("200 OK", "", &format!(r#"{{"code":0,"data":{{"status":"success","progress":100,"consumed_credit":20,"output":{{"model_url":"{model_url}"}}}}}}"#))]);
        let dir = tempfile::tempdir().unwrap();
        let board = JobBoard::new(dir.path());
        let mut orphan = record("huerfano", "running", true, 1);
        orphan.kind = "text_to_model".into();
        orphan.task_id = Some("t-31".into());
        orphan.request = serde_json::json!({ "prompt": "cofre", "model": "v3.1-20260211" });
        orphan.updated_at = now_secs() - STALE_SECS - 5;
        board.write(&orphan).unwrap();
        let mut lost = record("sin-tarea", "queued", false, 1);
        lost.updated_at = orphan.updated_at;
        board.write(&lost).unwrap();
        let context = JobContext {
            board: board.clone(),
            library: Library::new(dir.path().join("library")),
            key: "k".into(),
            base_url: Some(api.trim_end_matches("/mcp").to_string()),
            polling: Polling { interval: std::time::Duration::from_millis(1), max_wait: std::time::Duration::from_secs(5) },
            cancels: Cancels::default(),
            hook: Arc::new(|_| {}),
            source: "app".into(),
        };
        assert_eq!(resume_orphans(&context), vec!["huerfano".to_string()], "sin tarea creada no hay nada que reanudar");
        assert!(resume_orphans(&context).is_empty(), "ya reclamado: otro proceso no lo reanuda dos veces");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while board.get("huerfano").is_some_and(|r| r.is_active()) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let done = board.get("huerfano").unwrap();
        assert_eq!(done.stage, "done", "{:?}", done.log);
        assert_eq!(done.credits, Some(20.0));
        assert!(done.log.iter().any(|l| l.contains("reanudado")));
        let entry = entry_of(&context.library, &done).unwrap();
        assert_eq!((entry.name.as_str(), entry.task_id.as_str(), entry.prompt.as_str()), ("huerfano", "t-31", "cofre"));
        assert!(!board.claim_path("huerfano").exists(), "al terminar suelta la reclamación");
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
