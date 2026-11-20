// =============================================================================
// DBV Unreal AI — Descubrimiento de instalaciones, proyectos y editores
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-01, REQ-03, REQ-118
//
// Hechos de la máquina de desarrollo que condicionan este código (2026-10-24):
//   · `LauncherInstalled.dat` NO lista siempre el motor: en ella, la carpeta de
//     UE_5.8 solo aparecía como `QuixelBridge_5.8`. Por eso no se filtra por
//     `AppName`: vale cualquier `InstallLocation` con `Engine/Build/Build.version`,
//     más una búsqueda en las carpetas típicas de Epic.
//   · La línea de comandos del editor es `UnrealEditor.exe "<ruta>.uproject"`. Cuando el editor se reinicia solo
//     (al activar plugins), la ruta llega RELATIVA a su carpeta `Engine/Binaries/Win64`
//     (`"../../../../../../Programacion/Unreal/P/P.uproject"`, visto el 2026-10-31): se resuelve contra ella.
//   · El servidor MCP escucha solo en `127.0.0.1` (RNF-06).

use std::collections::HashSet;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::{ini, io_error, UnrealError};

/// Sección de `UModelContextProtocolSettings` en UE 5.8 (módulo `ModelContextProtocolEngine`).
pub const MCP_SECTION: &str = "/Script/ModelContextProtocolEngine.ModelContextProtocolSettings";
/// La misma en UE 5.9, que movió la clase al módulo `ModelContextProtocol` (visto en `PruebaUnreal59`, 2026-11-13).
/// @spec REQ-110
pub const MCP_SECTION_59: &str = "/Script/ModelContextProtocol.ModelContextProtocolSettings";

/// Sección de los ajustes del MCP que lee el motor `engine`: la de 5.9 desde la 5.9; la de 5.8 si no se sabe.
/// @spec REQ-110
pub fn mcp_section_for(engine: Option<&EngineInstall>) -> &'static str {
    let is_59 = engine.is_some_and(|e| (e.major, e.minor) >= (5, 9));
    if is_59 {
        MCP_SECTION_59
    } else {
        MCP_SECTION
    }
}
pub const DEFAULT_MCP_PORT: u16 = 8000;
pub const DEFAULT_MCP_PATH: &str = "/mcp";

/// Una instalación del motor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInstall {
    /// `5.8.2`.
    pub version: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub root: String,
    /// Ejecutable del editor (puede no existir si la instalación está incompleta).
    pub editor: String,
    pub editor_exists: bool,
}

/// Un plugin citado en el `.uproject`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub name: String,
    pub enabled: bool,
}

/// Lo que importa de un `.uproject`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UProject {
    pub path: String,
    pub name: String,
    /// `5.8`, o el GUID de una compilación desde código fuente, o vacío.
    pub engine_association: String,
    pub modules: Vec<String>,
    pub plugins: Vec<PluginEntry>,
    /// Tiene módulos C++ o carpeta `Source/`.
    pub has_cpp: bool,
    /// Versión del motor resuelto (`5.8.4`), también si la asociación es un GUID; vacía si no se encuentra o no se
    /// ha resuelto (solo la rellena `unreal_read_project`, que busca los motores). @spec REQ-118
    pub engine_version: String,
}

/// Ajustes del servidor MCP del proyecto (por usuario, con los valores por defecto de Epic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    pub port: u16,
    pub path: String,
    /// `None` si ningún `.ini` lo fija (Epic: desactivado).
    pub auto_start: Option<bool>,
}

/// Un editor de Unreal en marcha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningEditor {
    pub pid: u32,
    pub executable: String,
    pub project: Option<String>,
}

// ─── Instalaciones ───────────────────────────────────────────────────

/// Versión `(major, minor, patch)` de `Engine/Build/Build.version`.
pub fn read_build_version(root: &Path) -> Option<(u32, u32, u32)> {
    let text = std::fs::read_to_string(root.join("Engine").join("Build").join("Build.version")).ok()?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let field = |name: &str| value.get(name).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok());
    Some((field("MajorVersion")?, field("MinorVersion")?, field("PatchVersion").unwrap_or(0)))
}

