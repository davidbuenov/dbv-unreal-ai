// =============================================================================
// DBV Unreal AI — Detección de UEFN, su versión y sus proyectos
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-122
//
// Lo que se vio en esta máquina (spike P-14, 2026-11-20):
//   · Versión: el `Build.version` de la instalación es la fuente exacta (`BranchName: ++Fortnite+Release-43.12`,
//     `Changelist`). `LauncherInstalled.dat` puede ir por detrás tras un parche (decía 43.10 con el editor en 43.12),
//     y el log del editor (`LogInit: Build:`) solo existe si UEFN se abrió alguna vez. Gana el changelist más alto.
//   · La entrada `Fortnite` del Launcher es el juego; la de UEFN es `Fortnite_Studio`.
//   · Proyecto abierto: `LastProjectFileName` del `.ini` del usuario. NUNCA la línea de comandos del proceso: lleva el
//     código de acceso de Epic (`-AUTH_PASSWORD`), el usuario y su id.
//   · Todo se lee; no se escribe nada (ni el registro de Epic, ni el `.ini`, ni los proyectos).

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::unreal::{io_error, UnrealError};

const INI_SECTION: &str = "/Script/ValkyrieEditor.ValkyrieEditorConfig";
const STUDIO_APP: &str = "Fortnite_Studio";

/// Versión de UEFN (`43.12`) con su changelist, y de dónde salió.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnVersion {
    pub version: String,
    pub changelist: u64,
    /// `build` (Build.version), `launcher` o `log`.
    pub source: String,
}

/// UEFN instalado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnInstall {
    pub root: String,
    pub version: UefnVersion,
}

/// Un proyecto UEFN (`<Nombre>.uefnproject` + `.uplugin`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnProject {
    pub title: String,
    /// Ruta del `.uefnproject`.
    pub path: String,
    pub dir: String,
    pub project_id: String,
    /// `projectVersePath`; vacío = aún sin vincular a una cuenta.
    pub verse_path: String,
    pub linked: bool,
    pub modules: Vec<String>,
    pub compatibility_version: String,
    /// Nombre del `.uplugin` raíz (puede no coincidir con el del proyecto).
    pub plugin_name: String,
}

// ─── Versión ────────────────────────────────────────────────────────

/// `43.12` y su changelist a partir de `++Fortnite+Release-43.12-CL-60598086[-Windows]` o de
/// `++Fortnite+Release-43.12` más un changelist aparte.
pub fn parse_release(text: &str) -> Option<(String, u64)> {
    let rest = text.trim().split("Release-").nth(1)?;
    let mut parts = rest.split('-');
    let version = parts.next().filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'))?.to_string();
    let changelist = match (parts.next(), parts.next()) {
        (Some("CL"), Some(cl)) => cl.parse().unwrap_or(0),
        _ => 0,
    };
    Some((version, changelist))
}

/// Versión desde `Engine/Build/Build.version` de la instalación de Fortnite.
pub fn version_from_build_file(text: &str) -> Option<UefnVersion> {
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let branch = value.get("BranchName").and_then(Value::as_str)?;
    let (version, _) = parse_release(branch)?;
    let changelist = value.get("Changelist").and_then(Value::as_u64).unwrap_or(0);
    Some(UefnVersion { version, changelist, source: "build".into() })
}

/// Carpeta de instalación y versión de la entrada `Fortnite_Studio` de `LauncherInstalled.dat` (la entrada
/// `Fortnite` es el juego y se ignora).
pub fn studio_from_launcher(text: &str) -> Option<(PathBuf, UefnVersion)> {
    let parsed: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let entry = parsed.get("InstallationList")?.as_array()?.iter().find(|e| e.get("AppName").and_then(Value::as_str) == Some(STUDIO_APP))?;
    let root = PathBuf::from(entry.get("InstallLocation").and_then(Value::as_str)?);
    let (version, changelist) = parse_release(entry.get("AppVersion").and_then(Value::as_str)?)?;
    Some((root, UefnVersion { version, changelist, source: "launcher".into() }))
}

/// Versión de la línea `LogInit: Build: ++Fortnite+Release-43.12-CL-60598086` del log del editor.
pub fn version_from_log(text: &str) -> Option<UefnVersion> {
    let line = text.lines().take(5000).find(|l| l.contains("LogInit: Build:"))?;
    let (version, changelist) = parse_release(line.split("Build:").nth(1)?)?;
    Some(UefnVersion { version, changelist, source: "log".into() })
}

