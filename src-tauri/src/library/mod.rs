// =============================================================================
// DBV Unreal AI — Biblioteca local de assets generados
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-17
//
// Todo lo generado (Tripo hoy, World Labs en v0.2) vive en la carpeta de datos de la app,
// FUERA de cualquier proyecto, para reutilizarlo entre proyectos: `library/<id>/` con sus
// ficheros y un índice `library.json` con prompt, proveedor, parámetros, coste y a qué
// proyectos se envió.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::unreal::{io_error, prepare::inside, UnrealError};



const INDEX: &str = "library.json";
/// Candado del índice entre procesos (la ventana y el `--mcp` de un agente escriben a la vez).
const LOCK: &str = "library.json.lock";
/// Un candado más viejo que esto es de un proceso que murió con él tomado.
const STALE_LOCK: Duration = Duration::from_secs(30);

/// Candado dentro del proceso: varias variantes terminan a la vez en hilos distintos.
static IN_PROCESS: Mutex<()> = Mutex::new(());

/// Fichero de candado; se borra al soltarlo.
struct FileLock(PathBuf);

impl FileLock {
    fn take(root: &Path) -> Result<Self, UnrealError> {
        std::fs::create_dir_all(root).map_err(|error| io_error(root, &error))?;
        let path = root.join(LOCK);
        let started = Instant::now();
        loop {
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => break Ok(Self(path)),
                Err(_) if started.elapsed() > Duration::from_secs(10) => break Err(UnrealError::Io(format!("la biblioteca está bloqueada por otro proceso ({})", path.display()))),
                Err(_) => {
                    let stale = std::fs::metadata(&path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > STALE_LOCK);
                    if stale {
                        let _ = std::fs::remove_file(&path);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A qué proyecto y ruta de Unreal se envió un asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentRecord {
    pub project: String,
    pub asset_path: String,
    pub at: u64,
}

/// Un asset de la biblioteca.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    /// `tripo`, `worldlabs`.
    pub provider: String,
    /// `model`, `world`.
    pub kind: String,
    pub prompt: String,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub task_id: String,
    pub created_at: u64,
    #[serde(default)]
    pub credits: Option<f64>,
    /// Ficheros dentro de la carpeta del asset (`model.glb`…).
    pub files: Vec<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub sent_to: Vec<SentRecord>,
    /// Asset del que sale este: el modelo de un rig, el rig de una animación (REQ-16).
    #[serde(default)]
    pub parent: Option<String>,
    /// Datos de la salida del proveedor que no son ficheros (`riggable`, `rig_type`, `normalize_scale`…).
    #[serde(default)]
    pub output: Value,
}

/// Extensiones de modelo que Unreal importa.
pub const MODEL_EXTENSIONS: [&str; 5] = ["glb", "gltf", "fbx", "obj", "usdz"];
/// Extensiones de Gaussian Splatting (mundos/escenarios 3D).
pub const SPLAT_EXTENSIONS: [&str; 2] = ["spz", "ply"];
/// Imágenes que se pueden guardar como referencia (y subir a Tripo).
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Extrae el nombre de fichero legible desde una URL eliminando parámetros de consulta y hashes.
pub fn filename_from_url(url: &str) -> Option<String> {
    let clean = url.split(['?', '#']).next().unwrap_or(url);
    let segment = clean.rsplit('/').next().unwrap_or_default().trim();
    if segment.contains('.') {
        Some(segment.to_string())
    } else {
        None
    }
}

/// Nombre legible a partir del nombre de un fichero (`linterna_hierro-02.glb` → «Linterna hierro 02»).
pub fn name_from_file(path: &Path) -> String {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let spaced = stem.replace(['_', '-', '.'], " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = spaced.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_else(|| "Asset".into())
}

/// Ficheros que acompañan a un modelo y hay que copiar con él: los `uri` relativos de un `.gltf` (búferes y
/// texturas) y el `.mtl` de un `.obj` con sus texturas. Sin ellos el modelo llega a Unreal sin geometría o sin color.
fn companions(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let found: Vec<String> = match extension(path).as_str() {
        "gltf" => serde_json::from_str::<Value>(&text)
            .ok()
            .map(|gltf| ["buffers", "images"].iter().flat_map(|key| gltf.get(*key).and_then(Value::as_array).cloned().unwrap_or_default()).filter_map(|item| item.get("uri").and_then(Value::as_str).map(str::to_string)).collect())
            .unwrap_or_default(),
        "obj" => text.lines().filter_map(|line| line.trim().strip_prefix("mtllib ").map(|m| m.trim().to_string())).collect(),
        "mtl" => text.lines().filter_map(|line| line.trim().strip_prefix("map_").and_then(|rest| rest.split_whitespace().last()).map(str::to_string)).collect(),
        _ => vec![],
    };
    // Solo rutas relativas y dentro de la carpeta del modelo: nada de `data:` ni de salirse con `..`.
    found.into_iter().filter(|uri| !uri.starts_with("data:") && !uri.contains("://") && !uri.contains("..") && !Path::new(uri).is_absolute()).collect()
}

impl LibraryEntry {
    /// El fichero principal: el primer modelo o splat, o el primer fichero si no hay modelos (una imagen).
    pub fn primary_file(&self) -> Option<String> {
        let is_primary = |f: &&String| {
            f.rsplit_once('.').is_some_and(|(_, ext)| {
                let lower = ext.to_lowercase();
                MODEL_EXTENSIONS.contains(&lower.as_str()) || SPLAT_EXTENSIONS.contains(&lower.as_str())
            })
        };
        self.files.iter().find(is_primary).or_else(|| self.files.first()).cloned()
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Nombre de fichero/carpeta seguro a partir de un texto libre (`Cofre del tesoro` → `cofre_del_tesoro`).
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.to_lowercase().chars() {
        let mapped = match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '_',
        };
        if !(mapped == '_' && out.ends_with('_')) {
            out.push(mapped);
        }
    }
    let trimmed: String = out.trim_matches('_').chars().take(40).collect();
    if trimmed.is_empty() {
        "asset".to_string()
    } else {
        trimmed
    }
}

/// Palabras que no describen el objeto en un prompt (artículos, estilo, pose…), en español e inglés.
const FILLER: [&str; 52] = [
    "a", "an", "the", "of", "with", "and", "in", "on", "at", "for", "to", "un", "una", "el", "la", "los", "las", "de", "del", "con", "y", "en", "para", "por", "stylized", "stylised", "low", "poly", "lowpoly", "game", "ready", "full", "body", "standing", "pose", "t-pose", "tpose", "model", "3d", "character", "realistic", "cartoon", "style", "estilo", "modelo", "personaje", "estilizado", "cuerpo", "completo", "listo", "juego", "its",
];

/// Nombre corto deducido de un prompt cuando nadie lo dio (sin IA): las primeras palabras que describen el
/// objeto, en mayúscula inicial («A stylized pirate with a tricorn hat» → «Pirate Tricorn Hat»). @spec REQ-28
pub fn suggest_name(prompt: &str) -> String {
    let words: Vec<String> = prompt
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .map(str::to_lowercase)
        .filter(|w| !w.is_empty() && !FILLER.contains(&w.as_str()))
        .take(3)
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
        })
        .collect();
    if words.is_empty() {
        "Asset".to_string()
    } else {
        words.join(" ")
    }
}

/// La biblioteca en `root` (carpeta de datos de la app + `library/`).
#[derive(Debug, Clone)]
pub struct Library {
    pub root: PathBuf,
}

impl Library {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Entradas, la más reciente primero. Un índice ilegible se trata como vacío (no se pierde nada en disco).
    pub fn list(&self) -> Vec<LibraryEntry> {
        let mut entries: Vec<LibraryEntry> = std::fs::read_to_string(self.root.join(INDEX)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        entries.sort_by_key(|e| std::cmp::Reverse(e.created_at));
        entries
    }

    fn save(&self, entries: &[LibraryEntry]) -> Result<(), UnrealError> {
        std::fs::create_dir_all(&self.root).map_err(|error| io_error(&self.root, &error))?;
        let text = serde_json::to_string_pretty(entries).map_err(|error| UnrealError::Format(error.to_string()))?;
        // Escritura atómica: un corte a mitad no deja el índice a medias.
        let tmp = self.root.join(format!("{INDEX}.tmp"));
        std::fs::write(&tmp, text).map_err(|error| io_error(&tmp, &error))?;
        std::fs::rename(&tmp, self.root.join(INDEX)).map_err(|error| io_error(&self.root, &error))
    }

    /// Carpeta nueva y vacía para un asset (`<slug>-<marca de tiempo>`).
    pub fn new_entry_dir(&self, name: &str) -> Result<(String, PathBuf), UnrealError> {
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let id = format!("{}-{millis}", slug(name));
        let dir = self.root.join(&id);
        std::fs::create_dir_all(&dir).map_err(|error| io_error(&dir, &error))?;
        Ok((id, dir))
    }

    /// Ruta de un fichero de una entrada, sin salirse de la biblioteca.
    pub fn file_path(&self, id: &str, file: &str) -> Result<PathBuf, UnrealError> {
        inside(&inside(&self.root, id)?, file)
    }

    /// Leer, cambiar y guardar el índice sin que otro hilo o proceso escriba en medio: dos variantes que
    /// terminaban a la vez leían el mismo índice y la segunda borraba la entrada de la primera.
    fn update<T>(&self, change: impl FnOnce(&mut Vec<LibraryEntry>) -> Result<T, UnrealError>) -> Result<T, UnrealError> {
        let _thread = IN_PROCESS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let _process = FileLock::take(&self.root)?;
        let mut entries = self.list();
        let value = change(&mut entries)?;
        self.save(&entries)?;
        Ok(value)
    }

    pub fn add(&self, entry: LibraryEntry) -> Result<(), UnrealError> {
        self.update(|entries| {
            entries.retain(|e| e.id != entry.id);
            entries.push(entry);
            Ok(())
        })
    }

    pub fn get(&self, id: &str) -> Option<LibraryEntry> {
        self.list().into_iter().find(|e| e.id == id)
    }

    /// Borra la entrada y su carpeta.
    pub fn remove(&self, id: &str) -> Result<(), UnrealError> {
        let dir = inside(&self.root, id)?;
        self.update(|entries| {
            let before = entries.len();
            entries.retain(|e| e.id != id);
            if entries.len() == before {
                return Err(UnrealError::NotFound(format!("no existe el asset {id}")));
            }
            if dir.is_dir() {
                std::fs::remove_dir_all(&dir).map_err(|error| io_error(&dir, &error))?;
            }
            Ok(())
        })
    }

    /// Cambia el nombre de un asset (REQ-28).
    pub fn rename(&self, id: &str, name: &str) -> Result<LibraryEntry, UnrealError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(UnrealError::Format("el nombre está vacío".into()));
        }
        self.update(|entries| {
            let entry = entries.iter_mut().find(|e| e.id == id).ok_or_else(|| UnrealError::NotFound(format!("no existe el asset {id}")))?;
            entry.name = name.to_string();
            Ok(entry.clone())
        })
    }

    /// Copia un modelo (`.glb`, `.gltf`, `.fbx`, `.obj`, `.usdz`), un Gaussian Splat (`.spz`, `.ply`) o una imagen
    /// a la biblioteca como entrada nueva (proveedor `import`), con los ficheros que necesita (búferes, `.mtl`, texturas). @spec REQ-30, REQ-41
    pub fn import_file(&self, source: &Path, name: Option<&str>) -> Result<LibraryEntry, UnrealError> {
        let ext = extension(source);
        let kind = if MODEL_EXTENSIONS.contains(&ext.as_str()) {
            "model"
        } else if SPLAT_EXTENSIONS.contains(&ext.as_str()) {
            "world"
        } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            "image"
        } else {
            return Err(UnrealError::Format(format!("{} no es un modelo (glb, gltf, fbx, obj, usdz), splat (spz, ply) ni una imagen (png, jpg, webp)", source.display())));
        };
        let folder = source.parent().ok_or_else(|| UnrealError::NotFound(source.display().to_string()))?;
        let file_name = source.file_name().map(|n| n.to_string_lossy().to_string()).ok_or_else(|| UnrealError::NotFound(source.display().to_string()))?;
        let name = name.map(str::to_string).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| name_from_file(source));
        let (id, dir) = self.new_entry_dir(&name)?;
        let copy = |relative: &str| -> Result<(), UnrealError> {
            let from = folder.join(relative);
            let to = inside(&dir, relative)?;
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
            }
            std::fs::copy(&from, &to).map(|_| ()).map_err(|error| io_error(&from, &error))
        };
        let mut pending = vec![file_name.clone()];
        let mut copied: Vec<String> = Vec::new();
        while let Some(relative) = pending.pop() {
            if copied.contains(&relative) {
                continue;
            }
            if let Err(error) = copy(&relative) {
                // El fichero principal es imprescindible; un acompañante que falta se avisa en el registro de Unreal.
                if relative == file_name {
                    let _ = std::fs::remove_dir_all(&dir);
                    return Err(error);
                }
                continue;
            }
            pending.extend(companions(&folder.join(&relative)));
            copied.push(relative);
        }
        let entry = LibraryEntry {
            id,
            name,
            provider: "import".into(),
            kind: kind.into(),
            prompt: String::new(),
            parameters: serde_json::json!({ "source": source.to_string_lossy() }),
            task_id: String::new(),
            created_at: now_secs(),
            credits: None,
            files: vec![file_name],
            thumbnail: None,
            sent_to: vec![],
            parent: None,
            output: Value::Null,
        };
        self.add(entry.clone())?;
        Ok(entry)
    }

    /// Descarga un fichero desde una URL (HTTPS, HTTP o localhost en tests) y lo importa a la biblioteca.
    /// Admite modelos 3D (.glb, .gltf, .fbx, .obj, .usdz), Gaussian Splats (.spz, .ply) e imágenes.
    /// @spec REQ-30
    pub fn import_url(&self, url: &str, name: Option<&str>) -> Result<LibraryEntry, UnrealError> {
        let trimmed_url = url.trim();
        let allowed = trimmed_url.starts_with("https://")
            || trimmed_url.starts_with("http://")
            || (cfg!(test) && trimmed_url.starts_with("http://127.0.0.1"));
        if !allowed {
            return Err(UnrealError::Protocol(format!("protocolo de URL no admitido: {trimmed_url}")));
        }

        let mut deduced_filename = filename_from_url(trimmed_url);
        if deduced_filename.is_none() {
            if let Some(n) = name.filter(|n| n.contains('.')) {
                deduced_filename = Some(n.trim().to_string());
            }
        }
        let file_name = match deduced_filename {
            Some(f) if !f.trim().is_empty() => f,
            _ => {
                return Err(UnrealError::Format(
                    "no se pudo deducir el nombre del fichero a partir de la URL; indica un nombre con extensión (.glb, .spz, etc.)".into(),
                ));
            }
        };

        let ext = file_name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
        let is_supported = MODEL_EXTENSIONS.contains(&ext.as_str())
            || SPLAT_EXTENSIONS.contains(&ext.as_str())
            || IMAGE_EXTENSIONS.contains(&ext.as_str());
        if !is_supported {
            return Err(UnrealError::Format(format!(
                "extensión «.{ext}» no admitida. Se admiten: glb, gltf, fbx, obj, usdz, spz, ply, png, jpg, webp"
            )));
        }

        let temp_dir = std::env::temp_dir().join(format!("dbv_url_import_{}_{}", now_secs(), slug(&file_name)));
        std::fs::create_dir_all(&temp_dir).map_err(|err| io_error(&temp_dir, &err))?;
        let target_path = temp_dir.join(&file_name);

        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .new_agent();

        let outcome = (|| -> Result<LibraryEntry, UnrealError> {
            let response = agent.get(trimmed_url).call().map_err(|error| {
                UnrealError::Unreachable(format!("error descargando desde {trimmed_url}: {error}"))
            })?;
            let status = response.status().as_u16();
            if status != 200 {
                return Err(UnrealError::Protocol(format!("la descarga respondió HTTP {status}")));
            }
            let mut bytes = Vec::new();
            use std::io::Read;
            response
                .into_body()
                .into_reader()
                .take(512 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| io_error(&target_path, &error))?;
            if bytes.len() > 512 * 1024 * 1024 {
                return Err(UnrealError::Protocol("el fichero descargado supera el límite de 512 MB".into()));
            }
            std::fs::write(&target_path, &bytes).map_err(|error| io_error(&target_path, &error))?;

            let mut imported = self.import_file(&target_path, name)?;
            imported.parameters = serde_json::json!({
                "source_url": trimmed_url,
                "file_name": file_name,
            });
            self.update(|entries| {
                if let Some(entry) = entries.iter_mut().find(|e| e.id == imported.id) {
                    entry.parameters = imported.parameters.clone();
                }
                Ok(())
            })?;
            Ok(imported)
        })();

        let _ = std::fs::remove_dir_all(&temp_dir);
        outcome
    }

    /// Une otra biblioteca (una carpeta con `library.json`) a esta: copia las entradas que no tiene, con sus
    /// ficheros, y deja las que ya existen. Devuelve cuántas añadió. @spec REQ-30
    pub fn merge_from(&self, other: &Path) -> Result<usize, UnrealError> {
        if other == self.root {
            return Ok(0);
        }
        let incoming = Library::new(other.to_path_buf()).list();
        let known: Vec<String> = self.list().into_iter().map(|e| e.id).collect();
        let mut added = 0;
        for entry in incoming.into_iter().filter(|e| !known.contains(&e.id)) {
            let from = inside(other, &entry.id)?;
            let to = inside(&self.root, &entry.id)?;
            if from.is_dir() {
                copy_dir(&from, &to)?;
            }
            self.add(entry)?;
            added += 1;
        }
        Ok(added)
    }

    /// Guarda una miniatura PNG hecha por el visor 3D de la app para un asset que no la traía (REQ-31).
    pub fn save_thumbnail(&self, id: &str, png: &[u8]) -> Result<LibraryEntry, UnrealError> {
        const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        if !png.starts_with(&PNG) || png.len() > 4 * 1024 * 1024 {
            return Err(UnrealError::Format("la miniatura no es un PNG o pasa de 4 MB".into()));
        }
        let path = self.file_path(id, "thumbnail.png")?;
        self.update(|entries| {
            let entry = entries.iter_mut().find(|e| e.id == id).ok_or_else(|| UnrealError::NotFound(format!("no existe el asset {id}")))?;
            std::fs::write(&path, png).map_err(|error| io_error(&path, &error))?;
            entry.thumbnail = Some("thumbnail.png".into());
            Ok(entry.clone())
        })
    }

    /// Anota que `id` se envió a un proyecto.
    pub fn record_sent(&self, id: &str, project: &str, asset_path: &str) -> Result<LibraryEntry, UnrealError> {
        self.update(|entries| {
            let entry = entries.iter_mut().find(|e| e.id == id).ok_or_else(|| UnrealError::NotFound(format!("no existe el asset {id}")))?;
            entry.sent_to.push(SentRecord { project: project.to_string(), asset_path: asset_path.to_string(), at: now_secs() });
            Ok(entry.clone())
        })
    }
}

