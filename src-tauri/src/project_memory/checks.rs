// =============================================================================
// DBV Unreal AI — Comprobaciones de «hecho» de las tareas en el editor
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-37
//
// «El estado del editor es el juez» (§8 de la spec) aplicado al plan: una tarea con un criterio de un tipo fijo
// solo se marca hecha si el editor lo confirma. La comprobación es CÓDIGO FIJO DE LA APP que solo lee: el criterio
// llega como dato (hexadecimal de un JSON que Python decodifica), así que un `task.md` ajeno no puede colar código
// (R-16). Se ejecuta con `run_python` del toolset DBV, igual en sus dos variantes sin recompilar la de C++ (D14).

use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use super::{apply, parse_tasks, read, Doc, Edit, Task};

/// Lo que se sabe comprobar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    AssetExists(String),
    ActorExists { name: String, min: u32 },
    LevelIs(String),
    NoErrorsInLog,
}

/// Interpreta un criterio; `None` si es texto libre (lo confirma la persona).
pub fn parse(criterion: &str) -> Option<Check> {
    let text = criterion.trim();
    let lower = text.to_lowercase();
    let rest = |prefix: &str| text[prefix.len()..].trim().to_string();
    let check = if lower.starts_with("asset exists ") {
        Some(Check::AssetExists(rest("asset exists ")))
    } else if lower.starts_with("level is ") {
        Some(Check::LevelIs(rest("level is ")))
    } else if lower.starts_with("no errors in log") {
        Some(Check::NoErrorsInLog)
    } else if lower.starts_with("actor exists ") {
        let body = rest("actor exists ");
        let (name, min) = match body.to_lowercase().find(" count >= ") {
            Some(at) => (body[..at].trim().to_string(), body[at + 10..].trim().parse::<u32>().ok().unwrap_or(1)),
            None => (body.clone(), 1),
        };
        Some(Check::ActorExists { name, min: min.max(1) })
    } else {
        None
    };
    check.filter(|c| !matches!(c, Check::AssetExists(p) | Check::LevelIs(p) if !p.starts_with('/')))
}

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

/// Código Python (fijo) que comprueba `check` sin modificar nada y deja `result = {"passed": …, "detail": …}`.
pub fn python(check: &Check) -> String {
    let args = match check {
        Check::AssetExists(path) => json!({ "kind": "asset", "path": path }),
        Check::ActorExists { name, min } => json!({ "kind": "actor", "name": name, "min": min }),
        Check::LevelIs(path) => json!({ "kind": "level", "path": path }),
        Check::NoErrorsInLog => json!({ "kind": "log" }),
    };
    format!(
        r#"import json, unreal
_a = json.loads(bytes.fromhex("{}").decode("utf-8"))
if _a["kind"] == "asset":
    _ok = bool(unreal.EditorAssetLibrary.does_asset_exist(_a["path"]))
    result = {{"passed": _ok, "detail": ("found " if _ok else "missing ") + _a["path"]}}
elif _a["kind"] == "actor":
    _n = _a["name"].lower()
    _found = [x for x in unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors() if x.get_actor_label().lower().startswith(_n) or x.get_class().get_name().lower() == _n]
    result = {{"passed": len(_found) >= _a["min"], "detail": "%d actor(s) match %s (need %d)" % (len(_found), _a["name"], _a["min"])}}
elif _a["kind"] == "level":
    _w = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    _p = _w.get_path_name().split(".")[0] if _w else ""
    result = {{"passed": _p.lower() == _a["path"].split(".")[0].lower(), "detail": "open level: " + _p}}
else:
    result = {{"passed": None, "detail": "the editor log cannot be checked automatically yet: the user confirms it"}}
"#,
        hex(&args.to_string())
    )
}

/// Resultado de una comprobación: `None` en `passed` = no se pudo comprobar.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub passed: Option<bool>,
    pub detail: String,
}

