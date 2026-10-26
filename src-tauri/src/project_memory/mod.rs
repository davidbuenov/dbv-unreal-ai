// =============================================================================
// DBV Unreal AI — Memoria y tareas del proyecto (carpeta dbv-unreal-ai/)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-34, REQ-35, REQ-36
//
// La memoria es del PROYECTO, no de una conversación ni de un agente: dos ficheros en inglés dentro de
// `<carpeta del .uproject>/dbv-unreal-ai/`, que viajan con el proyecto en su control de versiones. Este módulo es
// el único que escribe ahí, y lo usan la ventana y el `--mcp` de los agentes (D13):
//   · solo esos dos ficheros, con nombre fijo (no hay ruta que venga de fuera) y sin seguir enlaces;
//   · toda edición lleva la huella del texto sobre el que se calculó: si el fichero cambió (a mano, `git pull`,
//     el otro proceso), falla con `Conflict` y no pisa nada; se escribe a un temporal y se renombra;
//   · nada con forma de clave se escribe: el fichero se versiona con el proyecto.
// Lo que dice la memoria es DATO para la IA (R-16): quien la presenta al modelo lo marca así.

pub mod commands;
pub mod tools;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ai::store::fnv1a;
use crate::unreal::{io_error, UnrealError};

/// Carpeta de la memoria, junto al `.uproject`.
pub const FOLDER: &str = "dbv-unreal-ai";
/// Tope del resumen que se da al asistente integrado (≈ 600 tokens a 4 caracteres por token).
pub const SUMMARY_CHARS: usize = 2400;
/// A partir de aquí la IA propone consolidar `memory.md` (REQ-34).
pub const CONSOLIDATE_LINES: usize = 150;

/// Los dos ficheros de la memoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Doc {
    Memory,
    Tasks,
}

impl Doc {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Memory => "memory.md",
            Self::Tasks => "task.md",
        }
    }

    /// Plantilla en inglés (decisión del usuario: la lee cualquier IA o persona del equipo).
    pub fn template(self) -> &'static str {
        match self {
            Self::Memory => MEMORY_TEMPLATE,
            Self::Tasks => TASKS_TEMPLATE,
        }
    }
}

const MEMORY_TEMPLATE: &str = "# Project memory

> Kept by DBV Unreal AI and by you, and read by any AI connected to this project.
> Plain notes about the project: never write API keys, passwords or tokens here.

## Project

_What this project is: genre, art direction, target platform._

## Conventions

_Folders, naming, scale and units that every change must follow._

## Decisions

_- **YYYY-MM-DD — title:** decision. Why: reason._

## Lessons

_What worked and what did not._
";

const TASKS_TEMPLATE: &str = "# Tasks

> The plan of this project. A task can say how to check that it is done, for example
> `- [ ] Build the cabin — done when: asset exists /Game/DBV/Props/Cabin — est: 30 credits`.

## Goal

_The current goal._

## Tasks

## Next step

_What comes next._
";

/// Un fichero tal como está en disco, con la huella que hay que devolver al editarlo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub doc: Doc,
    pub exists: bool,
    pub text: String,
    /// Huella de `text` («» si no existe).
    pub hash: String,
}

/// Un cambio en uno de los dos ficheros.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Edit {
    /// Añade una entrada al final de una sección (`## <section>`), creándola si no existe.
    #[serde(rename_all = "camelCase")]
    Append { section: String, entry: String },
    /// Sustituye un texto que aparece exactamente una vez.
    #[serde(rename_all = "camelCase")]
    Replace { search: String, replace: String },
    /// Un plan aceptado (REQ-36): fija el objetivo y el siguiente paso y añade las tareas (líneas ya formadas).
    #[serde(rename_all = "camelCase")]
    Plan { goal: Option<String>, tasks: Vec<String>, next_step: Option<String> },
}

/// Una tarea de `task.md`: `- [ ] Title — done when: <criterion> — est: <n> credits`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub done: bool,
    pub title: String,
    pub criterion: Option<String>,
    pub credits: Option<f64>,
    /// La línea tal cual, para editarla con `Edit::Replace`.
    pub line: String,
}

