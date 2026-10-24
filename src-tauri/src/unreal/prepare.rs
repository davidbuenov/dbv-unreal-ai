// =============================================================================
// DBV Unreal AI — Diagnóstico y preparación de un proyecto de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-04, REQ-05
//
// Qué hace falta para que un proyecto hable con la IA (verificado en DBVMCPFirst y en las
// lecciones de MCPCLiente):
//   1. Plugins activos en el `.uproject`: ModelContextProtocol, ToolsetRegistry,
//      PythonScriptPlugin y los toolsets de Epic que exponen las >100 herramientas reales
//      (EditorToolset, AutomationTestToolset, SlateInspectorToolset).
//   2. «Auto Start Server» en los ajustes DEL USUARIO (Saved/Config/<Plataforma>Editor/
//      EditorPerProjectUserSettings.ini): no es un ajuste del `.uproject` y Epic lo trae apagado.
//   3. El toolset de DBV Unreal AI en `Plugins/` (variante Python o C++).
//
// Reglas: el plan se calcula sin escribir nada y se enseña como diff; aplicar guarda antes
// una copia de seguridad fuera del proyecto; revertir deja los ficheros byte a byte como
// estaban. Nunca se escribe fuera de la carpeta del proyecto.

use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::discovery::{self, EngineInstall, McpSettings, UProject, MCP_SECTION};
use super::{ini, io_error, UnrealError};

/// Plugins de Epic que la preparación activa.
pub const REQUIRED_PLUGINS: [&str; 6] = ["ModelContextProtocol", "ToolsetRegistry", "PythonScriptPlugin", "EditorToolset", "AutomationTestToolset", "SlateInspectorToolset"];
/// Plugin de solo contenido con el toolset en Python.
pub const PYTHON_PLUGIN: &str = "DBVUnrealAI";
/// Plugin C++ (registra el toolset desde C++ y delega en el paquete Python).
pub const CPP_PLUGIN: &str = "DBVUnrealAICpp";
/// Versión mínima del motor con el servidor MCP integrado.
pub const MIN_ENGINE: (u32, u32) = (5, 8);

/// Variante del toolset (P-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Variant {
    Python,
    Cpp,
}

impl Variant {
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("cpp") | Some("c++") => Self::Cpp,
            _ => Self::Python,
        }
    }

    /// Plugins propios que instala esta variante (la C++ necesita también el paquete Python).
    pub fn plugins(self) -> &'static [&'static str] {
        match self {
            Self::Python => &[PYTHON_PLUGIN],
            Self::Cpp => &[PYTHON_PLUGIN, CPP_PLUGIN],
        }
    }
}

/// Una comprobación del diagnóstico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// `engine`, `plugin:<Nombre>`, `autoStart`, `toolset`.
    pub id: String,
    pub ok: bool,
    /// Dato para la interfaz (versión encontrada, versión instalada…). El texto lo pone el frontend.
    pub detail: String,
}

/// Diagnóstico de un proyecto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub project: UProject,
    pub engine: Option<EngineInstall>,
    pub mcp: McpSettings,
    pub checks: Vec<Check>,
    /// Versión del toolset instalada en el proyecto, si lo está.
    pub toolset_installed: Option<String>,
    /// Versión que trae esta app.
    pub toolset_bundled: String,
    pub ready: bool,
}

/// Un fichero que la preparación crea o modifica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// Ruta relativa a la carpeta del proyecto, con `/`.
    pub relative: String,
    /// Contenido actual (`None` si el fichero no existe).
    pub before: Option<String>,
    pub after: String,
}

/// Lo que haría «Preparar».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparePlan {
    pub variant: Variant,
    pub changes: Vec<FileChange>,
    /// Hay que reiniciar el editor para que surta efecto (siempre que haya cambios).
    pub needs_restart: bool,
}

/// Manifiesto de una copia de seguridad.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    project_dir: String,
    files: Vec<BackedUp>,
    created_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackedUp {
    relative: String,
    existed: bool,
}

// ─── Origen del toolset ──────────────────────────────────────────────