/// La versión más nueva (changelist más alto) de las encontradas; a igualdad, la primera (la de más confianza).
pub fn newest(candidates: Vec<UefnVersion>) -> Option<UefnVersion> {
    let mut best: Option<UefnVersion> = None;
    for candidate in candidates {
        if best.as_ref().is_none_or(|b| candidate.changelist > b.changelist) {
            best = Some(candidate);
        }
    }
    best
}

/// ¿Es `Build.version` de una instalación de Fortnite (y no de un motor de UE)? Fortnite trae su propio `Engine/`
/// con `MajorVersion: 6`, que sin esto se tomaría por un «UE 6.0».
pub fn is_fortnite_build(build_version_text: &str) -> bool {
    let branch = serde_json::from_str::<Value>(build_version_text.trim_start_matches('\u{feff}'))
        .ok()
        .and_then(|v| v.get("BranchName").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    branch.contains("Fortnite")
}

// ─── Rutas del sistema ──────────────────────────────────────────────

/// `%LOCALAPPDATA%\UnrealEditorFortnite\Saved`.
pub fn saved_dir() -> Option<PathBuf> {
    let dir = dirs::data_local_dir().map(|d| d.join("UnrealEditorFortnite").join("Saved"));
    dir
}

/// `.ini` del usuario de UEFN.
pub fn user_ini() -> Option<PathBuf> {
    let file = saved_dir().map(|s| s.join("Config").join("WindowsEditor").join("EditorPerProjectUserSettings.ini"));
    file
}

/// Log del editor de UEFN.
pub fn editor_log() -> Option<PathBuf> {
    let file = saved_dir().map(|s| s.join("Logs").join("UnrealEditorFortnite.log"));
    file
}

fn launcher_file() -> PathBuf {
    let program_data = std::env::var_os("PROGRAMDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    program_data.join("Epic").join("UnrealEngineLauncher").join("LauncherInstalled.dat")
}

/// UEFN instalado en esta máquina, con la versión más fiable. `None` fuera de Windows o si no está.
pub fn find_uefn() -> Option<UefnInstall> {
    if !cfg!(target_os = "windows") {
        return None;
    }
    let read = |path: &Path| std::fs::read_to_string(path).ok();
    let launcher = read(&launcher_file()).and_then(|text| studio_from_launcher(&text));
    let log_version = editor_log().and_then(|p| read(&p)).and_then(|text| version_from_log(&text));
    let found = launcher.and_then(|(root, launcher_version)| {
        let build = read(&root.join("Engine").join("Build").join("Build.version")).and_then(|text| version_from_build_file(&text));
        let candidates: Vec<UefnVersion> = [build, Some(launcher_version), log_version].into_iter().flatten().collect();
        newest(candidates).map(|version| UefnInstall { root: root.to_string_lossy().into_owned(), version })
    });
    found
}

// ─── Proyectos ──────────────────────────────────────────────────────

/// Valores de una clave del `.ini` en una sección, admitiendo `Clave=` y `+Clave=` (listas).
fn ini_values(text: &str, section: &str, key: &str) -> Vec<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    let mut values = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line.eq_ignore_ascii_case(&header);
        } else if inside {
            if let Some((name, value)) = line.split_once('=') {
                if name.trim_start_matches('+').eq_ignore_ascii_case(key) && !value.trim().is_empty() {
                    values.push(value.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    values
}

/// Carpetas donde buscar proyectos: `LastCreatedProjectLocation` y `AdditionalProjectPaths`; si no hay ninguna,
/// `Documentos\Fortnite Projects`.
pub fn project_folders(ini_text: &str, documents: Option<&Path>) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = ["LastCreatedProjectLocation", "AdditionalProjectPaths"]
        .iter()
        .flat_map(|key| ini_values(ini_text, INI_SECTION, key))
        .map(PathBuf::from)
        .collect();
    if folders.is_empty() {
        folders.extend(documents.map(|d| d.join("Fortnite Projects")));
    }
    folders.dedup();
    folders
}

/// El último proyecto abierto en UEFN (`LastProjectFileName`). Con UEFN en marcha, es el que tiene abierto.
pub fn last_project_file(ini_text: &str) -> Option<PathBuf> {
    let file = ini_values(ini_text, INI_SECTION, "LastProjectFileName").into_iter().next().map(PathBuf::from);
    file
}

/// Lee un `.uefnproject` y el nombre del `.uplugin` de su carpeta.
pub fn read_uefnproject(path: &Path) -> Result<UefnProject, UnrealError> {
    let text = std::fs::read_to_string(path).map_err(|error| io_error(path, &error))?;
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|error| UnrealError::Format(format!("{}: {error}", path.display())))?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let text_of = |pointer: &str| value.pointer(pointer).and_then(Value::as_str).unwrap_or("").to_string();
    let modules = value.pointer("/bindings/modules").and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    let plugin_name = std::fs::read_dir(dir)
        .ok()
        .and_then(|entries| entries.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("uplugin"))))
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let title = Some(text_of("/title")).filter(|t| !t.is_empty()).unwrap_or(stem);
    let verse_path = text_of("/bindings/projectVersePath");
    let project = UefnProject {
        title,
        path: path.to_string_lossy().into_owned(),
        dir: dir.to_string_lossy().into_owned(),
        project_id: text_of("/bindings/projectId"),
        linked: !verse_path.is_empty(),
        verse_path,
        modules,
        compatibility_version: text_of("/compatibilityVersion"),
        plugin_name,
    };
    Ok(project)
}

