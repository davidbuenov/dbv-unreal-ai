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
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::unreal::{io_error, prepare::inside, UnrealError};



const INDEX: &str = "library.json";

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

/// La biblioteca en `root` (carpeta de datos de la app + `library/`).
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
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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

    pub fn add(&self, entry: LibraryEntry) -> Result<(), UnrealError> {
        let mut entries = self.list();
        entries.retain(|e| e.id != entry.id);
        entries.push(entry);
        self.save(&entries)
    }

    pub fn get(&self, id: &str) -> Option<LibraryEntry> {
        self.list().into_iter().find(|e| e.id == id)
    }

    /// Borra la entrada y su carpeta.
    pub fn remove(&self, id: &str) -> Result<(), UnrealError> {
        let dir = inside(&self.root, id)?;
        let mut entries = self.list();
        let before = entries.len();
        entries.retain(|e| e.id != id);
        if entries.len() == before {
            return Err(UnrealError::NotFound(format!("no existe el asset {id}")));
        }
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|error| io_error(&dir, &error))?;
        }
        self.save(&entries)
    }

    /// Anota que `id` se envió a un proyecto.
    pub fn record_sent(&self, id: &str, project: &str, asset_path: &str) -> Result<LibraryEntry, UnrealError> {
        let mut entries = self.list();
        let entry = entries.iter_mut().find(|e| e.id == id).ok_or_else(|| UnrealError::NotFound(format!("no existe el asset {id}")))?;
        entry.sent_to.push(SentRecord { project: project.to_string(), asset_path: asset_path.to_string(), at: now_secs() });
        let updated = entry.clone();
        self.save(&entries)?;
        Ok(updated)
    }
}

/// Carpeta de la biblioteca dentro de la carpeta de datos de la app.
pub fn library_root(data_dir: &Path) -> PathBuf {
    data_dir.join("library")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, at: u64) -> LibraryEntry {
        LibraryEntry { id: id.into(), name: id.into(), provider: "tripo".into(), kind: "model".into(), prompt: "p".into(), parameters: Value::Null, task_id: "t".into(), created_at: at, credits: Some(30.0), files: vec!["model.glb".into()], thumbnail: None, sent_to: vec![] }
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
}