/// Carpeta `unreal-plugin/` que acompaña a la app: variable `DBV_UNREAL_PLUGIN_DIR`, junto al
/// ejecutable (recursos de Tauri en Windows/Linux), `Resources/` en macOS o el repo en desarrollo.
pub fn toolset_source() -> Result<PathBuf, UnrealError> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("DBV_UNREAL_PLUGIN_DIR").map(PathBuf::from).into_iter().collect();
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        candidates.push(exe_dir.join("unreal-plugin"));
        candidates.push(exe_dir.join("..").join("Resources").join("unreal-plugin"));
        candidates.push(exe_dir.join("..").join("lib").join("DBV Unreal AI").join("unreal-plugin"));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("unreal-plugin"));
    candidates
        .into_iter()
        .find(|dir| dir.join(PYTHON_PLUGIN).join(format!("{PYTHON_PLUGIN}.uplugin")).is_file())
        .ok_or_else(|| UnrealError::NotFound("no se encuentra el toolset de DBV Unreal AI (unreal-plugin/) junto a la aplicación".into()))
}

/// `VersionName` de un `.uplugin`.
pub fn plugin_version(uplugin: &Path) -> Option<String> {
    let text = std::fs::read_to_string(uplugin).ok()?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    value.get("VersionName").and_then(Value::as_str).map(str::to_string)
}

// ─── Diagnóstico ─────────────────────────────────────────────────────

/// `5.8` → `(5, 8)`; `None` si no es una versión (GUID de una compilación propia).
pub fn parse_association(association: &str) -> Option<(u32, u32)> {
    let (major, minor) = association.trim().split_once('.')?;
    Some((major.parse().ok()?, minor.split('.').next()?.parse().ok()?))
}

fn project_dir(uproject: &Path) -> &Path {
    uproject.parent().unwrap_or(Path::new("."))
}

/// Diagnóstico del proyecto (REQ-04): qué falta para conectarlo.
pub fn readiness(uproject: &Path, source: &Path) -> Result<Readiness, UnrealError> {
    let project = discovery::read_uproject(uproject)?;
    let dir = project_dir(uproject);
    let engines = discovery::find_engines();
    let engine = discovery::engine_for(&engines, &project.engine_association).cloned();
    let version = parse_association(&project.engine_association);
    let mut checks = vec![Check {
        id: "engine".into(),
        ok: version.is_some_and(|v| v >= MIN_ENGINE) && engine.is_some(),
        detail: engine.as_ref().map(|e| e.version.clone()).unwrap_or_else(|| project.engine_association.clone()),
    }];
    for name in REQUIRED_PLUGINS {
        let enabled = project.plugins.iter().any(|p| p.name == name && p.enabled);
        checks.push(Check { id: format!("plugin:{name}"), ok: enabled, detail: String::new() });
    }
    let mcp = discovery::read_mcp_settings(dir);
    checks.push(Check { id: "autoStart".into(), ok: mcp.auto_start == Some(true), detail: mcp.port.to_string() });
    let installed = plugin_version(&dir.join("Plugins").join(PYTHON_PLUGIN).join(format!("{PYTHON_PLUGIN}.uplugin")));
    let bundled = plugin_version(&source.join(PYTHON_PLUGIN).join(format!("{PYTHON_PLUGIN}.uplugin"))).unwrap_or_default();
    let toolset_enabled = project.plugins.iter().any(|p| p.name == PYTHON_PLUGIN && p.enabled);
    checks.push(Check { id: "toolset".into(), ok: toolset_enabled && installed.as_deref() == Some(bundled.as_str()), detail: installed.clone().unwrap_or_default() });
    let ready = checks.iter().all(|c| c.ok);
    let result = Readiness { project, engine, mcp, checks, toolset_installed: installed, toolset_bundled: bundled, ready };
    Ok(result)
}

// ─── Plan ────────────────────────────────────────────────────────────

/// JSON con el estilo de Unreal: tabuladores y el salto de línea original.
pub fn unreal_json(value: &Value, newline: &str) -> Result<String, UnrealError> {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    serde::Serialize::serialize(value, &mut serializer).map_err(|error| UnrealError::Format(error.to_string()))?;
    let text = String::from_utf8(out).map_err(|error| UnrealError::Format(error.to_string()))?;
    Ok(format!("{}{newline}", text.replace('\n', newline)))
}