fn uefnproject_in(dir: &Path) -> Option<PathBuf> {
    let found = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("uefnproject")));
    found
}

/// Proyectos UEFN en `folders` (la propia carpeta o sus subcarpetas directas). Los ilegibles se saltan.
pub fn projects_in(folders: &[PathBuf]) -> Vec<UefnProject> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for folder in folders.iter().filter(|f| f.is_dir()) {
        dirs.push(folder.clone());
        if let Ok(entries) = std::fs::read_dir(folder) {
            dirs.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
        }
    }
    let mut projects: Vec<UefnProject> = dirs.iter().filter_map(|d| uefnproject_in(d)).filter_map(|p| read_uefnproject(&p).ok()).collect();
    projects.sort_by_key(|p| p.title.to_lowercase());
    projects.dedup_by(|a, b| a.path.eq_ignore_ascii_case(&b.path));
    projects
}

/// Proyectos UEFN de esta máquina, según el `.ini` del usuario.
pub fn find_projects() -> Vec<UefnProject> {
    let ini_text = user_ini().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let documents = dirs::document_dir();
    projects_in(&project_folders(&ini_text, documents.as_deref()))
}

/// ¿Es más nueva la versión del proyecto (`43.20`) que la instalada (`43.12`)? Compara número a número.
pub fn project_newer_than(project_version: &str, installed: &str) -> bool {
    let parse = |v: &str| v.split('.').map(|n| n.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    let newer = !project_version.is_empty() && parse(project_version) > parse(installed);
    newer
}

// ─── Editor en marcha ───────────────────────────────────────────────

/// ¿Es el nombre de un proceso del editor de UEFN? (`UnrealEditorFortnite-Win64-Shipping.exe`).
pub fn is_uefn_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("unrealeditorfortnite")
}

/// ¿Hay un editor de UEFN abierto? Solo mira nombres de proceso: nunca su línea de comandos.
pub fn uefn_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let running = system.processes().values().any(|p| is_uefn_process(&p.name().to_string_lossy()));
    running
}

/// El proyecto que tiene abierto UEFN ahora, si hay editor y el `.ini` lo dice.
pub fn open_project() -> Option<UefnProject> {
    if !uefn_running() {
        return None;
    }
    let ini_text = user_ini().and_then(|p| std::fs::read_to_string(p).ok())?;
    let project = last_project_file(&ini_text).and_then(|p| read_uefnproject(&p).ok());
    project
}

// ─── Resumen para la interfaz ───────────────────────────────────────

/// Un proyecto con lo que la lista necesita mostrar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnProjectStatus {
    #[serde(flatten)]
    pub project: UefnProject,
    /// Es el que tiene abierto UEFN ahora.
    pub open: bool,
    /// Pide una UEFN más nueva que la instalada.
    pub newer_than_installed: bool,
}

/// Lo que la vista Proyecto sabe de UEFN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnOverview {
    pub install: Option<UefnInstall>,
    pub running: bool,
    pub projects: Vec<UefnProjectStatus>,
}