/// `InstallLocation` de cada entrada de `LauncherInstalled.dat`.
pub fn parse_launcher_installed(text: &str) -> Vec<PathBuf> {
    let parsed: Option<Value> = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok();
    let list = parsed.as_ref().and_then(|v| v.get("InstallationList")).and_then(Value::as_array).cloned().unwrap_or_default();
    list.iter().filter_map(|entry| entry.get("InstallLocation").and_then(Value::as_str)).map(PathBuf::from).collect()
}

/// Ruta del ejecutable del editor dentro de una instalación, según la plataforma.
pub fn editor_executable(root: &Path) -> PathBuf {
    let binaries = root.join("Engine").join("Binaries");
    if cfg!(target_os = "windows") {
        binaries.join("Win64").join("UnrealEditor.exe")
    } else if cfg!(target_os = "macos") {
        binaries.join("Mac").join("UnrealEditor.app").join("Contents").join("MacOS").join("UnrealEditor")
    } else {
        binaries.join("Linux").join("UnrealEditor")
    }
}

/// Carpetas candidatas: lista del launcher de Epic + carpetas típicas `UE_*`.
fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut launcher_files = Vec::new();
    let mut parents = Vec::new();
    if cfg!(target_os = "windows") {
        let program_data = std::env::var_os("PROGRAMDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        launcher_files.push(program_data.join("Epic").join("UnrealEngineLauncher").join("LauncherInstalled.dat"));
        for letter in b'C'..=b'Z' {
            parents.push(PathBuf::from(format!("{}:\\Program Files\\Epic Games", letter as char)));
        }
    } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        launcher_files.push(home.join("Library/Application Support/Epic/UnrealEngineLauncher/LauncherInstalled.dat"));
        parents.push(PathBuf::from("/Users/Shared/Epic Games"));
        parents.push(home.join("UnrealEngine"));
    }
    // Motores registrados (GUID o nombre → carpeta): compilados desde código o registrados a mano (REQ-118).
    roots.extend(registered_builds().into_iter().map(|(_, path)| path));
    for file in launcher_files {
        if let Ok(text) = std::fs::read_to_string(&file) {
            roots.extend(parse_launcher_installed(&text));
        }
    }
    for parent in parents.iter().filter(|p| p.is_dir()) {
        if let Ok(entries) = std::fs::read_dir(parent) {
            roots.extend(entries.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("UE_"))));
        }
    }
    roots
}

/// Instalaciones del motor encontradas, sin duplicados, de la más nueva a la más antigua.
pub fn find_engines() -> Vec<EngineInstall> {
    engines_from(candidate_roots())
}

/// ¿Es `root` la instalación de Fortnite/UEFN? Trae su propio `Engine/Build/Build.version` (`MajorVersion: 6`), que no
/// es un motor de UE (REQ-122, P-14).
fn is_fortnite_install(root: &Path) -> bool {
    let text = std::fs::read_to_string(root.join("Engine").join("Build").join("Build.version")).unwrap_or_default();
    super::uefn::discovery::is_fortnite_build(&text)
}

/// Instalaciones válidas entre `roots` (las que tienen `Build.version`; la de Fortnite no cuenta).
pub fn engines_from(roots: Vec<PathBuf>) -> Vec<EngineInstall> {
    let mut seen = HashSet::new();
    let mut engines: Vec<EngineInstall> = Vec::new();
    for root in roots {
        let key = root.to_string_lossy().to_lowercase().replace('\\', "/").trim_end_matches('/').to_string();
        if !seen.insert(key) || is_fortnite_install(&root) {
            continue;
        }
        if let Some((major, minor, patch)) = read_build_version(&root) {
            let editor = editor_executable(&root);
            engines.push(EngineInstall {
                version: format!("{major}.{minor}.{patch}"),
                major,
                minor,
                patch,
                root: root.to_string_lossy().into_owned(),
                editor_exists: editor.is_file(),
                editor: editor.to_string_lossy().into_owned(),
            });
        }
    }
    engines.sort_by_key(|e| std::cmp::Reverse((e.major, e.minor, e.patch)));
    engines
}