/// Carpeta de la memoria de un proyecto (`project_dir` = carpeta del `.uproject`).
pub fn folder(project_dir: &Path) -> PathBuf {
    project_dir.join(FOLDER)
}

/// Ruta de un fichero, comprobando que ni la carpeta ni el fichero son enlaces que salgan del proyecto.
fn file_path(project_dir: &Path, doc: Doc) -> Result<PathBuf, UnrealError> {
    let dir = folder(project_dir);
    let path = dir.join(doc.file_name());
    let linked = [&dir, &path].iter().any(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()));
    if linked {
        return Err(UnrealError::Outside(format!("{} es un enlace: la memoria solo se escribe dentro del proyecto", path.display())));
    }
    Ok(path)
}

/// Lee uno de los ficheros.
pub fn read(project_dir: &Path, doc: Doc) -> Result<Snapshot, UnrealError> {
    let path = file_path(project_dir, doc)?;
    let snapshot = match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Snapshot { doc, exists: true, hash: fnv1a(&text), text }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Snapshot { doc, exists: false, text: String::new(), hash: String::new() }),
        Err(error) => Err(io_error(&path, &error)),
    };
    snapshot
}

/// ¿Está activada la memoria en este proyecto?
pub fn is_active(project_dir: &Path) -> bool {
    folder(project_dir).join(Doc::Memory.file_name()).is_file()
}

/// Escritura atómica: un temporal en la misma carpeta y un renombrado.
fn write(path: &Path, text: &str) -> Result<(), UnrealError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
    }
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, text).map_err(|error| io_error(&tmp, &error))?;
    std::fs::rename(&tmp, path).map_err(|error| io_error(path, &error))
}

/// «Activar memoria del proyecto»: crea los ficheros que falten con su plantilla. Nunca sobrescribe uno que ya
/// exista (puede venir del repositorio del equipo). Devuelve los que creó.
pub fn activate(project_dir: &Path) -> Result<Vec<Doc>, UnrealError> {
    if !project_dir.is_dir() {
        return Err(UnrealError::NotFound(format!("no existe la carpeta del proyecto {}", project_dir.display())));
    }
    let mut created = Vec::new();
    for doc in [Doc::Memory, Doc::Tasks] {
        if !read(project_dir, doc)?.exists {
            write(&file_path(project_dir, doc)?, doc.template())?;
            created.push(doc);
        }
    }
    Ok(created)
}