/// `.uproject` con `plugins` activados (los que falten se añaden al final, en orden).
pub fn enable_plugins(text: &str, plugins: &[&str]) -> Result<String, UnrealError> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|error| UnrealError::Format(format!(".uproject: {error}")))?;
    let root = value.as_object_mut().ok_or_else(|| UnrealError::Format(".uproject no es un objeto JSON".into()))?;
    let list = root.entry("Plugins").or_insert_with(|| json!([]));
    let entries = list.as_array_mut().ok_or_else(|| UnrealError::Format("Plugins no es una lista".into()))?;
    for name in plugins {
        match entries.iter_mut().find(|p| p.get("Name").and_then(Value::as_str) == Some(name)) {
            Some(entry) => entry["Enabled"] = json!(true),
            None => entries.push(json!({ "Name": name, "Enabled": true })),
        }
    }
    unreal_json(&value, newline)
}

/// Ficheros de un plugin de origen (sin cachés de Python), como rutas relativas con `/`.
fn plugin_files(source: &Path, plugin: &str) -> Result<Vec<(String, PathBuf)>, UnrealError> {
    let root = source.join(plugin);
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| e.file_name() != "__pycache__") {
        let entry = entry.map_err(|error| UnrealError::Io(error.to_string()))?;
        if entry.file_type().is_file() && entry.path().extension().is_none_or(|ext| ext != "pyc") {
            let relative = entry.path().strip_prefix(&root).map_err(|error| UnrealError::Io(error.to_string()))?;
            let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            files.push((format!("Plugins/{plugin}/{}", parts.join("/")), entry.path().to_path_buf()));
        }
    }
    files.sort();
    Ok(files)
}

fn read_optional(path: &Path) -> Result<Option<String>, UnrealError> {
    let result = match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, &error)),
    };
    result
}

fn relative_of(dir: &Path, path: &Path) -> String {
    path.strip_prefix(dir).unwrap_or(path).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Cambios que haría «Preparar», sin escribir nada (REQ-05).
pub fn plan(uproject: &Path, source: &Path, variant: Variant, port: Option<u16>) -> Result<PreparePlan, UnrealError> {
    let dir = project_dir(uproject);
    let mut changes = Vec::new();
    // 1. Plugins del `.uproject`.
    let before = std::fs::read_to_string(uproject).map_err(|error| io_error(uproject, &error))?;
    let mut wanted: Vec<&str> = REQUIRED_PLUGINS.to_vec();
    wanted.extend_from_slice(variant.plugins());
    let after = enable_plugins(&before, &wanted)?;
    if after != before {
        changes.push(FileChange { relative: relative_of(dir, uproject), before: Some(before), after });
    }
    // 2. Arranque automático del servidor MCP (ajustes del usuario).
    let settings = discovery::user_settings_ini(dir);
    let current = read_optional(&settings)?;
    let mut text = ini::set(current.as_deref().unwrap_or(""), MCP_SECTION, "bAutoStartServer", "True");
    if let Some(port) = port {
        text = ini::set(&text, MCP_SECTION, "ServerPortNumber", &port.to_string());
    }
    if current.as_deref() != Some(text.as_str()) {
        changes.push(FileChange { relative: relative_of(dir, &settings), before: current, after: text });
    }
    // 3. El toolset.
    for plugin in variant.plugins() {
        for (relative, origin) in plugin_files(source, plugin)? {
            let after = std::fs::read_to_string(&origin).map_err(|error| io_error(&origin, &error))?;
            let before = read_optional(&dir.join(&relative))?;
            if before.as_deref() != Some(after.as_str()) {
                changes.push(FileChange { relative, before, after });
            }
        }
    }
    let result = PreparePlan { variant, needs_restart: !changes.is_empty(), changes };
    Ok(result)
}

// ─── Aplicar y revertir ──────────────────────────────────────────────

/// Ruta dentro de `dir` para un `relative` del plan; rechaza `..`, raíces y prefijos de unidad.
pub fn inside(dir: &Path, relative: &str) -> Result<PathBuf, UnrealError> {
    let path = Path::new(relative);
    if relative.is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(UnrealError::Outside(relative.to_string()));
    }
    Ok(dir.join(path))
}