/// La instalación que corresponde a `EngineAssociation` (`5.8` → la 5.8.x más nueva).
pub fn engine_for<'a>(engines: &'a [EngineInstall], association: &str) -> Option<&'a EngineInstall> {
    let wanted = association.trim();
    engines.iter().find(|engine| format!("{}.{}", engine.major, engine.minor) == wanted)
}

/// `[Installations]` de un `Install.ini` de Epic (Linux y macOS): `identificador=ruta`. @spec REQ-118
pub fn parse_install_ini(text: &str) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.eq_ignore_ascii_case("[Installations]");
        } else if let (true, Some((id, path))) = (in_section, trimmed.split_once('=')) {
            out.push((id.trim().to_string(), PathBuf::from(path.trim())));
        }
    }
    out
}

/// Motores registrados en el sistema: en Windows, los valores de `HKCU\Software\Epic Games\Unreal Engine\Builds`
/// (GUID o nombre → carpeta); en Linux y macOS, `Install.ini`. Solo se leen. @spec REQ-118
pub fn registered_builds() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    #[cfg(windows)]
    {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey_with_flags(r"Software\Epic Games\Unreal Engine\Builds", winreg::enums::KEY_READ) {
            for (name, _) in key.enum_values().flatten() {
                if let Ok(path) = key.get_value::<String, _>(&name) {
                    out.push((name, PathBuf::from(path)));
                }
            }
        }
    }
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for file in [home.join(".config/Epic/UnrealEngine/Install.ini"), home.join("Library/Application Support/Epic/UnrealEngine/Install.ini")] {
            if let Ok(text) = std::fs::read_to_string(file) {
                out.extend(parse_install_ini(&text));
            }
        }
    }
    out
}

/// Normaliza un identificador de motor registrado para compararlo: sin llaves, sin espacios a los lados, en minúsculas.
fn association_key(id: &str) -> String {
    id.trim().trim_start_matches('{').trim_end_matches('}').trim().to_ascii_lowercase()
}

/// La carpeta del motor que contiene `dir` (un proyecto con la asociación vacía vive dentro del árbol del motor).
pub fn enclosing_engine(dir: &Path) -> Option<PathBuf> {
    let found = dir.ancestors().find(|a| a.join("Engine").join("Build").join("Build.version").is_file()).map(Path::to_path_buf);
    found
}

/// De dónde salió el motor de un proyecto (para el diagnóstico, REQ-120).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineSource {
    /// Por el número de versión (`5.8`), entre los del Launcher y las carpetas típicas.
    Version,
    /// Un motor registrado (GUID o nombre) en el registro o en `Install.ini`.
    Registered,
    /// El árbol de motor que contiene el proyecto (asociación vacía).
    Enclosing,
    /// Elegido a mano por el usuario (REQ-119).
    Chosen,
}

/// El motor de un proyecto por su `EngineAssociation`, en el orden de REQ-118: número de versión, motor registrado
/// (GUID o nombre) y, si está vacía, el motor que contiene el proyecto. Pura: las fuentes del sistema se pasan.
/// @spec REQ-118
pub fn resolve_engine(association: &str, engines: &[EngineInstall], registered: &[(String, PathBuf)], project_dir: &Path) -> Option<(EngineInstall, EngineSource)> {
    let wanted = association_key(association);
    let by_version = engine_for(engines, association).cloned().map(|e| (e, EngineSource::Version));
    let by_registry = || {
        registered
            .iter()
            .find(|(id, _)| !wanted.is_empty() && association_key(id) == wanted)
            .and_then(|(_, path)| engines_from(vec![path.clone()]).into_iter().next())
            .map(|e| (e, EngineSource::Registered))
    };
    let by_enclosing = || {
        wanted
            .is_empty()
            .then(|| enclosing_engine(project_dir))
            .flatten()
            .and_then(|root| engines_from(vec![root]).into_iter().next())
            .map(|e| (e, EngineSource::Enclosing))
    };
    let found = by_version.or_else(by_registry).or_else(by_enclosing);
    found
}

