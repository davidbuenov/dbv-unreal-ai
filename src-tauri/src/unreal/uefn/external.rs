// =============================================================================
// DBV Unreal AI — Repositorios de Epic para UEFN: la skill de Verse
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-124
//
// La skill de Verse es trabajo de Epic Games (Magnus Enebakk) y su repositorio no tiene licencia: la app NO la incluye
// ni la copia a su repositorio. Se descarga en la máquina del usuario (decisión 2 del usuario, 2026-11-19):
//   · con `git clone --depth 1` si hay git (así «Actualizar» es un `git pull`);
//   · si no, con el zip de GitHub, que se descomprime con el `tar.exe` de Windows (UEFN solo existe en Windows) y deja
//     un `.dbv-skill.json` con el commit y la fecha.
// Destino: `<proyecto>/.claude/skills/verse` (por defecto) o `~/.claude/skills/verse`. Claude Code la usa sola; el resto
// de IAs llega a ella con `verse_reference` (REQ-125).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::unreal::{io_error, UnrealError};

pub const SKILL_REPO: &str = "https://github.com/magnusenebakk-epic/verse-skill";
const SKILL_ZIP: &str = "https://codeload.github.com/magnusenebakk-epic/verse-skill/zip/refs/heads/main";
const SKILL_API_HEAD: &str = "https://api.github.com/repos/magnusenebakk-epic/verse-skill/commits/HEAD";
const VERSION_FILE: &str = ".dbv-skill.json";
/// Como mucho una comprobación de versión nueva al día (REQ-124).
pub const CHECK_EVERY_SECS: u64 = 24 * 60 * 60;

/// Dónde se instala la skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillScope {
    /// `<proyecto>/.claude/skills/verse`.
    Project,
    /// `~/.claude/skills/verse`.
    User,
}

/// La skill instalada, con su versión para la atribución.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    pub path: String,
    pub scope: SkillScope,
    /// Commit corto (`abc1234`), vacío si no se sabe.
    pub commit: String,
    /// Fecha del commit (`2026-10-14`), vacía si no se sabe.
    pub date: String,
    pub via_git: bool,
    pub repository: String,
}

/// Carpeta de la skill para un ámbito.
pub fn skill_dir(scope: SkillScope, project_dir: &Path, home: &Path) -> PathBuf {
    let base = match scope {
        SkillScope::Project => project_dir.to_path_buf(),
        SkillScope::User => home.to_path_buf(),
    };
    base.join(".claude").join("skills").join("verse")
}