fn backup_id(project: &str) -> String {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let name: String = project.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    format!("{millis}-{name}")
}

/// Carpetas que tendrá que crear el plan, de fuera hacia dentro y sin repetir (las quita «Revertir»).
fn missing_dirs(dir: &Path, targets: &[PathBuf]) -> Vec<String> {
    let mut created: Vec<String> = Vec::new();
    for target in targets {
        let mut missing: Vec<PathBuf> = target.ancestors().skip(1).take_while(|a| *a != dir && !a.exists()).map(Path::to_path_buf).collect();
        missing.reverse();
        for folder in missing.iter().map(|d| relative_of(dir, d)) {
            if !created.contains(&folder) {
                created.push(folder);
            }
        }
    }
    created
}

/// Escribe los ficheros del plan (las copias y el manifiesto ya están guardados).
fn write_changes(plan: &PreparePlan, targets: &[PathBuf]) -> Result<(), UnrealError> {
    for (change, target) in plan.changes.iter().zip(targets) {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
        }
        std::fs::write(target, &change.after).map_err(|error| io_error(target, &error))?;
    }
    Ok(())
}

/// Aplica el plan TODO O NADA y devuelve el id de la copia de seguridad (REQ-05).
///
/// Orden: copias de lo que existe → manifiesto → escrituras. Si una escritura falla a mitad, se
/// revierte sola con ese mismo manifiesto y se devuelve el error (antes el manifiesto se escribía
/// al final y un fallo a mitad dejaba el proyecto a medias y sin forma de revertirlo).
pub fn apply(uproject: &Path, plan: &PreparePlan, backups: &Path) -> Result<String, UnrealError> {
    let dir = project_dir(uproject);
    let targets: Vec<PathBuf> = plan.changes.iter().map(|c| inside(dir, &c.relative)).collect::<Result<_, _>>()?;
    let id = backup_id(uproject.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().as_str());
    let store = backups.join(&id);
    std::fs::create_dir_all(&store).map_err(|error| io_error(&store, &error))?;
    let mut manifest = Manifest { project_dir: dir.to_string_lossy().into_owned(), files: Vec::new(), created_dirs: missing_dirs(dir, &targets) };
    for (change, target) in plan.changes.iter().zip(&targets) {
        let existed = target.is_file();
        if existed {
            let copy = inside(&store.join("files"), &change.relative)?;
            std::fs::create_dir_all(copy.parent().unwrap_or(&store)).map_err(|error| io_error(&copy, &error))?;
            std::fs::copy(target, &copy).map_err(|error| io_error(target, &error))?;
        }
        manifest.files.push(BackedUp { relative: change.relative.clone(), existed });
    }
    let text = serde_json::to_string_pretty(&manifest).map_err(|error| UnrealError::Format(error.to_string()))?;
    std::fs::write(store.join("manifest.json"), text).map_err(|error| io_error(&store, &error))?;
    let result = match write_changes(plan, &targets) {
        Ok(()) => Ok(id),
        Err(error) => Err(match revert(uproject, backups, &id) {
            Ok(()) => UnrealError::Io(format!("{error} — no se cambió nada: la preparación se deshizo sola")),
            Err(rollback) => UnrealError::Io(format!("{error} — y no se pudo deshacer ({rollback}): revierte con la copia {id}")),
        }),
    };
    result
}