/// El motor de un proyecto con las fuentes de este equipo (Launcher, carpetas típicas y motores registrados). @spec REQ-118
pub fn engine_of_project(project: &UProject) -> Option<(EngineInstall, EngineSource)> {
    let dir = Path::new(&project.path).parent().map(Path::to_path_buf).unwrap_or_default();
    // @spec REQ-119 Lo que el usuario eligió a mano va primero.
    let chosen = super::engine_choice::overrides_file()
        .and_then(|file| super::engine_choice::chosen_root(&file, Path::new(&project.path)))
        .and_then(|root| engines_from(vec![root]).into_iter().next())
        .map(|engine| (engine, EngineSource::Chosen));
    let found = chosen.or_else(|| resolve_engine(&project.engine_association, &find_engines(), &registered_builds(), &dir));
    found
}

/// Error de «no encuentro el motor» en palabras llanas, con qué puede hacer el usuario. @spec REQ-118, REQ-119
pub fn engine_not_found(project: &UProject) -> UnrealError {
    let error = UnrealError::NotFound(format!(
        "no encuentro el motor de Unreal de este proyecto: {}. Si tienes Unreal 5.8 o posterior instalado en otra carpeta, elígela con «Elegir la carpeta de Unreal…». Ten en cuenta que abrir el proyecto con otra versión lo convierte y ya no se puede volver atrás: si quieres probarlo con otra, hazlo con una copia",
        describe_association(&project.engine_association)
    ));
    error
}

/// Qué es una `EngineAssociation`, en palabras llanas, para los mensajes de error. @spec REQ-118, REQ-119
pub fn describe_association(association: &str) -> String {
    let trimmed = association.trim();
    let is_version = trimmed.split('.').count() == 2 && trimmed.split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    let is_guid = association_key(trimmed).len() == 32 + 4 && association_key(trimmed).chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let text = if trimmed.is_empty() {
        "el proyecto no dice con qué motor se abre (no está dentro de la carpeta de ningún motor)".to_string()
    } else if is_version {
        format!("el proyecto es de Unreal {trimmed} y esa versión no aparece instalada (ni en el Epic Games Launcher ni en las carpetas habituales)")
    } else if is_guid {
        format!("el proyecto está asociado a un motor registrado en otro equipo o compilado desde código ({trimmed}) que este equipo no tiene registrado")
    } else {
        format!("el proyecto está asociado a un motor con nombre «{trimmed}» que este equipo no tiene registrado")
    };
    text
}

// ─── Proyectos ───────────────────────────────────────────────────────

/// Lee un `.uproject`.
pub fn read_uproject(path: &Path) -> Result<UProject, UnrealError> {
    let text = std::fs::read_to_string(path).map_err(|error| io_error(path, &error))?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|error| UnrealError::Format(format!("{}: {error}", path.display())))?;
    let strings = |field: &str, name: &str| -> Vec<Value> { value.get(field).and_then(Value::as_array).cloned().unwrap_or_default().into_iter().filter(|v| v.get(name).is_some()).collect() };
    let modules: Vec<String> = strings("Modules", "Name").iter().filter_map(|m| m.get("Name").and_then(Value::as_str)).map(str::to_string).collect();
    let plugins = strings("Plugins", "Name")
        .iter()
        .filter_map(|p| Some(PluginEntry { name: p.get("Name")?.as_str()?.to_string(), enabled: p.get("Enabled").and_then(Value::as_bool).unwrap_or(false) }))
        .collect();
    let folder = path.parent().unwrap_or(Path::new("."));
    let result = UProject {
        path: path.to_string_lossy().into_owned(),
        name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        engine_association: value.get("EngineAssociation").and_then(Value::as_str).unwrap_or("").to_string(),
        has_cpp: !modules.is_empty() || folder.join("Source").is_dir(),
        modules,
        plugins,
        engine_version: String::new(),
    };
    Ok(result)
}

/// Nombre de la carpeta de configuración del editor de esta plataforma (`Saved/Config/<x>`).
pub fn platform_editor_dir() -> &'static str {
    if cfg!(target_os = "windows") {
        "WindowsEditor"
    } else if cfg!(target_os = "macos") {
        "MacEditor"
    } else {
        "LinuxEditor"
    }
}

/// `.ini` de ajustes del usuario para este proyecto (donde vive «Auto Start Server»).
pub fn user_settings_ini(project_dir: &Path) -> PathBuf {
    project_dir.join("Saved").join("Config").join(platform_editor_dir()).join("EditorPerProjectUserSettings.ini")
}