/// Proceso sin ventana de consola, con la salida capturada.
fn quiet(program: &Path) -> Command {
    let mut command = Command::new(program);
    command.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Ejecuta git y devuelve su salida, o el error que dio.
fn git(git: &Path, args: &[&str], cwd: Option<&Path>) -> Result<String, UnrealError> {
    let mut command = quiet(git);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let output = command.output().map_err(|error| UnrealError::Io(format!("git: {error}")))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let result = if output.status.success() { Ok(text) } else { Err(UnrealError::Protocol(format!("git {}: {}", args.first().unwrap_or(&""), String::from_utf8_lossy(&output.stderr).trim()))) };
    result
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VersionFile {
    commit: String,
    date: String,
}

/// Commit y fecha de una skill instalada: de git si es un clon, si no del `.dbv-skill.json` del zip.
fn read_version(path: &Path, git_program: Option<&Path>) -> (String, String, bool) {
    let via_git = path.join(".git").exists();
    let from_git = git_program.filter(|_| via_git).and_then(|g| git(g, &["log", "-1", "--format=%h %cs"], Some(path)).ok());
    let version = match from_git {
        Some(line) => {
            let (commit, date) = line.split_once(' ').unwrap_or((line.as_str(), ""));
            (commit.to_string(), date.to_string(), true)
        }
        None => {
            let file: VersionFile = std::fs::read_to_string(path.join(VERSION_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            (file.commit, file.date, via_git)
        }
    };
    version
}

/// La skill instalada en el proyecto o, si no, en la carpeta del usuario. Solo cuenta si tiene `SKILL.md`.
pub fn detect(project_dir: Option<&Path>, home: &Path, git_program: Option<&Path>) -> Option<InstalledSkill> {
    let candidates = [project_dir.map(|p| (SkillScope::Project, skill_dir(SkillScope::Project, p, home))), Some((SkillScope::User, skill_dir(SkillScope::User, Path::new(""), home)))];
    let found = candidates.into_iter().flatten().find(|(_, dir)| dir.join("SKILL.md").is_file()).map(|(scope, dir)| {
        let (commit, date, via_git) = read_version(&dir, git_program);
        InstalledSkill { path: dir.to_string_lossy().into_owned(), scope, commit, date, via_git, repository: SKILL_REPO.to_string() }
    });
    found
}

/// Último commit del repositorio (`sha` completo y fecha), con git o con la API de GitHub.
pub fn remote_head(repo: &str, git_program: Option<&Path>) -> Option<(String, String)> {
    let head = match git_program {
        Some(g) => git(g, &["ls-remote", repo, "HEAD"], None).ok().and_then(|out| out.split_whitespace().next().map(|sha| (sha.to_string(), String::new()))),
        None => {
            let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().new_agent();
            let body: Option<Value> = agent.get(SKILL_API_HEAD).header("User-Agent", "dbv-unreal-ai").call().ok().and_then(|mut r| r.body_mut().read_json().ok());
            body.and_then(|v| Some((v.get("sha")?.as_str()?.to_string(), v.pointer("/commit/committer/date").and_then(Value::as_str).unwrap_or("").chars().take(10).collect())))
        }
    };
    head
}

/// ¿Hay commits nuevos? Compara el commit corto instalado con el principio del remoto.
pub fn has_update(installed: &InstalledSkill, remote_sha: &str) -> bool {
    !installed.commit.is_empty() && !remote_sha.is_empty() && !remote_sha.starts_with(&installed.commit)
}

/// ¿Toca volver a comprobar? Como mucho una vez cada `CHECK_EVERY_SECS`.
pub fn should_check(last_check: Option<u64>, now: u64) -> bool {
    last_check.is_none_or(|last| now.saturating_sub(last) >= CHECK_EVERY_SECS)
}

/// Descomprime el zip de GitHub en `target`: el zip trae una carpeta `verse-skill-main/` que se quita.
pub fn extract_zip(zip: &Path, target: &Path, commit: &str, date: &str) -> Result<(), UnrealError> {
    let staging = target.with_extension("dbv-unzip");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|error| io_error(&staging, &error))?;
    let tar = std::env::var_os("SystemRoot").map(|root| PathBuf::from(root).join("System32").join("tar.exe")).filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("tar"));
    let output = quiet(&tar).arg("-xf").arg(zip).arg("-C").arg(&staging).output().map_err(|error| UnrealError::Io(format!("tar: {error}")))?;
    output.status.success().then_some(()).ok_or_else(|| UnrealError::Format(format!("the zip could not be extracted: {}", String::from_utf8_lossy(&output.stderr).trim())))?;
    let inner = std::fs::read_dir(&staging).map_err(|error| io_error(&staging, &error))?.flatten().map(|e| e.path()).find(|p| p.is_dir()).unwrap_or(staging.clone());
    inner.join("SKILL.md").is_file().then_some(()).ok_or_else(|| UnrealError::Format("the download does not look like the Verse skill (no SKILL.md)".into()))?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
    }
    let _ = std::fs::remove_dir_all(target);
    std::fs::rename(&inner, target).map_err(|error| io_error(target, &error))?;
    let _ = std::fs::remove_dir_all(&staging);
    let version = serde_json::to_string_pretty(&VersionFile { commit: commit.chars().take(7).collect(), date: date.to_string() }).unwrap_or_default();
    let result = std::fs::write(target.join(VERSION_FILE), version).map_err(|error| io_error(target, &error));
    result
}

fn download_zip(to: &Path) -> Result<(), UnrealError> {
    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().new_agent();
    let mut response = agent.get(SKILL_ZIP).header("User-Agent", "dbv-unreal-ai").call().map_err(|error| UnrealError::Unreachable(format!("GitHub: {error}")))?;
    let bytes = response.body_mut().with_config().limit(50 * 1024 * 1024).read_to_vec().map_err(|error| UnrealError::Io(error.to_string()))?;
    let result = std::fs::write(to, bytes).map_err(|error| io_error(to, &error));
    result
}

/// Instala la skill en `target`: `git clone` si hay git, si no el zip. `repo` es la URL (en los tests, un repositorio
/// local). Si ya hay algo en `target`, no se pisa.
pub fn install(repo: &str, target: &Path, git_program: Option<&Path>) -> Result<(), UnrealError> {
    if target.join("SKILL.md").is_file() {
        return Err(UnrealError::Conflict(format!("the Verse skill is already installed in {}", target.display())));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
    }
    let result = match git_program {
        Some(g) => git(g, &["clone", "--depth", "1", repo, &target.to_string_lossy()], None).map(|_| ()),
        None => {
            let zip = target.with_extension("dbv-download.zip");
            let (sha, date) = remote_head(repo, None).unwrap_or_default();
            let done = download_zip(&zip).and_then(|_| extract_zip(&zip, target, &sha, &date));
            let _ = std::fs::remove_file(&zip);
            done
        }
    };
    result
}

/// Actualiza una skill instalada: `git pull --ff-only` en un clon; si no, se vuelve a descargar el zip.
pub fn update(installed: &InstalledSkill, git_program: Option<&Path>) -> Result<(), UnrealError> {
    let path = PathBuf::from(&installed.path);
    let result = match git_program.filter(|_| installed.via_git) {
        Some(g) => git(g, &["pull", "--ff-only"], Some(&path)).map(|_| ()),
        None => {
            let zip = path.with_extension("dbv-download.zip");
            let (sha, date) = remote_head(SKILL_REPO, None).unwrap_or_default();
            let done = download_zip(&zip).and_then(|_| extract_zip(&zip, &path, &sha, &date));
            let _ = std::fs::remove_file(&zip);
            done
        }
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Repositorio git local que hace de `verse-skill` (sin red).
    fn fake_repo(dir: &Path, git_program: &Path) -> PathBuf {
        let repo = dir.join("verse skill origin");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("SKILL.md"), "# Verse\n- gotchas: gotchas.md\n").unwrap();
        for args in [&["init", "-q"][..], &["add", "."], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "uno"]] {
            git(git_program, args, Some(&repo)).unwrap();
        }
        repo
    }

    fn git_or_skip() -> Option<PathBuf> {
        crate::ai::detect::which("git")
    }

    #[test]
    fn instala_con_git_en_una_ruta_con_espacios_detecta_y_actualiza() {
        // @spec REQ-124
        let Some(git_program) = git_or_skip() else { return };
        let temp = tempfile::tempdir().unwrap();
        let repo = fake_repo(temp.path(), &git_program);
        let project = temp.path().join("Mi Proyecto UEFN");
        let home = temp.path().join("home");
        let target = skill_dir(SkillScope::Project, &project, &home);
        install(&repo.to_string_lossy(), &target, Some(&git_program)).unwrap();
        let installed = detect(Some(&project), &home, Some(&git_program)).unwrap();
        assert_eq!(installed.scope, SkillScope::Project);
        assert!(installed.via_git);
        assert_eq!(installed.commit.len(), 7);
        assert_eq!(installed.repository, SKILL_REPO);
        // Commit nuevo en el origen: se nota y «Actualizar» lo trae.
        std::fs::write(repo.join("gotchas.md"), "x").unwrap();
        git(&git_program, &["add", "."], Some(&repo)).unwrap();
        git(&git_program, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "dos"], Some(&repo)).unwrap();
        let (sha, _) = remote_head(&repo.to_string_lossy(), Some(&git_program)).unwrap();
        assert!(has_update(&installed, &sha));
        update(&installed, Some(&git_program)).unwrap();
        assert!(target.join("gotchas.md").is_file());
        let after = detect(Some(&project), &home, Some(&git_program)).unwrap();
        assert!(!has_update(&after, &sha));
    }

    #[test]
    fn no_se_duplica_si_ya_esta_instalada_y_la_del_usuario_tambien_cuenta() {
        // @spec REQ-124
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let user = skill_dir(SkillScope::User, Path::new(""), &home);
        std::fs::create_dir_all(&user).unwrap();
        std::fs::write(user.join("SKILL.md"), "# Verse").unwrap();
        std::fs::write(user.join(VERSION_FILE), r#"{"commit": "abc1234", "date": "2026-10-14"}"#).unwrap();
        let found = detect(Some(&temp.path().join("P")), &home, None).unwrap();
        assert_eq!((found.scope, found.commit.as_str(), found.date.as_str(), found.via_git), (SkillScope::User, "abc1234", "2026-10-14", false));
        assert!(matches!(install("x", &user, None), Err(UnrealError::Conflict(_))));
    }

    #[cfg(windows)]
    #[test]
    fn sin_git_descomprime_el_zip_de_github_y_guarda_la_version() {
        // @spec REQ-124 — el zip de GitHub trae una carpeta `verse-skill-main/` que se quita.
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("src").join("verse-skill-main");
        std::fs::create_dir_all(source.join("language")).unwrap();
        std::fs::write(source.join("SKILL.md"), "# Verse").unwrap();
        std::fs::write(source.join("language").join("effects.md"), "x").unwrap();
        let zip = temp.path().join("skill.zip");
        let made = quiet(Path::new("tar")).args(["-a", "-c", "-f"]).arg(&zip).arg("-C").arg(temp.path().join("src")).arg("verse-skill-main").output().unwrap();
        assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
        let target = temp.path().join("con espacios").join(".claude").join("skills").join("verse");
        extract_zip(&zip, &target, "0123456789abcdef", "2026-10-14").unwrap();
        assert!(target.join("language").join("effects.md").is_file());
        let found = detect(Some(&temp.path().join("con espacios")), &temp.path().join("home"), None).unwrap();
        assert_eq!((found.commit.as_str(), found.date.as_str()), ("0123456", "2026-10-14"));
    }

    #[test]
    fn un_zip_sin_skill_md_no_se_instala() {
        // @spec REQ-124
        let temp = tempfile::tempdir().unwrap();
        let zip = temp.path().join("vacio.zip");
        std::fs::write(&zip, b"no es un zip").unwrap();
        assert!(extract_zip(&zip, &temp.path().join("verse"), "", "").is_err());
        assert!(!temp.path().join("verse").join("SKILL.md").exists());
    }

    #[test]
    fn se_comprueba_como_mucho_una_vez_al_dia() {
        // @spec REQ-124
        assert!(should_check(None, 100));
        assert!(!should_check(Some(100), 100 + CHECK_EVERY_SECS - 1));
        assert!(should_check(Some(100), 100 + CHECK_EVERY_SECS));
    }

    #[test]
    fn la_skill_no_viaja_en_el_instalador_ni_en_el_repositorio() {
        // @spec REQ-124 — no tiene licencia: nunca se empaqueta.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let conf = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json")).unwrap();
        assert!(!conf.contains("skills") && !conf.contains("verse-skill"), "tauri.conf.json no debe empaquetar la skill");
        for dir in ["src", "unreal-plugin", "src-tauri/src", "docs"] {
            let found = walkdir::WalkDir::new(root.join(dir)).into_iter().flatten().any(|e| e.file_name() == "SKILL.md");
            assert!(!found, "hay un SKILL.md dentro de {dir}");
        }
    }
}