/// Deshace una preparación: restaura lo que existía, borra lo creado y sus carpetas (REQ-05).
pub fn revert(uproject: &Path, backups: &Path, id: &str) -> Result<(), UnrealError> {
    let dir = project_dir(uproject);
    let store = inside(backups, id)?;
    let text = std::fs::read_to_string(store.join("manifest.json")).map_err(|_| UnrealError::NotFound(format!("no existe la copia de seguridad {id}")))?;
    let manifest: Manifest = serde_json::from_str(&text).map_err(|error| UnrealError::Format(error.to_string()))?;
    if Path::new(&manifest.project_dir) != dir {
        return Err(UnrealError::NotFound(format!("la copia {id} es de otro proyecto: {}", manifest.project_dir)));
    }
    for file in &manifest.files {
        let target = inside(dir, &file.relative)?;
        if file.existed {
            let copy = inside(&store.join("files"), &file.relative)?;
            std::fs::copy(&copy, &target).map_err(|error| io_error(&copy, &error))?;
        } else if target.is_file() {
            std::fs::remove_file(&target).map_err(|error| io_error(&target, &error))?;
        }
    }
    // De dentro hacia fuera, y solo si quedaron vacías (no se borra nada que el usuario añadiera después).
    for relative in manifest.created_dirs.iter().rev() {
        let folder = inside(dir, relative)?;
        if folder.read_dir().is_ok_and(|mut entries| entries.next().is_none()) {
            std::fs::remove_dir(&folder).map_err(|error| io_error(&folder, &error))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPROJECT: &str = "{\r\n\t\"FileVersion\": 3,\r\n\t\"EngineAssociation\": \"5.8\",\r\n\t\"Category\": \"\",\r\n\t\"Plugins\": [\r\n\t\t{\r\n\t\t\t\"Name\": \"ModelContextProtocol\",\r\n\t\t\t\"Enabled\": false\r\n\t\t}\r\n\t]\r\n}\r\n";

    fn source() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("unreal-plugin")
    }

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Demo.uproject");
        std::fs::write(&path, UPROJECT).unwrap();
        (dir, path)
    }

    /// Todos los ficheros del proyecto con su contenido, para comparar byte a byte.
    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<(String, Vec<u8>)> = walkdir::WalkDir::new(dir)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file() || e.file_type().is_dir())
            .map(|e| (relative_of(dir, e.path()), if e.file_type().is_file() { std::fs::read(e.path()).unwrap() } else { Vec::new() }))
            .collect();
        files.sort();
        files
    }

    /// @spec REQ-05
    #[test]
    fn activa_plugins_conservando_orden_tabuladores_y_crlf() {
        let out = enable_plugins(UPROJECT, &["ModelContextProtocol", "EditorToolset"]).unwrap();
        assert!(out.starts_with("{\r\n\t\"FileVersion\": 3,\r\n\t\"EngineAssociation\": \"5.8\","), "{out}");
        assert!(out.contains("\"Name\": \"ModelContextProtocol\",\r\n\t\t\t\"Enabled\": true"));
        assert!(out.contains("\"Name\": \"EditorToolset\",\r\n\t\t\t\"Enabled\": true"));
        assert!(!out.contains('\n') || out.matches('\n').count() == out.matches("\r\n").count());
        assert_eq!(enable_plugins(&out, &["EditorToolset"]).unwrap(), out, "idempotente");
    }

    /// @spec REQ-04
    #[test]
    fn el_diagnostico_de_un_proyecto_en_blanco_marca_lo_que_falta() {
        let (_dir, path) = project();
        let report = readiness(&path, &source()).unwrap();
        assert!(!report.ready);
        let failing: Vec<&str> = report.checks.iter().filter(|c| !c.ok).map(|c| c.id.as_str()).collect();
        for expected in ["plugin:ModelContextProtocol", "plugin:EditorToolset", "autoStart", "toolset"] {
            assert!(failing.contains(&expected), "{expected} debería fallar: {failing:?}");
        }
        assert_eq!(report.toolset_bundled, "0.2.0");
    }

    /// @spec REQ-04, REQ-05
    #[test]
    fn preparar_deja_el_proyecto_listo_salvo_el_motor_y_revertir_lo_deja_igual() {
        let (dir, path) = project();
        let backups = tempfile::tempdir().unwrap();
        let original = snapshot(dir.path());
        let plan = plan(&path, &source(), Variant::Python, Some(8123)).unwrap();
        assert!(plan.needs_restart);
        assert!(plan.changes.iter().any(|c| c.relative == "Plugins/DBVUnrealAI/Content/Python/init_unreal.py" && c.before.is_none()));
        assert!(plan.changes.iter().all(|c| !c.relative.contains("__pycache__")));
        let id = apply(&path, &plan, backups.path()).unwrap();
        let report = readiness(&path, &source()).unwrap();
        let failing: Vec<&str> = report.checks.iter().filter(|c| !c.ok).map(|c| c.id.as_str()).collect();
        // En CI no hay Unreal instalado: solo puede fallar la comprobación del motor.
        assert!(failing.iter().all(|id| *id == "engine"), "{failing:?}");
        assert_eq!(report.mcp.port, 8123);
        assert!(super::plan(&path, &source(), Variant::Python, Some(8123)).unwrap().changes.is_empty(), "preparar dos veces no cambia nada");
        revert(&path, backups.path(), &id).unwrap();
        assert_eq!(snapshot(dir.path()), original, "revertir deja el proyecto byte a byte como estaba");
    }

    /// @spec REQ-05
    #[test]
    fn la_variante_cpp_instala_tambien_el_paquete_python() {
        let (_dir, path) = project();
        let plan = plan(&path, &source(), Variant::Cpp, None).unwrap();
        assert!(plan.changes.iter().any(|c| c.relative.starts_with("Plugins/DBVUnrealAICpp/Source/")));
        assert!(plan.changes.iter().any(|c| c.relative.starts_with("Plugins/DBVUnrealAI/Content/Python/")));
        let uproject = plan.changes.iter().find(|c| c.relative == "Demo.uproject").unwrap();
        assert!(uproject.after.contains("\"Name\": \"DBVUnrealAICpp\""));
    }

    /// @spec REQ-05, RNF-IA.3
    #[test]
    fn nunca_escribe_fuera_del_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        for bad in ["../x", "/etc/x", "a/../../x", "", "C:\\x"] {
            assert!(matches!(inside(dir.path(), bad), Err(UnrealError::Outside(_))) || cfg!(not(windows)) && bad == "C:\\x", "{bad}");
        }
        assert!(inside(dir.path(), "Plugins/DBVUnrealAI/x.py").is_ok());
    }

    /// Un fallo de escritura a mitad no deja el proyecto a medias (hallazgo Crítico de `/code-simplify`).
    /// @spec REQ-05
    #[test]
    fn si_una_escritura_falla_a_mitad_se_deshace_todo() {
        let (dir, path) = project();
        let backups = tempfile::tempdir().unwrap();
        let plan = plan(&path, &source(), Variant::Python, Some(8123)).unwrap();
        // Una carpeta donde va un fichero del plan: esa escritura fallará después de que otras hayan funcionado.
        let blocked = dir.path().join("Plugins").join("DBVUnrealAI").join("Content").join("Python").join("init_unreal.py");
        std::fs::create_dir_all(&blocked).unwrap();
        let before = snapshot(dir.path());
        let error = apply(&path, &plan, backups.path()).unwrap_err();
        assert!(error.to_string().contains("se deshizo sola"), "{error}");
        assert_eq!(snapshot(dir.path()), before, "el proyecto queda exactamente como estaba");
    }

    /// @spec REQ-05
    #[test]
    fn revertir_no_borra_lo_que_el_usuario_anadio_despues() {
        let (dir, path) = project();
        let backups = tempfile::tempdir().unwrap();
        let id = apply(&path, &plan(&path, &source(), Variant::Python, None).unwrap(), backups.path()).unwrap();
        let mine = dir.path().join("Plugins").join("DBVUnrealAI").join("mio.txt");
        std::fs::write(&mine, "mío").unwrap();
        revert(&path, backups.path(), &id).unwrap();
        assert!(mine.is_file());
        assert!(matches!(revert(&path, backups.path(), "no-existe"), Err(UnrealError::NotFound(_))));
    }

    #[test]
    fn interpreta_la_version_del_proyecto() {
        assert_eq!(parse_association("5.8"), Some((5, 8)));
        assert_eq!(parse_association("{0A1B}"), None);
        assert_eq!(Variant::parse(Some("CPP")), Variant::Cpp);
        assert_eq!(Variant::parse(None), Variant::Python);
    }
}