/// Copia una carpeta entera (sin seguir enlaces fuera de ella).
fn copy_dir(from: &Path, to: &Path) -> Result<(), UnrealError> {
    std::fs::create_dir_all(to).map_err(|error| io_error(to, &error))?;
    for item in std::fs::read_dir(from).map_err(|error| io_error(from, &error))?.flatten() {
        let target = to.join(item.file_name());
        let kind = item.file_type().map_err(|error| io_error(&item.path(), &error))?;
        if kind.is_dir() {
            copy_dir(&item.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(item.path(), &target).map_err(|error| io_error(&item.path(), &error))?;
        }
    }
    Ok(())
}

/// ¿Es una carpeta de biblioteca (tiene `library.json`)?
pub fn is_library(path: &Path) -> bool {
    path.join(INDEX).is_file()
}

/// Carpeta de la biblioteca dentro de la carpeta de datos de la app.
pub fn library_root(data_dir: &Path) -> PathBuf {
    data_dir.join("library")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, at: u64) -> LibraryEntry {
        LibraryEntry { id: id.into(), name: id.into(), provider: "tripo".into(), kind: "model".into(), prompt: "p".into(), parameters: Value::Null, task_id: "t".into(), created_at: at, credits: Some(30.0), files: vec!["model.glb".into()], thumbnail: None, sent_to: vec![], parent: None, output: Value::Null }
    }

    /// Ocho hilos añaden a la vez: no se pierde ninguna entrada (antes la última escritura pisaba a las demás).
    /// @spec REQ-17, REQ-29
    #[test]
    fn escrituras_simultaneas_no_pierden_entradas() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("library");
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let library = Library::new(root.clone());
                std::thread::spawn(move || library.add(entry(&format!("e{i}"), i)).unwrap())
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(Library::new(root.clone()).list().len(), 8);
        assert!(!root.join(LOCK).exists(), "el candado se suelta");
    }

    /// Importar un glTF con sus búferes y texturas, un OBJ con su .mtl, renombrar y unir otra biblioteca.
    /// @spec REQ-30, REQ-28
    #[test]
    fn importa_ficheros_con_sus_acompanantes_y_une_bibliotecas() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("descargas");
        std::fs::create_dir_all(src.join("tex")).unwrap();
        std::fs::write(src.join("linterna_hierro.gltf"), r#"{"buffers":[{"uri":"linterna.bin"}],"images":[{"uri":"tex/color.png"},{"uri":"data:image/png;base64,AAAA"},{"uri":"../fuera.png"}]}"#).unwrap();
        std::fs::write(src.join("linterna.bin"), b"bin").unwrap();
        std::fs::write(src.join("tex/color.png"), b"png").unwrap();
        std::fs::write(src.join("barril.obj"), "mtllib barril.mtl\nv 0 0 0\n").unwrap();
        std::fs::write(src.join("barril.mtl"), "newmtl m\nmap_Kd barril_color.jpg\n").unwrap();
        std::fs::write(src.join("barril_color.jpg"), b"jpg").unwrap();
        std::fs::write(src.join("notas.txt"), b"x").unwrap();

        let library = Library::new(dir.path().join("library"));
        let lantern = library.import_file(&src.join("linterna_hierro.gltf"), None).unwrap();
        assert_eq!((lantern.name.as_str(), lantern.provider.as_str(), lantern.kind.as_str()), ("Linterna hierro", "import", "model"));
        for file in ["linterna_hierro.gltf", "linterna.bin", "tex/color.png"] {
            assert!(library.file_path(&lantern.id, file).unwrap().is_file(), "{file}");
        }
        let barrel = library.import_file(&src.join("barril.obj"), Some("Barril")).unwrap();
        assert!(library.file_path(&barrel.id, "barril.mtl").unwrap().is_file() && library.file_path(&barrel.id, "barril_color.jpg").unwrap().is_file());
        assert!(library.import_file(&src.join("notas.txt"), None).is_err());
        assert_eq!(library.rename(&barrel.id, "Barril de roble").unwrap().name, "Barril de roble");
        // La miniatura que hace el visor (REQ-31): solo PNG.
        assert!(library.save_thumbnail(&barrel.id, b"GIF89a").is_err());
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3];
        assert_eq!(library.save_thumbnail(&barrel.id, &png).unwrap().thumbnail.as_deref(), Some("thumbnail.png"));
        assert_eq!(std::fs::read(library.file_path(&barrel.id, "thumbnail.png").unwrap()).unwrap(), png);

        let other = Library::new(dir.path().join("otra"));
        other.import_file(&src.join("barril.obj"), Some("Otro barril")).unwrap();
        assert!(is_library(&other.root));
        assert_eq!(library.merge_from(&other.root).unwrap(), 1);
        assert_eq!(library.merge_from(&other.root).unwrap(), 0, "no duplica");
        assert_eq!(library.list().len(), 3);
    }

    /// @spec REQ-17
    #[test]
    fn guarda_lista_anota_envios_y_borra() {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("library"));
        assert!(library.list().is_empty());
        let (id, folder) = library.new_entry_dir("Cofre del tesoro").unwrap();
        assert!(id.starts_with("cofre_del_tesoro-") && folder.is_dir());
        library.add(entry(&id, 10)).unwrap();
        library.add(entry("otro", 20)).unwrap();
        assert_eq!(library.list().iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["otro", id.as_str()]);
        let sent = library.record_sent(&id, "D:/P/P.uproject", "/Game/DBV/Generated/cofre").unwrap();
        assert_eq!(sent.sent_to[0].asset_path, "/Game/DBV/Generated/cofre");
        library.remove(&id).unwrap();
        assert!(!folder.exists());
        assert!(matches!(library.remove(&id), Err(UnrealError::NotFound(_))));
    }

    /// @spec REQ-17, RNF-IA.3
    #[test]
    fn los_ficheros_no_se_salen_de_la_biblioteca() {
        let library = Library::new(PathBuf::from("lib"));
        assert!(library.file_path("a", "model.glb").is_ok());
        assert!(matches!(library.file_path("..", "x"), Err(UnrealError::Outside(_))));
        assert!(matches!(library.file_path("a", "../../x"), Err(UnrealError::Outside(_))));
    }

    #[test]
    fn el_slug_es_seguro_y_legible() {
        assert_eq!(slug("¡Silla medieval, low-poly!"), "silla_medieval_low_poly");
        assert_eq!(slug("..//"), "asset");
        assert_eq!(slug("Ñandú"), "nandu");
    }

    /// @spec REQ-30, REQ-41
    #[test]
    fn importa_gaussian_splatting_spz_y_ply() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("splats");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("isla_pirata.spz"), b"SPZ1...test data").unwrap();
        std::fs::write(src.join("escenario.ply"), b"ply\nformat binary_little_endian 1.0\n").unwrap();

        let library = Library::new(dir.path().join("library"));
        let spz_entry = library.import_file(&src.join("isla_pirata.spz"), None).unwrap();
        assert_eq!((spz_entry.name.as_str(), spz_entry.kind.as_str()), ("Isla pirata", "world"));
        assert_eq!(spz_entry.primary_file().as_deref(), Some("isla_pirata.spz"));

        let ply_entry = library.import_file(&src.join("escenario.ply"), Some("Mi Escenario")).unwrap();
        assert_eq!((ply_entry.name.as_str(), ply_entry.kind.as_str()), ("Mi Escenario", "world"));
        assert_eq!(ply_entry.primary_file().as_deref(), Some("escenario.ply"));
    }

    /// @spec REQ-30
    #[test]
    fn deduce_nombre_de_fichero_de_url() {
        assert_eq!(filename_from_url("https://cdn.worldlabs.ai/scenes/island.spz?token=123#view").as_deref(), Some("island.spz"));
        assert_eq!(filename_from_url("https://example.com/asset.glb").as_deref(), Some("asset.glb"));
        assert_eq!(filename_from_url("https://example.com/no-extension/"), None);
    }
}

