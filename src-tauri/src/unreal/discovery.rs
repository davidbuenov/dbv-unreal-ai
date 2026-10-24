// =============================================================================
// DBV Unreal AI — Descubrimiento de instalaciones, proyectos y editores
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-01, REQ-03
//
// Hechos de la máquina de desarrollo que condicionan este código (2026-10-24):
//   · `LauncherInstalled.dat` NO lista siempre el motor: en ella, la carpeta de
//     UE_5.8 solo aparecía como `QuixelBridge_5.8`. Por eso no se filtra por
//     `AppName`: vale cualquier `InstallLocation` con `Engine/Build/Build.version`,
//     más una búsqueda en las carpetas típicas de Epic.
//   · La línea de comandos del editor es `UnrealEditor.exe "<ruta>.uproject"`.
//   · El servidor MCP escucha solo en `127.0.0.1` (RNF-06).

use std::collections::HashSet;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::{ini, io_error, UnrealError};

/// Sección de `UModelContextProtocolSettings` (módulo `ModelContextProtocolEngine`).
pub const MCP_SECTION: &str = "/Script/ModelContextProtocolEngine.ModelContextProtocolSettings";
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
        // Linux: compilaciones registradas en `Install.ini` ([Installations] GUID=ruta).
        if let Ok(text) = std::fs::read_to_string(home.join(".config/Epic/UnrealEngine/Install.ini")) {
            let mut in_section = false;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') {
                    in_section = trimmed.eq_ignore_ascii_case("[Installations]");
                } else if let (true, Some((_, path))) = (in_section, trimmed.split_once('=')) {
                    roots.push(PathBuf::from(path.trim()));
                }
            }
        }
    }
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

/// Instalaciones válidas entre `roots` (las que tienen `Build.version`).
pub fn engines_from(roots: Vec<PathBuf>) -> Vec<EngineInstall> {
    let mut seen = HashSet::new();
    let mut engines: Vec<EngineInstall> = Vec::new();
    for root in roots {
        let key = root.to_string_lossy().to_lowercase().replace('\\', "/").trim_end_matches('/').to_string();
        if !seen.insert(key) {
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
    engines.sort_by(|a, b| (b.major, b.minor, b.patch).cmp(&(a.major, a.minor, a.patch)));
    engines
}

/// La instalación que corresponde a `EngineAssociation` (`5.8` → la 5.8.x más nueva).
pub fn engine_for<'a>(engines: &'a [EngineInstall], association: &str) -> Option<&'a EngineInstall> {
    let wanted = association.trim();
    engines.iter().find(|engine| format!("{}.{}", engine.major, engine.minor) == wanted)
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

/// Ajustes MCP efectivos: los del usuario mandan sobre los del proyecto, y estos sobre los de Epic.
pub fn read_mcp_settings(project_dir: &Path) -> McpSettings {
    let texts: Vec<String> = [default_settings_ini(project_dir), user_settings_ini(project_dir)].iter().filter_map(|p| std::fs::read_to_string(p).ok()).collect();
    let mut settings = McpSettings { port: DEFAULT_MCP_PORT, path: DEFAULT_MCP_PATH.to_string(), auto_start: None };
    for text in &texts {
        if let Some(port) = ini::get(text, MCP_SECTION, "ServerPortNumber").and_then(|v| v.parse().ok()) {
            settings.port = port;
        }
        if let Some(path) = ini::get(text, MCP_SECTION, "ServerUrlPath").filter(|v| v.starts_with('/')) {
            settings.path = path;
        }
        if let Some(flag) = ini::get(text, MCP_SECTION, "bAutoStartServer") {
            settings.auto_start = Some(flag.eq_ignore_ascii_case("true"));
        }
    }
    settings
}

// ─── Editores en marcha ──────────────────────────────────────────────

/// El `.uproject` de una línea de comandos del editor, si la hay.
pub fn project_from_args<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter().skip(1).map(|a| a.as_ref().trim_matches('"')).find(|a| a.to_lowercase().ends_with(".uproject")).map(str::to_string)
}

/// Editores de Unreal abiertos ahora mismo.
pub fn running_editors() -> Vec<RunningEditor> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_exe(UpdateKind::Always));
    let mut editors: Vec<RunningEditor> = system
        .processes()
        .values()
        .filter(|process| process.name().to_string_lossy().to_lowercase().starts_with("unrealeditor"))
        .map(|process| {
            let args: Vec<String> = process.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect();
            RunningEditor {
                pid: process.pid().as_u32(),
                executable: process.exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(),
                project: project_from_args(&args),
            }
        })
        .collect();
    editors.sort_by_key(|editor| editor.pid);
    editors
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

    /// @spec REQ-01
    #[test]
    fn saca_el_uproject_de_la_linea_de_comandos_del_editor() {
        let args = ["d:/Program Files/Epic Games/UE_5.8/Engine/Binaries/Win64/UnrealEditor.exe", "\"D:\\Programacion\\Unreal\\DBVMCPFirst\\DBVMCPFirst.uproject\"", "-log"];
        assert_eq!(project_from_args(&args).as_deref(), Some(r"D:\Programacion\Unreal\DBVMCPFirst\DBVMCPFirst.uproject"));
        assert_eq!(project_from_args(&["UnrealEditor.exe"]), None);
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
}