/// Aplica `edit` sobre el texto cuya huella es `base_hash`. Si el fichero cambió desde entonces, `Conflict`.
pub fn apply(project_dir: &Path, doc: Doc, base_hash: &str, edit: &Edit) -> Result<Snapshot, UnrealError> {
    let current = read(project_dir, doc)?;
    if !current.exists {
        return Err(UnrealError::NotFound(format!("la memoria del proyecto no está activada ({}/{} no existe)", FOLDER, doc.file_name())));
    }
    if current.hash != base_hash {
        return Err(UnrealError::Conflict(format!("{}/{} cambió desde que se leyó: vuelve a leerlo y repite el cambio", FOLDER, doc.file_name())));
    }
    let added = match edit {
        Edit::Append { entry, .. } => entry.clone(),
        Edit::Replace { replace, .. } => replace.clone(),
        Edit::Plan { goal, tasks, next_step } => [goal.clone().unwrap_or_default(), tasks.join("
"), next_step.clone().unwrap_or_default()].join("
"),
    };
    if let Some(kind) = find_secret(&added) {
        return Err(UnrealError::Format(format!("no se guarda: parece {kind}, y este fichero se versiona con el proyecto")));
    }
    let next = match edit {
        Edit::Append { section, entry } => append_entry(&current.text, section, entry)?,
        Edit::Replace { search, replace } => replace_once(&current.text, search, replace)?,
        Edit::Plan { goal, tasks, next_step } => apply_plan(&current.text, goal.as_deref(), tasks, next_step.as_deref())?,
    };
    write(&file_path(project_dir, doc)?, &next)?;
    let snapshot = Snapshot { doc, exists: true, hash: fnv1a(&next), text: next };
    Ok(snapshot)
}

/// ¿Es una línea de ayuda de la plantilla (`_…_`)? No cuenta como contenido.
fn is_placeholder(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() > 1 && trimmed.starts_with('_') && trimmed.ends_with('_')
}

/// Normaliza una entrada para comparar duplicados: minúsculas, sin viñeta ni espacios repetidos.
fn normalized(entry: &str) -> String {
    entry.trim().trim_start_matches(['-', '*']).split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Añade `entry` al final de `## section` (quita la línea de ayuda si es lo único que había). Una entrada que ya
/// está en la sección no se repite (REQ-35).
pub fn append_entry(text: &str, section: &str, entry: &str) -> Result<String, UnrealError> {
    let entry = entry.trim();
    if entry.is_empty() {
        return Err(UnrealError::Format("la entrada está vacía".into()));
    }
    let heading = format!("## {}", section.trim().trim_start_matches('#').trim());
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&heading));
    let end = start.map(|s| lines.iter().skip(s + 1).position(|l| l.starts_with("## ")).map_or(lines.len(), |i| s + 1 + i));
    let body: Vec<&str> = match (start, end) {
        (Some(s), Some(e)) => lines[s + 1..e].iter().copied().filter(|l| !is_placeholder(l) && !l.trim().is_empty()).collect(),
        _ => Vec::new(),
    };
    if body.iter().any(|l| normalized(l) == normalized(entry)) {
        return Err(UnrealError::Format(format!("ya está en «{}»", heading.trim_start_matches("## "))));
    }
    let result = match (start, end) {
        (Some(s), Some(e)) => {
            // Cabecera, una línea en blanco, lo que había (sin la ayuda), la entrada y, si hay más secciones, el resto.
            let mut out: Vec<String> = lines[..=s].iter().map(|l| (*l).to_string()).collect();
            out.push(String::new());
            out.extend(lines[s + 1..e].iter().filter(|l| !is_placeholder(l)).map(|l| (*l).to_string()).skip_while(|l| l.trim().is_empty()));
            while out.last().is_some_and(|l| l.trim().is_empty()) && out.len() > s + 2 {
                out.pop();
            }
            out.extend(entry.lines().map(str::to_string));
            if e < lines.len() {
                out.push(String::new());
                out.extend(lines[e..].iter().map(|l| (*l).to_string()));
            }
            format!("{}\n", out.join("\n"))
        }
        _ => format!("{}\n\n{heading}\n\n{entry}\n", text.trim_end()),
    };
    Ok(result)
}

/// Sustituye el cuerpo de `## section` (o la añade al final si no existe).
pub fn set_section(text: &str, section: &str, body: &str) -> String {
    let heading = format!("## {section}");
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&heading));
    let result = match start {
        Some(s) => {
            let end = lines.iter().skip(s + 1).position(|l| l.starts_with("## ")).map_or(lines.len(), |i| s + 1 + i);
            let mut out: Vec<String> = lines[..=s].iter().map(|l| (*l).to_string()).collect();
            out.extend([String::new(), body.trim().to_string()]);
            if end < lines.len() {
                out.push(String::new());
                out.extend(lines[end..].iter().map(|l| (*l).to_string()));
            }
            format!("{}
", out.join("
"))
        }
        None => format!("{}

{heading}

{}
", text.trim_end(), body.trim()),
    };
    result
}

/// Un plan aceptado sobre `task.md`: objetivo y siguiente paso nuevos, tareas añadidas a las que hubiera (las de
/// antes no se pierden: se cierran a mano o con `project_task_update`).
pub fn apply_plan(text: &str, goal: Option<&str>, tasks: &[String], next_step: Option<&str>) -> Result<String, UnrealError> {
    if tasks.iter().any(|t| !t.trim_start().starts_with("- [")) {
        return Err(UnrealError::Format("cada tarea tiene que ser una línea «- [ ] …»".into()));
    }
    let mut next = text.to_string();
    if let Some(goal) = goal.filter(|g| !g.trim().is_empty()) {
        next = set_section(&next, "Goal", goal);
    }
    for task in tasks {
        // Una tarea que ya está no se repite (y no tira el resto del plan).
        if !parse_tasks(&next).iter().any(|t| normalized(&t.line) == normalized(task)) {
            next = append_entry(&next, "Tasks", task)?;
        }
    }
    if let Some(step) = next_step.filter(|s| !s.trim().is_empty()) {
        next = set_section(&next, "Next step", step);
    }
    Ok(next)
}

/// Sustituye `search` por `replace` si aparece exactamente una vez.
pub fn replace_once(text: &str, search: &str, replace: &str) -> Result<String, UnrealError> {
    if search.is_empty() {
        return Err(UnrealError::Format("el texto a buscar está vacío".into()));
    }
    let found = text.matches(search).count();
    if found != 1 {
        return Err(UnrealError::Format(format!("el texto a sustituir aparece {found} veces (tiene que aparecer una sola)")));
    }
    Ok(text.replacen(search, replace, 1))
}

/// Si el texto contiene algo con forma de secreto, qué parece. Las claves de los servicios que usa la app y las
/// formas genéricas más comunes; mejor rechazar de más que versionar una clave.
pub fn find_secret(text: &str) -> Option<&'static str> {
    const MARKERS: [(&str, &str); 4] = [("-----BEGIN", "una clave privada"), ("Authorization:", "una cabecera de autenticación"), ("Bearer ", "un token de autenticación"), ("WLT-Api-Key", "una clave de World Labs")];
    const PREFIXES: [(&str, &str, usize); 8] = [
        ("sk-ant-", "una clave de Anthropic", 20),
        ("sk-", "una clave de API", 20),
        ("tsk_", "una clave de Tripo", 16),
        ("AIza", "una clave de Google", 30),
        ("ghp_", "un token de GitHub", 30),
        ("github_pat_", "un token de GitHub", 30),
        ("xox", "un token de Slack", 20),
        ("hf_", "un token de Hugging Face", 30),
    ];
    let marker = MARKERS.iter().find(|(m, _)| text.contains(m)).map(|(_, kind)| *kind);
    let token = text
        .split(|c: char| c.is_whitespace() || matches!(c, '`' | '"' | '\'' | '(' | ')' | ',' | ';' | '=' | ':' | '<' | '>'))
        .find_map(|word| PREFIXES.iter().find(|(prefix, _, min)| word.starts_with(prefix) && word.len() >= *min && word[prefix.len()..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).map(|(_, kind, _)| *kind));
    marker.or(token)
}

/// Secciones `## ` de un Markdown, con su cuerpo sin las líneas de ayuda de la plantilla.
pub fn sections(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, Vec<&str>)> = Vec::new();
    for line in text.lines() {
        match line.strip_prefix("## ") {
            Some(title) => out.push((title.trim().to_string(), Vec::new())),
            None => {
                if let Some((_, body)) = out.last_mut() {
                    if !is_placeholder(line) {
                        body.push(line);
                    }
                }
            }
        }
    }
    out.into_iter().map(|(title, body)| (title, body.join("\n").trim().to_string())).collect()
}