/// Lee lo que devolvió `run_python` (`{success, result, error…}` ya desenvuelto).
pub fn interpret(value: &Value) -> Outcome {
    let result = value.get("result").cloned().unwrap_or(Value::Null);
    let outcome = if value.get("success") == Some(&json!(false)) {
        Outcome { passed: None, detail: format!("the check could not run: {}", value.get("error").and_then(Value::as_str).unwrap_or("unknown error")) }
    } else {
        Outcome { passed: result.get("passed").and_then(Value::as_bool), detail: result.get("detail").and_then(Value::as_str).unwrap_or("").to_string() }
    };
    outcome
}

/// Qué pasó al intentar marcar o reabrir una tarea.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    /// Se escribió el cambio en `task.md`.
    pub saved: bool,
    pub task: String,
    /// Resultado de la comprobación, si la hubo.
    pub check: Option<Outcome>,
    pub message: String,
}

/// Busca una tarea por su título: exacto (sin distinguir mayúsculas) o, si no, el único que empieza así.
pub fn find_task<'a>(tasks: &'a [Task], title: &str) -> Result<&'a Task, String> {
    let wanted = title.trim().to_lowercase();
    let exact: Vec<&Task> = tasks.iter().filter(|t| t.title.to_lowercase() == wanted).collect();
    let prefix: Vec<&Task> = tasks.iter().filter(|t| t.title.to_lowercase().starts_with(&wanted)).collect();
    let found = match (exact.as_slice(), prefix.as_slice()) {
        ([one, ..], _) => Ok(*one),
        ([], [one]) => Ok(*one),
        ([], []) => Err(format!("no task «{title}» in the plan (see project_tasks_list)")),
        _ => Err(format!("«{title}» matches several tasks: use the full title")),
    };
    found
}