/// `.ini` por defecto del proyecto (versionado; lo pisa el del usuario).
pub fn default_settings_ini(project_dir: &Path) -> PathBuf {
    project_dir.join("Config").join("DefaultEditorPerProjectUserSettings.ini")
}

/// Ajustes MCP efectivos: los del usuario mandan sobre los del proyecto, y estos sobre los de Epic. En cada fichero
/// se leen la sección de 5.8 y la de 5.9 (gana la de 5.9): el `.ini` no dice qué motor lo escribió, y un proyecto
/// que pasó de 5.8 a 5.9 puede tener las dos. @spec REQ-02, REQ-110
pub fn read_mcp_settings(project_dir: &Path) -> McpSettings {
    let texts: Vec<String> = [default_settings_ini(project_dir), user_settings_ini(project_dir)].iter().filter_map(|p| std::fs::read_to_string(p).ok()).collect();
    let mut settings = McpSettings { port: DEFAULT_MCP_PORT, path: DEFAULT_MCP_PATH.to_string(), auto_start: None };
    for text in &texts {
        for section in [MCP_SECTION, MCP_SECTION_59] {
            if let Some(port) = ini::get(text, section, "ServerPortNumber").and_then(|v| v.parse().ok()) {
                settings.port = port;
            }
            if let Some(path) = ini::get(text, section, "ServerUrlPath").filter(|v| v.starts_with('/')) {
                settings.path = path;
            }
            if let Some(flag) = ini::get(text, section, "bAutoStartServer") {
                settings.auto_start = Some(flag.eq_ignore_ascii_case("true"));
            }
        }
    }
    settings
}

// ─── Editores en marcha ──────────────────────────────────────────────

/// El `.uproject` de una línea de comandos del editor, si la hay.
pub fn project_from_args<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter().skip(1).map(|a| a.as_ref().trim_matches('"')).find(|a| a.to_lowercase().ends_with(".uproject")).map(str::to_string)
}

/// Ruta absoluta del `.uproject` de un editor: una relativa se resuelve contra la carpeta de su ejecutable, no
/// contra la de la app (que no tiene por qué estar en la misma unidad). Los `..` se quitan sin tocar el disco.
pub fn absolute_project(project: &str, executable: &Path) -> String {
    let path = Path::new(project);
    let joined = match executable.parent() {
        Some(dir) if path.is_relative() => dir.join(path),
        _ => path.to_path_buf(),
    };
    let mut clean = PathBuf::new();
    for part in joined.components() {
        match part {
            std::path::Component::ParentDir => {
                clean.pop();
            }
            std::path::Component::CurDir => {}
            other => clean.push(other),
        }
    }
    clean.to_string_lossy().into_owned()
}

/// ¿Es el nombre de un proceso del editor de Unreal Engine? UEFN (`UnrealEditorFortnite…`) no lo es: su línea de
/// comandos lleva el código de acceso de Epic y no se puede leer nunca (REQ-122, P-14).
pub fn is_ue_editor_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("unrealeditor") && !lower.contains("fortnite")
}

/// Editores de Unreal abiertos ahora mismo. Primero se miran solo los nombres; la línea de comandos se lee después y
/// solo de los procesos de UE, para que la de UEFN no llegue a la memoria de la app.
pub fn running_editors() -> Vec<RunningEditor> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let pids: Vec<Pid> = system.processes().iter().filter(|(_, process)| is_ue_editor_process(&process.name().to_string_lossy())).map(|(pid, _)| *pid).collect();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_exe(UpdateKind::Always));
    let mut editors: Vec<RunningEditor> = system
        .processes()
        .values()
        .filter(|process| is_ue_editor_process(&process.name().to_string_lossy()))
        .map(|process| {
            let args: Vec<String> = process.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect();
            let executable = process.exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            let project = project_from_args(&args).map(|p| absolute_project(&p, Path::new(&executable)));
            RunningEditor { pid: process.pid().as_u32(), executable, project }
        })
        .collect();
    editors.sort_by_key(|editor| editor.pid);
    editors
}