fn section<'a>(all: &'a [(String, String)], name: &str) -> &'a str {
    all.iter().find(|(title, _)| title.eq_ignore_ascii_case(name)).map_or("", |(_, body)| body.as_str())
}

/// Tareas de `task.md` (cualquier lista de casillas del fichero).
pub fn parse_tasks(text: &str) -> Vec<Task> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.strip_prefix("- [").or_else(|| trimmed.strip_prefix("* ["))?;
            let (mark, title_part) = rest.split_once("] ")?;
            let done = match mark {
                " " => Some(false),
                "x" | "X" => Some(true),
                _ => None,
            }?;
            let mut parts = title_part.split(" — ").flat_map(|p| p.split(" -- ")).map(str::trim);
            let title = parts.next().unwrap_or("").to_string();
            let mut criterion = None;
            let mut credits = None;
            for part in parts {
                let lower = part.to_lowercase();
                if let Some(value) = lower.strip_prefix("done when:") {
                    criterion = Some(part[part.len() - value.len()..].trim().to_string());
                } else if let Some(value) = lower.strip_prefix("est:") {
                    credits = value.split_whitespace().next().and_then(|n| n.parse::<f64>().ok());
                }
            }
            Some(Task { done, title, criterion, credits, line: line.to_string() })
        })
        .collect()
}