/// Marca (`done`) o reabre una tarea. Con un criterio comprobable, solo se marca si `run` (el editor) lo confirma;
/// con uno en texto libre, la IA no la marca: lo confirma la persona en la vista Plan (REQ-37).
pub fn update_task(project_dir: &Path, title: &str, done: bool, by_ai: bool, run: impl FnOnce(&str) -> Result<Value, String>) -> Result<Update, String> {
    let snapshot = read(project_dir, Doc::Tasks).map_err(|e| e.to_string())?;
    let tasks = parse_tasks(&snapshot.text);
    let task = find_task(&tasks, title)?.clone();
    let check = task.criterion.as_deref().and_then(parse);
    let free_text = task.criterion.is_some() && check.is_none();
    let outcome = match (&check, done) {
        (Some(c), true) => Some(run(&python(c)).map(|v| interpret(&v)).unwrap_or_else(|error| Outcome { passed: None, detail: error })),
        _ => None,
    };
    let allowed = !done || (outcome.as_ref().map_or(!(by_ai && free_text), |o| o.passed == Some(true)));
    let message = match (&outcome, allowed, done) {
        (_, true, false) => format!("Reopened «{}».", task.title),
        (Some(o), true, true) => format!("Done: «{}» — the editor confirms it ({}).", task.title, o.detail),
        (None, true, true) => format!("Done: «{}».", task.title),
        (Some(o), false, _) if o.passed.is_none() => format!("Not marked: «{}» could not be checked ({}). It stays pending until it can be checked or the user marks it.", task.title, o.detail),
        (Some(o), false, _) => format!("Not marked: «{}» is not done yet according to the editor ({}).", task.title, o.detail),
        (None, false, _) => format!("Not marked: «{}» is checked by the user (done when: {}). Ask them to confirm it in the Plan view.", task.title, task.criterion.clone().unwrap_or_default()),
    };
    let new_line = if done { task.line.replacen("[ ]", "[x]", 1) } else { task.line.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1) };
    let saved = allowed && new_line != task.line;
    if saved {
        // La línea entera (con sus saltos): el ejemplo de la plantilla, citado en el texto, no cuenta como la tarea.
        apply(project_dir, Doc::Tasks, &snapshot.hash, &Edit::Replace { search: format!("\n{}\n", task.line), replace: format!("\n{new_line}\n") }).map_err(|e| e.to_string())?;
    }
    Ok(Update { saved, task: task.title, check: outcome, message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_memory::{activate, tools::propose_plan_and_save};

    /// @spec REQ-37
    #[test]
    fn entiende_los_criterios_comprobables_y_deja_el_texto_libre() {
        assert_eq!(parse("asset exists /Game/DBV/Props/Cabin"), Some(Check::AssetExists("/Game/DBV/Props/Cabin".into())));
        assert_eq!(parse("Actor exists PointLight count >= 3"), Some(Check::ActorExists { name: "PointLight".into(), min: 3 }));
        assert_eq!(parse("actor exists Pirata"), Some(Check::ActorExists { name: "Pirata".into(), min: 1 }));
        assert_eq!(parse("level is /Game/DBV/Maps/Test"), Some(Check::LevelIs("/Game/DBV/Maps/Test".into())));
        assert_eq!(parse("no errors in log since task start"), Some(Check::NoErrorsInLog));
        assert_eq!(parse("the user likes the lighting"), None);
        assert_eq!(parse("asset exists Cabin"), None, "una ruta que no es de Unreal no se comprueba");
    }

    /// Un `task.md` ajeno no puede colar código en la comprobación: el criterio va como dato. @spec REQ-37, R-16
    #[test]
    fn el_criterio_viaja_como_dato_y_no_como_codigo() {
        let code = python(&Check::AssetExists("/Game/x\")\nimport os; os.remove('a')#".into()));
        assert!(!code.contains("os.remove"), "{code}");
        assert!(code.contains("bytes.fromhex(\""));
        assert!(code.contains("does_asset_exist") && !code.contains("delete") && !code.contains("spawn"));
    }

    fn project_with_plan() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        activate(dir.path()).unwrap();
        propose_plan_and_save(
            dir.path(),
            &json!({ "tasks": [{ "title": "Build the cabin", "done_when": "asset exists /Game/DBV/Props/Cabin" }, { "title": "Light the scene", "done_when": "the user likes the lighting" }, { "title": "Pick a style" }] }),
        )
        .unwrap();
        dir
    }

    fn task(dir: &Path, title: &str) -> Task {
        parse_tasks(&read(dir, Doc::Tasks).unwrap().text).into_iter().find(|t| t.title == title).unwrap()
    }

    /// Solo se marca hecha si el editor lo confirma. @spec REQ-37
    #[test]
    fn una_tarea_comprobable_solo_se_marca_si_el_editor_lo_confirma() {
        let dir = project_with_plan();
        let missing = update_task(dir.path(), "build the cabin", true, true, |_| Ok(json!({ "success": true, "result": { "passed": false, "detail": "missing /Game/DBV/Props/Cabin" } }))).unwrap();
        assert!(!missing.saved && missing.message.contains("not done yet"));
        assert!(!task(dir.path(), "Build the cabin").done);
        let offline = update_task(dir.path(), "Build", true, false, |_| Err("no editor connected".into())).unwrap();
        assert!(!offline.saved && offline.message.contains("could not be checked"), "pendiente de comprobar");
        let found = update_task(dir.path(), "Build the cabin", true, true, |code| {
            assert!(code.contains("does_asset_exist"));
            Ok(json!({ "success": true, "result": { "passed": true, "detail": "found /Game/DBV/Props/Cabin" } }))
        })
        .unwrap();
        assert!(found.saved && found.message.contains("the editor confirms"));
        assert!(task(dir.path(), "Build the cabin").done);
        let reopened = update_task(dir.path(), "Build the cabin", false, true, |_| panic!("reabrir no comprueba")).unwrap();
        assert!(reopened.saved && !task(dir.path(), "Build the cabin").done);
    }

    /// El texto libre lo confirma la persona; sin criterio, la IA puede marcarla. @spec REQ-37
    #[test]
    fn el_texto_libre_lo_confirma_la_persona() {
        let dir = project_with_plan();
        let by_ai = update_task(dir.path(), "Light the scene", true, true, |_| panic!("no hay nada que comprobar")).unwrap();
        assert!(!by_ai.saved && by_ai.message.contains("Ask them to confirm"));
        assert!(update_task(dir.path(), "Light the scene", true, false, |_| panic!()).unwrap().saved, "la persona sí");
        assert!(update_task(dir.path(), "Pick a style", true, true, |_| panic!()).unwrap().saved, "sin criterio, la IA puede");
        assert!(update_task(dir.path(), "Nothing", true, true, |_| panic!()).unwrap_err().contains("no task"));
    }
}
