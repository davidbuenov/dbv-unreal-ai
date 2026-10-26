// =============================================================================
// DBV Unreal AI — Memoria del proyecto: frontera con el frontend
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-34, REQ-35, REQ-36
//
// Los comandos reciben la ruta del `.uproject` (la que ya tiene la conexión) y trabajan en su carpeta. Escribir
// solo lo hace `apply`, que exige la huella del texto leído: la tarjeta «¿Guardar…?» y la vista Plan la llevan.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{activate, apply, is_active, parse_tasks, read, summary, Doc, Edit, Snapshot, Task};
use crate::unreal::UnrealError;

/// Todo lo que enseña la vista Plan de un proyecto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    pub active: bool,
    pub memory: Snapshot,
    pub tasks: Snapshot,
    pub task_list: Vec<Task>,
    pub summary: String,
    /// Lo que crearía «Activar» (ficheros que faltan y su contenido), para enseñarlo ANTES (REQ-34).
    pub missing: Vec<Template>,
}

/// Un fichero que «Activar» crearía.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub file: String,
    pub text: String,
}

/// Carpeta del proyecto a partir de la ruta de su `.uproject` (o de la propia carpeta).
fn project_dir(project: &str) -> Result<PathBuf, UnrealError> {
    let path = Path::new(project);
    let dir = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("uproject")) { path.parent().map(Path::to_path_buf) } else { Some(path.to_path_buf()) };
    dir.filter(|d| d.is_dir()).ok_or_else(|| UnrealError::NotFound(format!("no existe el proyecto {project}")))
}

/// Estado de la memoria de un proyecto (para cualquier proceso: la ventana y `--mcp`).
pub fn status_of(dir: &Path) -> Result<MemoryStatus, UnrealError> {
    let tasks = read(dir, Doc::Tasks)?;
    let memory = read(dir, Doc::Memory)?;
    let missing = [(&memory, Doc::Memory), (&tasks, Doc::Tasks)].iter().filter(|(snap, _)| !snap.exists).map(|(_, doc)| Template { file: format!("{}/{}", super::FOLDER, doc.file_name()), text: doc.template().to_string() }).collect();
    let status = MemoryStatus { active: is_active(dir), memory, task_list: parse_tasks(&tasks.text), tasks, summary: summary(dir)?, missing };
    Ok(status)
}

#[tauri::command]
pub fn memory_status(project: String) -> Result<MemoryStatus, UnrealError> {
    status_of(&project_dir(&project)?)
}

/// «Activar memoria del proyecto»: crea lo que falte, nunca sobrescribe. @spec REQ-34
#[tauri::command]
pub fn memory_activate(project: String) -> Result<MemoryStatus, UnrealError> {
    let dir = project_dir(&project)?;
    activate(&dir)?;
    status_of(&dir)
}

/// Aplica un cambio aceptado por el usuario (tarjeta «¿Guardar…?», propuesta de plan o vista Plan). @spec REQ-35, REQ-36
#[tauri::command]
pub fn memory_apply(project: String, doc: Doc, base_hash: String, edit: Edit) -> Result<Snapshot, UnrealError> {
    apply(&project_dir(&project)?, doc, &base_hash, &edit)
}

/// Definiciones de las herramientas de memoria, las mismas que publica el MCP propio (D10). @spec REQ-38
#[tauri::command]
pub fn memory_tool_specs() -> Vec<serde_json::Value> {
    super::tools::specs()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La ruta del `.uproject` y la de su carpeta valen igual; una que no existe, no. @spec REQ-34
    #[test]
    fn acepta_el_uproject_o_su_carpeta() {
        let dir = tempfile::tempdir().unwrap();
        let uproject = dir.path().join("Demo.uproject");
        std::fs::write(&uproject, "{}").unwrap();
        assert_eq!(project_dir(&uproject.to_string_lossy()).unwrap(), dir.path());
        assert_eq!(project_dir(&dir.path().to_string_lossy()).unwrap(), dir.path());
        assert!(matches!(project_dir("Z:/no/existe/Demo.uproject"), Err(UnrealError::NotFound(_))));
        let before = memory_status(uproject.to_string_lossy().into_owned()).unwrap();
        assert_eq!(before.missing.iter().map(|m| m.file.as_str()).collect::<Vec<_>>(), vec!["dbv-unreal-ai/memory.md", "dbv-unreal-ai/task.md"], "se enseña antes lo que se creará");
        let status = memory_activate(uproject.to_string_lossy().into_owned()).unwrap();
        assert!(status.missing.is_empty());
        assert!(status.active && status.tasks.exists && status.task_list.is_empty());
        let edit: Edit = serde_json::from_value(serde_json::json!({ "op": "append", "section": "Tasks", "entry": "- [ ] Build the cabin" })).unwrap();
        let saved = memory_apply(uproject.to_string_lossy().into_owned(), Doc::Tasks, status.tasks.hash, edit).unwrap();
        assert!(saved.text.contains("- [ ] Build the cabin"));
        assert_eq!(memory_status(uproject.to_string_lossy().into_owned()).unwrap().task_list.len(), 1);
    }
}