/// ¿Está abierto el editor de este `.uproject`? (Windows no distingue mayúsculas en las rutas.)
pub fn editor_open_for(project_path: &Path) -> bool {
    let wanted = project_path.to_string_lossy().replace('\\', "/").to_lowercase();
    running_editors().into_iter().any(|editor| editor.project.is_some_and(|p| p.replace('\\', "/").to_lowercase() == wanted))
}

/// ¿Hay algo escuchando en `127.0.0.1:port`? Respuesta en ≤ 300 ms.
pub fn port_listening(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_engine(dir: &Path, version: (u32, u32, u32)) -> PathBuf {
        let build = dir.join("Engine").join("Build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("Build.version"), format!("{{\"MajorVersion\": {}, \"MinorVersion\": {}, \"PatchVersion\": {}}}", version.0, version.1, version.2)).unwrap();
        dir.to_path_buf()
    }

    /// @spec REQ-01
    #[test]
    fn lee_las_rutas_de_la_lista_del_launcher_aunque_no_digan_ue() {
        let text = "\u{feff}{\"InstallationList\":[{\"InstallLocation\":\"d:\\\\Program Files\\\\Epic Games\\\\UE_5.8\",\"AppName\":\"QuixelBridge_5.8\"},{\"AppName\":\"x\"}]}";
        assert_eq!(parse_launcher_installed(text), vec![PathBuf::from(r"d:\Program Files\Epic Games\UE_5.8")]);
        assert!(parse_launcher_installed("no es json").is_empty());
    }

    /// @spec REQ-118 GUID (con o sin llaves y en otra caja), nombre con espacios, versión y asociación vacía.
    #[test]
    fn resuelve_guid_nombre_version_y_asociacion_vacia() {
        let base = tempfile::tempdir().unwrap();
        let launcher = fake_engine(&base.path().join("UE_5.8"), (5, 8, 4));
        let source = fake_engine(&base.path().join("Source/UnrealEngine"), (5, 8, 4));
        let named = fake_engine(&base.path().join("Pixo"), (5, 9, 0));
        let engines = engines_from(vec![launcher.clone()]);
        let registered = vec![
            ("{95B798E8-4CE8-821F-3703-E085BBF457EF}".to_string(), source.clone()),
            ("Aximmetry DE 5.9".to_string(), named.clone()),
        ];
        let project = base.path().join("Proyectos/Demo");
        let root = |r: Option<(EngineInstall, EngineSource)>| r.map(|(e, s)| (PathBuf::from(e.root), s));

        assert_eq!(root(resolve_engine("5.8", &engines, &registered, &project)), Some((launcher.clone(), EngineSource::Version)));
        assert_eq!(root(resolve_engine("{95B798E8-4CE8-821F-3703-E085BBF457EF}", &engines, &registered, &project)), Some((source.clone(), EngineSource::Registered)));
        assert_eq!(root(resolve_engine("95b798e8-4ce8-821f-3703-e085bbf457ef", &engines, &registered, &project)), Some((source, EngineSource::Registered)), "sin llaves y en minúsculas");
        assert_eq!(root(resolve_engine("Aximmetry DE 5.9", &engines, &registered, &project)), Some((named, EngineSource::Registered)));
        assert!(resolve_engine("{00000000-0000-0000-0000-000000000000}", &engines, &registered, &project).is_none(), "un GUID desconocido no se adivina");
        assert!(resolve_engine("", &engines, &registered, &project).is_none(), "vacía y fuera de un motor");

        let inside = launcher.join("Samples/Demo");
        std::fs::create_dir_all(&inside).unwrap();
        assert_eq!(root(resolve_engine("", &engines, &registered, &inside)), Some((launcher, EngineSource::Enclosing)));
    }

    /// @spec REQ-118
    #[test]
    fn lee_install_ini_y_explica_cada_asociacion() {
        let text = "[Installations]\n{AB12}=/home/a/UnrealEngine\nMi Motor = /opt/ue \n[Otra]\nX=/no";
        assert_eq!(parse_install_ini(text), vec![("{AB12}".to_string(), PathBuf::from("/home/a/UnrealEngine")), ("Mi Motor".to_string(), PathBuf::from("/opt/ue"))]);
        assert!(describe_association("{95B798E8-4CE8-821F-3703-E085BBF457EF}").contains("compilado desde código"));
        assert!(describe_association("5.8").contains("Unreal 5.8"));
        assert!(describe_association("Pixotope_22.305").contains("«Pixotope_22.305»"));
        assert!(describe_association("").contains("no dice con qué motor"));
    }

    /// @spec REQ-01
    #[test]
    fn deduplica_y_ordena_las_instalaciones_por_version() {
        let base = tempfile::tempdir().unwrap();
        let old = fake_engine(&base.path().join("UE_5.7"), (5, 7, 4));
        let new = fake_engine(&base.path().join("UE_5.8"), (5, 8, 2));
        let found = engines_from(vec![old.clone(), new.clone(), new.clone(), base.path().join("nada")]);
        assert_eq!(found.iter().map(|e| e.version.as_str()).collect::<Vec<_>>(), vec!["5.8.2", "5.7.4"]);
        assert!(!found[0].editor_exists);
        assert_eq!(engine_for(&found, "5.8").map(|e| e.version.as_str()), Some("5.8.2"));
        assert!(engine_for(&found, "5.9").is_none());
    }

    /// @spec REQ-01
    #[test]
    fn lee_un_uproject_con_bom_modulos_y_plugins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Demo.uproject");
        std::fs::write(&path, "\u{feff}{\"EngineAssociation\":\"5.8\",\"Modules\":[{\"Name\":\"Demo\"}],\"Plugins\":[{\"Name\":\"ModelContextProtocol\",\"Enabled\":true},{\"Name\":\"Off\",\"Enabled\":false}]}").unwrap();
        let project = read_uproject(&path).unwrap();
        assert_eq!(project.name, "Demo");
        assert_eq!(project.engine_association, "5.8");
        assert!(project.has_cpp);
        assert_eq!(project.plugins, vec![PluginEntry { name: "ModelContextProtocol".into(), enabled: true }, PluginEntry { name: "Off".into(), enabled: false }]);
        assert!(matches!(read_uproject(&dir.path().join("x.uproject")), Err(UnrealError::Io(_))));
    }

    /// @spec REQ-01
    #[test]
    fn los_ajustes_del_usuario_mandan_sobre_los_del_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_mcp_settings(dir.path()), McpSettings { port: 8000, path: "/mcp".into(), auto_start: None });
        std::fs::create_dir_all(dir.path().join("Config")).unwrap();
        std::fs::write(default_settings_ini(dir.path()), format!("[{MCP_SECTION}]\nServerPortNumber=8100\nbAutoStartServer=False\n")).unwrap();
        let user = user_settings_ini(dir.path());
        std::fs::create_dir_all(user.parent().unwrap()).unwrap();
        std::fs::write(&user, format!("[{MCP_SECTION}]\nbAutoStartServer=True\n")).unwrap();
        assert_eq!(read_mcp_settings(dir.path()), McpSettings { port: 8100, path: "/mcp".into(), auto_start: Some(true) });
    }

    /// En UE 5.9 los ajustes van en otra sección (el módulo cambió de nombre): se lee también, y en un mismo fichero
    /// gana la de 5.9. La sección se elige por el motor. @spec REQ-110
    #[test]
    fn lee_los_ajustes_del_mcp_de_ue_5_9() {
        let dir = tempfile::tempdir().unwrap();
        let user = user_settings_ini(dir.path());
        std::fs::create_dir_all(user.parent().unwrap()).unwrap();
        std::fs::write(&user, format!("[{MCP_SECTION}]\nServerPortNumber=8100\n\n[{MCP_SECTION_59}]\nServerPortNumber=8002\nbAutoStartServer=True\n")).unwrap();
        assert_eq!(read_mcp_settings(dir.path()), McpSettings { port: 8002, path: "/mcp".into(), auto_start: Some(true) });
        let engine = |minor: u32| EngineInstall { version: format!("5.{minor}.0"), major: 5, minor, patch: 0, root: String::new(), editor: String::new(), editor_exists: false };
        assert_eq!(mcp_section_for(Some(&engine(8))), MCP_SECTION);
        assert_eq!(mcp_section_for(Some(&engine(9))), MCP_SECTION_59);
        assert_eq!(mcp_section_for(None), MCP_SECTION);
    }

    /// @spec REQ-01
    #[test]
    fn saca_el_uproject_de_la_linea_de_comandos_del_editor() {
        let args = ["d:/Program Files/Epic Games/UE_5.8/Engine/Binaries/Win64/UnrealEditor.exe", "\"D:\\Programacion\\Unreal\\DBVMCPFirst\\DBVMCPFirst.uproject\"", "-log"];
        assert_eq!(project_from_args(&args).as_deref(), Some(r"D:\Programacion\Unreal\DBVMCPFirst\DBVMCPFirst.uproject"));
        assert_eq!(project_from_args(&["UnrealEditor.exe"]), None);
    }

    /// Tras reiniciarse solo, el editor da la ruta relativa a `Engine/Binaries/Win64`: vale su carpeta, no la de la
    /// app. Una ruta absoluta no cambia. @spec REQ-01, REQ-38
    #[test]
    #[cfg(windows)]
    fn una_ruta_relativa_del_editor_se_resuelve_contra_su_ejecutable() {
        let exe = Path::new(r"d:\Program Files\Epic Games\UE_5.8\Engine\Binaries\Win64\UnrealEditor.exe");
        let relative = "../../../../../../Programacion/Unreal/DBVUnrealAITest1/DBVUnrealAITest1.uproject";
        assert_eq!(absolute_project(relative, exe), r"d:\Programacion\Unreal\DBVUnrealAITest1\DBVUnrealAITest1.uproject");
        assert_eq!(absolute_project(r"C:\P\Demo\Demo.uproject", exe), r"C:\P\Demo\Demo.uproject");
    }

    #[test]
    fn un_puerto_cerrado_no_escucha() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_listening(port));
        drop(listener);
    }

    /// @spec RNF-01
    #[test]
    fn el_ejecutable_y_la_carpeta_de_ajustes_dependen_de_la_plataforma() {
        let editor = editor_executable(Path::new("UE_5.8")).to_string_lossy().replace('\\', "/");
        let settings = user_settings_ini(Path::new("P")).to_string_lossy().replace('\\', "/");
        if cfg!(target_os = "windows") {
            assert!(editor.ends_with("Engine/Binaries/Win64/UnrealEditor.exe"), "{editor}");
            assert!(settings.ends_with("Saved/Config/WindowsEditor/EditorPerProjectUserSettings.ini"), "{settings}");
        } else if cfg!(target_os = "macos") {
            assert!(editor.ends_with("Engine/Binaries/Mac/UnrealEditor.app/Contents/MacOS/UnrealEditor"), "{editor}");
            assert!(settings.contains("MacEditor"));
        } else {
            assert!(editor.ends_with("Engine/Binaries/Linux/UnrealEditor"), "{editor}");
            assert!(settings.contains("LinuxEditor"));
        }
    }

    #[test]
    fn uefn_no_cuenta_como_editor_de_ue_y_no_se_lee_su_linea_de_comandos() {
        // @spec REQ-122 — la línea de comandos de UEFN lleva `-AUTH_PASSWORD`: el filtro va antes de leerla.
        assert!(is_ue_editor_process("UnrealEditor.exe"));
        assert!(is_ue_editor_process("UnrealEditor-Win64-DebugGame.exe"));
        assert!(!is_ue_editor_process("UnrealEditorFortnite-Win64-Shipping.exe"));
        assert!(!is_ue_editor_process("FortniteClient-Win64-Shipping.exe"));
    }

    #[test]
    fn la_instalacion_de_fortnite_no_es_un_motor_de_ue() {
        // @spec REQ-122 — Fortnite trae `Engine/Build/Build.version` con `MajorVersion: 6`.
        let temp = tempfile::tempdir().unwrap();
        let ue = fake_engine(&temp.path().join("UE_5.8"), (5, 8, 4));
        let fortnite = temp.path().join("Fortnite");
        let build = fortnite.join("Engine").join("Build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("Build.version"), r#"{"MajorVersion": 6, "MinorVersion": 0, "PatchVersion": 0, "BranchName": "++Fortnite+Release-43.12"}"#).unwrap();
        let engines = engines_from(vec![ue, fortnite]);
        assert_eq!(engines.iter().map(|e| e.version.as_str()).collect::<Vec<_>>(), vec!["5.8.4"]);
    }
}