/// Cruza instalación, proyectos y proyecto abierto. Función pura para poder probarla sin UEFN.
pub fn build_overview(install: Option<UefnInstall>, running: bool, projects: Vec<UefnProject>, open_file: Option<&Path>) -> UefnOverview {
    let installed = install.as_ref().map(|i| i.version.version.clone()).unwrap_or_default();
    let open_key = open_file.filter(|_| running).map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase());
    let projects = projects
        .into_iter()
        .map(|project| {
            let open = open_key.as_deref() == Some(project.path.replace('\\', "/").to_lowercase().as_str());
            let newer_than_installed = !installed.is_empty() && project_newer_than(&project.compatibility_version, &installed);
            UefnProjectStatus { project, open, newer_than_installed }
        })
        .collect();
    UefnOverview { install, running, projects }
}

/// Resumen de UEFN en esta máquina. Sin UEFN instalado (o fuera de Windows) no hay proyectos que mostrar.
pub fn overview() -> UefnOverview {
    let install = find_uefn();
    let ini_text = user_ini().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let projects = if install.is_some() { find_projects() } else { Vec::new() };
    let running = install.is_some() && uefn_running();
    let open_file = last_project_file(&ini_text);
    build_overview(install, running, projects, open_file.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAUNCHER: &str = r#"{"InstallationList": [
        {"InstallLocation": "C:\\Program Files\\Epic Games\\Fortnite", "AppVersion": "++Fortnite+Release-43.10-CL-60118918-Windows", "AppName": "Fortnite"},
        {"InstallLocation": "C:\\Program Files\\Epic Games\\Fortnite", "AppVersion": "++Fortnite+Release-43.10-CL-60458657-Windows", "AppName": "Fortnite_Studio"},
        {"InstallLocation": "D:\\Epic\\UE_5.8", "AppVersion": "5.8.4-1+++UE5+Release-5.8", "AppName": "UE_5.8"}
    ]}"#;

    const BUILD_VERSION: &str = r#"{"MajorVersion": 6, "MinorVersion": 0, "PatchVersion": 0, "Changelist": 60598086,
        "BranchName": "++Fortnite+Release-43.12"}"#;

    #[test]
    fn lee_la_version_de_fortnite_studio_y_no_la_del_juego() {
        // @spec REQ-122
        let (root, version) = studio_from_launcher(LAUNCHER).unwrap();
        assert_eq!(root, PathBuf::from(r"C:\Program Files\Epic Games\Fortnite"));
        assert_eq!(version, UefnVersion { version: "43.10".into(), changelist: 60458657, source: "launcher".into() });
    }

    #[test]
    fn gana_la_version_con_el_changelist_mas_alto() {
        // @spec REQ-122 — el Launcher iba por detrás (43.10) del editor parcheado (43.12).
        let build = version_from_build_file(BUILD_VERSION).unwrap();
        let (_, launcher) = studio_from_launcher(LAUNCHER).unwrap();
        let log = version_from_log("LogInit: Display: x\nLogInit: Build: ++Fortnite+Release-43.12-CL-60598086\n").unwrap();
        assert_eq!(log.version, "43.12");
        let best = newest(vec![build.clone(), launcher, log]).unwrap();
        assert_eq!(best, build);
        assert_eq!(best.version, "43.12");
    }

    #[test]
    fn sin_entrada_del_launcher_no_hay_uefn() {
        // @spec REQ-122
        assert!(studio_from_launcher(r#"{"InstallationList": [{"AppName": "Fortnite", "InstallLocation": "C:\\F", "AppVersion": "++Fortnite+Release-43.10-CL-1"}]}"#).is_none());
        assert!(studio_from_launcher("no es json").is_none());
    }

    #[test]
    fn el_build_version_de_fortnite_no_es_un_motor_de_ue() {
        // @spec REQ-122
        assert!(is_fortnite_build(BUILD_VERSION));
        assert!(!is_fortnite_build(r#"{"MajorVersion": 5, "MinorVersion": 8, "BranchName": "++UE5+Release-5.8"}"#));
    }

    #[test]
    fn carpetas_de_proyectos_del_ini_con_barras_normales() {
        // @spec REQ-122
        let ini = "[Otra]\nLastCreatedProjectLocation=C:/no\n[/Script/ValkyrieEditor.ValkyrieEditorConfig]\nLastCreatedProjectLocation=D:/Programacion/UEFN-Fornite\n+AdditionalProjectPaths=E:/Mas Proyectos\nLastProjectFileName=D:/Programacion/UEFN-Fornite/P/P.uefnproject\n";
        assert_eq!(project_folders(ini, None), vec![PathBuf::from("D:/Programacion/UEFN-Fornite"), PathBuf::from("E:/Mas Proyectos")]);
        assert_eq!(last_project_file(ini), Some(PathBuf::from("D:/Programacion/UEFN-Fornite/P/P.uefnproject")));
        assert_eq!(project_folders("", Some(Path::new("C:/Docs"))), vec![PathBuf::from("C:/Docs/Fortnite Projects")]);
    }

    fn write_project(root: &Path, folder: &str, plugin: &str, verse_path: &str) -> PathBuf {
        let dir = root.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{plugin}.uplugin")), "{}").unwrap();
        let body = format!(
            r#"{{"title": "{folder}", "bindings": {{"projectId": "34c4-id", "modules": {{"{plugin}": "{plugin}"}}, "projectVersePath": "{verse_path}"}}, "compatibilityVersion": "43.10"}}"#
        );
        let file = dir.join(format!("{folder}.uefnproject"));
        std::fs::write(&file, body).unwrap();
        file
    }

    #[test]
    fn lee_proyectos_vinculados_y_sin_vincular_con_otro_nombre_de_plugin() {
        // @spec REQ-122
        let temp = tempfile::tempdir().unwrap();
        write_project(temp.path(), "DBVFornite", "DBVFornite", "/usuario@fortnite.com/DBVFornite");
        write_project(temp.path(), "DBVUnrealAITest", "ca8492f4-e5f2-4e71-8517-ab9056326ef8", "");
        std::fs::create_dir_all(temp.path().join("NoEsProyecto")).unwrap();
        let projects = projects_in(&[temp.path().to_path_buf()]);
        assert_eq!(projects.len(), 2);
        let linked = &projects[0];
        assert_eq!((linked.title.as_str(), linked.linked), ("DBVFornite", true));
        let test = &projects[1];
        assert_eq!((test.title.as_str(), test.linked, test.plugin_name.as_str()), ("DBVUnrealAITest", false, "ca8492f4-e5f2-4e71-8517-ab9056326ef8"));
        assert_eq!(test.modules, vec!["ca8492f4-e5f2-4e71-8517-ab9056326ef8".to_string()]);
        assert_eq!(test.compatibility_version, "43.10");
    }

    #[test]
    fn avisa_si_el_proyecto_es_de_una_version_mas_nueva() {
        // @spec REQ-122
        assert!(project_newer_than("43.20", "43.12"));
        assert!(!project_newer_than("43.10", "43.12"));
        assert!(!project_newer_than("", "43.12"));
        assert!(project_newer_than("44.0", "43.12"));
    }

    #[test]
    fn el_resumen_marca_el_proyecto_abierto_y_el_de_version_mas_nueva() {
        // @spec REQ-122
        let temp = tempfile::tempdir().unwrap();
        let open_file = write_project(temp.path(), "Abierto", "Abierto", "");
        let newer_file = write_project(temp.path(), "Nuevo", "Nuevo", "");
        let mut newer = read_uefnproject(&newer_file).unwrap();
        newer.compatibility_version = "43.20".into();
        let open = read_uefnproject(&open_file).unwrap();
        let install = UefnInstall { root: "C:/F".into(), version: UefnVersion { version: "43.12".into(), changelist: 1, source: "build".into() } };
        let overview = build_overview(Some(install.clone()), true, vec![open.clone(), newer.clone()], Some(&open_file));
        assert_eq!(overview.projects.iter().map(|p| (p.open, p.newer_than_installed)).collect::<Vec<_>>(), vec![(true, false), (false, true)]);
        // Con UEFN cerrado, `LastProjectFileName` no dice qué está abierto.
        let closed = build_overview(Some(install), false, vec![open], Some(&open_file));
        assert!(!closed.projects[0].open);
    }

    #[test]
    fn reconoce_el_proceso_de_uefn() {
        // @spec REQ-122
        assert!(is_uefn_process("UnrealEditorFortnite-Win64-Shipping.exe"));
        assert!(!is_uefn_process("UnrealEditor.exe"));
        assert!(!is_uefn_process("FortniteClient-Win64-Shipping.exe"));
    }
}