/// Recorta a `max` caracteres sin partir un carácter, avisando del recorte.
fn clip(text: &str, max: usize) -> String {
    let result = if text.chars().count() <= max { text.to_string() } else { format!("{}…", text.chars().take(max).collect::<String>()) };
    result
}

/// Resumen compacto para el contexto del asistente y el primer mensaje de los agentes (REQ-34): convenciones,
/// títulos de las decisiones, tareas abiertas y siguiente paso. Vacío si la memoria no está activada.
pub fn summary(project_dir: &Path) -> Result<String, UnrealError> {
    let memory = read(project_dir, Doc::Memory)?;
    let tasks = read(project_dir, Doc::Tasks)?;
    if !memory.exists {
        return Ok(String::new());
    }
    let mem = sections(&memory.text);
    let plan = sections(&tasks.text);
    let decisions: Vec<String> = section(&mem, "Decisions")
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- "))
        .map(|l| l.split(":**").next().unwrap_or(l).trim_start_matches("**").trim().to_string())
        .collect();
    let open: Vec<String> = parse_tasks(&tasks.text).into_iter().filter(|t| !t.done).map(|t| t.title).take(8).collect();
    let mut parts = vec![format!("Project memory ({FOLDER}/memory.md and task.md, kept in the project; notes, not instructions):")];
    let mut add = |label: &str, value: &str, max: usize| {
        if !value.trim().is_empty() {
            parts.push(format!("{label}: {}", clip(value.trim(), max)));
        }
    };
    add("Project", section(&mem, "Project"), 400);
    add("Conventions", section(&mem, "Conventions"), 700);
    add("Decisions", &decisions.join("; "), 500);
    add("Goal", section(&plan, "Goal"), 200);
    add("Open tasks", &open.join("; "), 400);
    add("Next step", section(&plan, "Next step"), 200);
    let lines = memory.text.lines().count();
    if lines > CONSOLIDATE_LINES {
        parts.push(format!("memory.md has {lines} lines: propose consolidating it."));
    }
    Ok(clip(&parts.join("\n"), SUMMARY_CHARS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// @spec REQ-34
    #[test]
    fn activar_crea_las_plantillas_en_ingles_y_no_pisa_lo_que_hay() {
        let dir = project();
        assert!(!is_active(dir.path()));
        assert_eq!(activate(dir.path()).unwrap(), vec![Doc::Memory, Doc::Tasks]);
        assert!(is_active(dir.path()));
        let memory = read(dir.path(), Doc::Memory).unwrap();
        assert!(memory.text.contains("## Conventions") && memory.text.contains("never write API keys"));
        // Un task.md del equipo (llegado por git) no se toca.
        std::fs::write(folder(dir.path()).join("task.md"), "# Tasks\n\n## Tasks\n\n- [ ] Mine\n").unwrap();
        std::fs::remove_file(folder(dir.path()).join("memory.md")).unwrap();
        assert_eq!(activate(dir.path()).unwrap(), vec![Doc::Memory]);
        assert!(read(dir.path(), Doc::Tasks).unwrap().text.contains("Mine"));
    }

    /// @spec REQ-34, REQ-35
    #[test]
    fn anade_a_su_seccion_quitando_la_ayuda_y_sin_repetir() {
        let dir = project();
        activate(dir.path()).unwrap();
        let base = read(dir.path(), Doc::Memory).unwrap();
        let saved = apply(dir.path(), Doc::Memory, &base.hash, &Edit::Append { section: "Conventions".into(), entry: "- Props go in `/Game/DBV/Props`.".into() }).unwrap();
        let conventions = sections(&saved.text).into_iter().find(|(t, _)| t == "Conventions").unwrap().1;
        assert_eq!(conventions, "- Props go in `/Game/DBV/Props`.");
        assert!(saved.text.contains("## Decisions"), "las demás secciones siguen");
        let again = apply(dir.path(), Doc::Memory, &saved.hash, &Edit::Append { section: "conventions".into(), entry: "-  props go in `/Game/DBV/Props`.".into() });
        assert!(matches!(again, Err(UnrealError::Format(m)) if m.contains("ya está")));
        let new_section = apply(dir.path(), Doc::Memory, &saved.hash, &Edit::Append { section: "Characters".into(), entry: "- Pirate: biped rig v1.0.".into() }).unwrap();
        assert!(new_section.text.trim_end().ends_with("## Characters\n\n- Pirate: biped rig v1.0."));
    }

    /// Dos escritores (ventana y `--mcp`) o una edición a mano entre leer y guardar: no se pisa nada. @spec REQ-34
    #[test]
    fn una_edicion_sobre_texto_viejo_falla_y_no_pisa() {
        let dir = project();
        activate(dir.path()).unwrap();
        let base = read(dir.path(), Doc::Memory).unwrap();
        let path = folder(dir.path()).join("memory.md");
        std::fs::write(&path, format!("{}\n- edited by hand\n", base.text)).unwrap();
        let result = apply(dir.path(), Doc::Memory, &base.hash, &Edit::Append { section: "Lessons".into(), entry: "- x".into() });
        assert!(matches!(result, Err(UnrealError::Conflict(_))));
        assert!(std::fs::read_to_string(&path).unwrap().contains("edited by hand"));
        assert!(!folder(dir.path()).join("memory.md.tmp").exists());
    }

    /// El fichero se versiona con el proyecto: una clave nunca llega a escribirse. @spec REQ-34
    #[test]
    fn rechaza_lo_que_parece_una_clave() {
        assert_eq!(find_secret("the Tripo key is tsk_abcdEFGH12345678xyz"), Some("una clave de Tripo"));
        assert_eq!(find_secret("use `sk-ant-api03-AAAAAAAAAAAAAAAAAAAA`"), Some("una clave de Anthropic"));
        assert_eq!(find_secret("header Authorization: Basic x"), Some("una cabecera de autenticación"));
        assert_eq!(find_secret("- Use sk-style names for skeletons"), None, "una palabra corta no es una clave");
        assert_eq!(find_secret("- Props go in /Game/DBV/Props, scale 1 uu = 1 cm."), None);
        let dir = project();
        activate(dir.path()).unwrap();
        let base = read(dir.path(), Doc::Memory).unwrap();
        let result = apply(dir.path(), Doc::Memory, &base.hash, &Edit::Append { section: "Lessons".into(), entry: "- Tripo key: tsk_abcdEFGH12345678xyz".into() });
        assert!(matches!(result, Err(UnrealError::Format(m)) if m.contains("clave de Tripo")));
        assert_eq!(read(dir.path(), Doc::Memory).unwrap().hash, base.hash);
    }

    /// @spec REQ-34, RNF-IA.3
    #[test]
    fn no_escribe_a_traves_de_un_enlace() {
        let outside = project();
        let dir = project();
        let linked = {
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(outside.path(), folder(dir.path())).is_ok()
            }
            #[cfg(not(windows))]
            {
                std::os::unix::fs::symlink(outside.path(), folder(dir.path())).is_ok()
            }
        };
        // En Windows crear enlaces puede exigir permisos: sin enlace no hay nada que probar.
        if linked {
            assert!(matches!(activate(dir.path()), Err(UnrealError::Outside(_))));
            assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
        }
    }

    /// @spec REQ-36
    #[test]
    fn lee_las_tareas_con_su_criterio_y_su_coste() {
        let text = "## Tasks\n\n- [ ] Build the cabin — done when: asset exists /Game/DBV/Props/Cabin — est: 30 credits\n- [x] Pick a style\n  - [ ] Generate five props -- est: 100 credits\n- [?] not a task\n";
        let tasks = parse_tasks(text);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].title, "Build the cabin");
        assert_eq!(tasks[0].criterion.as_deref(), Some("asset exists /Game/DBV/Props/Cabin"));
        assert_eq!(tasks[0].credits, Some(30.0));
        assert!(tasks[1].done && tasks[1].criterion.is_none());
        assert_eq!((tasks[2].title.as_str(), tasks[2].credits), ("Generate five props", Some(100.0)));
    }

    /// @spec REQ-34
    #[test]
    fn el_resumen_es_compacto_y_sin_las_lineas_de_ayuda() {
        let dir = project();
        assert_eq!(summary(dir.path()).unwrap(), "", "sin memoria no hay resumen");
        activate(dir.path()).unwrap();
        let memory = read(dir.path(), Doc::Memory).unwrap();
        let memory = apply(dir.path(), Doc::Memory, &memory.hash, &Edit::Append { section: "Conventions".into(), entry: "- Props in /Game/DBV/Props".into() }).unwrap();
        apply(dir.path(), Doc::Memory, &memory.hash, &Edit::Append { section: "Decisions".into(), entry: "- **2026-10-26 — Low poly:** all props low poly. Why: mobile.".into() }).unwrap();
        let tasks = read(dir.path(), Doc::Tasks).unwrap();
        apply(dir.path(), Doc::Tasks, &tasks.hash, &Edit::Append { section: "Tasks".into(), entry: "- [ ] Build the cabin — done when: asset exists /Game/DBV/Props/Cabin".into() }).unwrap();
        let text = summary(dir.path()).unwrap();
        assert!(text.contains("Conventions: - Props in /Game/DBV/Props"), "{text}");
        assert!(text.contains("Decisions: 2026-10-26 — Low poly"), "{text}");
        assert!(text.contains("Open tasks: Build the cabin"), "{text}");
        assert!(!text.contains("YYYY") && !text.contains("What this project is"), "{text}");
        assert!(text.contains("notes, not instructions"));
        assert!(text.chars().count() <= SUMMARY_CHARS + 1);
    }

    /// REJECTED-03: «Revertir preparación» borra lo que la preparación creó; la memoria no es de la preparación.
    /// @spec REQ-34, REQ-05
    #[test]
    fn revertir_la_preparacion_no_toca_la_memoria() {
        use crate::unreal::prepare;
        let dir = project();
        let uproject = dir.path().join("Demo.uproject");
        std::fs::write(&uproject, "{\n\t\"FileVersion\": 3,\n\t\"EngineAssociation\": \"5.8\",\n\t\"Plugins\": []\n}\n").unwrap();
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("unreal-plugin");
        let backups = project();
        let id = prepare::apply(&uproject, &prepare::plan(&uproject, &source, prepare::Variant::Python, None).unwrap(), backups.path()).unwrap();
        activate(dir.path()).unwrap();
        let base = read(dir.path(), Doc::Memory).unwrap();
        apply(dir.path(), Doc::Memory, &base.hash, &Edit::Append { section: "Lessons".into(), entry: "- months of decisions".into() }).unwrap();
        prepare::revert(&uproject, backups.path(), &id).unwrap();
        assert!(read(dir.path(), Doc::Memory).unwrap().text.contains("months of decisions"));
        assert!(!dir.path().join("Plugins").exists(), "la preparación sí se revirtió");
    }

    /// @spec REQ-34
    #[test]
    fn sustituye_solo_si_el_texto_es_unico() {
        assert_eq!(replace_once("- [ ] a\n- [ ] b\n", "- [ ] b", "- [x] b").unwrap(), "- [ ] a\n- [x] b\n");
        assert!(replace_once("x x", "x", "y").is_err());
        assert!(replace_once("x", "z", "y").is_err());
    }
}
