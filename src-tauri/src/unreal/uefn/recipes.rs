// =============================================================================
// DBV Unreal AI — Recetas de UEFN con Scene Graph primero
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-129
//
// Las recetas de UEFN se definen en la app (`uefn_recipes.json`), no en el `recipes.json` del plugin, que en UEFN no
// existe. Cada una dice qué componentes crear y con qué única responsabilidad, los pasos (en inglés, los lee la IA) y
// las líneas `ZZTEST` que debe imprimir. La comprobación es `BuildAll` sin errores y, si hay una sesión conectada, esas
// líneas en la salida de Verse. Ninguna escribe la jugabilidad en clases `creative_device`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const RECIPES_JSON: &str = include_str!("uefn_recipes.json");

pub const LIST: &str = "uefn_recipes_list";
pub const CHECK: &str = "uefn_recipe_check";

/// Una receta de UEFN. `title` y `summary` por idioma; `components`, `steps` y `expected` en inglés.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UefnRecipe {
    pub id: String,
    pub title: BTreeMap<String, String>,
    pub summary: BTreeMap<String, String>,
    pub needs: Vec<String>,
    pub components: Vec<String>,
    pub steps: String,
    /// Líneas que la salida de Verse debe contener con una sesión en marcha.
    pub expected: Vec<String>,
}

#[derive(Deserialize)]
struct RecipesFile {
    recipes: Vec<UefnRecipe>,
}

pub fn all() -> Vec<UefnRecipe> {
    serde_json::from_str::<RecipesFile>(RECIPES_JSON).map(|file| file.recipes).unwrap_or_default()
}

pub fn find(id: &str) -> Option<UefnRecipe> {
    all().into_iter().find(|recipe| recipe.id == id)
}

fn english(texts: &BTreeMap<String, String>) -> String {
    texts.get("en").cloned().unwrap_or_default()
}

/// Lo que se le pide a la IA al elegir la receta en el menú.
pub fn prompt(recipe: &UefnRecipe) -> String {
    let components = if recipe.components.is_empty() { String::new() } else { format!("\n\nComponents (one responsibility each):\n- {}", recipe.components.join("\n- ")) };
    format!(
        "Follow the DBV Unreal AI UEFN recipe \"{}\" (id {}). Scene Graph first: gameplay in components, never in creative_device classes. Ask for approval before each change.{components}\n\n{}\n\nWhen you finish, call {CHECK} with id \"{}\" and tell the user what passed and what is still missing.",
        english(&recipe.title),
        recipe.id,
        recipe.steps,
        recipe.id
    )
}

fn list_text() -> String {
    all()
        .iter()
        .map(|r| format!("- {} — {}: {}\n  Steps: {}", r.id, english(&r.title), english(&r.summary), r.steps))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Las dos herramientas de recetas de UEFN, iguales para la conversación y el `--mcp`. Comprobar compila (no cambia
/// el proyecto), así que no pide permiso.
pub fn tool_specs() -> Vec<Value> {
    let ids: Vec<String> = all().into_iter().map(|r| r.id).collect();
    vec![
        json!({ "name": LIST, "description": "Guided UEFN recipes (scoring zone, players with late join, timer and fuse, bring a Library model to the island): components, steps and the ZZTEST lines to expect. Read-only.", "inputSchema": { "type": "object", "properties": {}, "required": [] }, "annotations": { "readOnlyHint": true } }),
        json!({ "name": CHECK, "description": "Check a UEFN recipe: builds Verse with BuildAll and, if a play session is connected, looks for its expected ZZTEST lines in the Verse output. Run it when you finish a recipe.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string", "enum": ids } }, "required": ["id"] }, "annotations": { "readOnlyHint": true } }),
    ]
}

/// Informe de la comprobación: compila o no, y qué líneas esperadas aparecieron (si había sesión).
pub fn check_report(recipe: &UefnRecipe, build: &super::verse::VerseBuild, output: Option<&str>) -> String {
    let compile = if build.session_running {
        "– BuildAll: not run, a play session is connected (stop it or push your changes with uefn_session_push).".to_string()
    } else if build.ok {
        "✓ BuildAll: no errors.".to_string()
    } else {
        format!("✗ BuildAll: {} error(s): {}", build.errors.len(), build.errors.iter().map(|e| format!("{}:{} {} {}", e.relative, e.line, e.code, e.message)).collect::<Vec<_>>().join("; "))
    };
    let lines: Vec<String> = match output {
        Some(text) => recipe.expected.iter().map(|line| format!("{} output: \"{line}\"", if text.contains(line.as_str()) { "✓" } else { "✗" })).collect(),
        None if !recipe.expected.is_empty() => vec!["– output: no play session connected; start one with uefn_session_start to check the ZZTEST lines.".to_string()],
        None => Vec::new(),
    };
    std::iter::once(compile).chain(lines).collect::<Vec<_>>().join("\n")
}

/// Ejecuta una herramienta de recetas de UEFN en el editor de `url` para el proyecto de `project_dir`.
pub fn run(name: &str, args: &Value, url: &str, project_dir: Option<&Path>) -> Result<String, String> {
    let result = match name {
        LIST => Ok(list_text()),
        CHECK => {
            let id = args.get("id").and_then(Value::as_str).unwrap_or("");
            let recipe = find(id).ok_or_else(|| format!("unknown UEFN recipe \"{id}\"; call {LIST}"))?;
            let module = project_dir.and_then(super::verse::uefn_project_at).map(|p| super::verse::module_of(&p)).unwrap_or_default();
            let build = super::verse::build_all(url, &module)?;
            // Con sesión conectada, la salida de Verse dice si las líneas esperadas salieron.
            let output = if build.session_running { super::session::run(super::session::OUTPUT, &json!({ "pattern": "ZZTEST", "max_results": 200 }), url, None, &std::thread::sleep).ok() } else { None };
            Ok(check_report(&recipe, &build, output.as_deref()))
        }
        other => Err(format!("unknown UEFN recipe tool {other}")),
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::uefn::verse::{VerseBuild, VerseDiagnostic};

    #[test]
    fn hay_cuatro_recetas_con_scene_graph_primero() {
        // @spec REQ-129
        let recipes = all();
        let ids: Vec<&str> = recipes.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["uefn_scoring_zone", "uefn_late_join_players", "uefn_timer_fuse", "uefn_import_model"]);
        for recipe in &recipes {
            for lang in ["es", "en", "zh-CN"] {
                assert!(recipe.title.contains_key(lang) && recipe.summary.contains_key(lang), "{} {lang}", recipe.id);
            }
            assert!(recipe.components.iter().all(|c| c.contains("class<final_super>(component)")), "{}", recipe.id);
            let text = format!("{} {}", recipe.steps, recipe.components.join(" "));
            assert!(!text.contains("class(creative_device)") && !text.contains(":= class<concrete>(creative_device)"), "{}: sin creative_device para la jugabilidad", recipe.id);
        }
        let late = find("uefn_late_join_players").unwrap();
        assert!(late.steps.contains("TeamMemberSpawnedEvent"));
        assert!(find("uefn_timer_fuse").unwrap().steps.contains("race"));
        assert!(find("uefn_import_model").unwrap().steps.contains("StaticMeshToolset") && find("uefn_import_model").unwrap().steps.contains("ImportFile"));
    }

    #[test]
    fn el_prompt_pide_comprobar_con_la_herramienta_de_uefn() {
        // @spec REQ-129
        let text = prompt(&find("uefn_scoring_zone").unwrap());
        assert!(text.contains("never in creative_device") && text.contains(CHECK) && text.contains("zz_scoring_zone_component"));
    }

    fn build(ok: bool, session: bool) -> VerseBuild {
        let error = VerseDiagnostic { severity: "Error".into(), code: 3512, message: "decides".into(), verse_path: "/m/zz.verse".into(), relative: "Content/zz.verse".into(), line: 4, column: 1 };
        VerseBuild { ok, session_running: session, errors: if ok { vec![] } else { vec![error] }, warnings: vec![], experimental: false }
    }

    #[test]
    fn la_comprobacion_es_build_all_y_las_lineas_esperadas() {
        // @spec REQ-129
        let recipe = find("uefn_timer_fuse").unwrap();
        assert_eq!(check_report(&recipe, &build(true, false), None), "✓ BuildAll: no errors.\n– output: no play session connected; start one with uefn_session_start to check the ZZTEST lines.");
        assert!(check_report(&recipe, &build(false, false), None).contains("✗ BuildAll: 1 error(s): Content/zz.verse:4 3512"));
        let with_session = check_report(&recipe, &build(false, true), Some("zz_fuse.verse:9 ZZTEST fuse armed"));
        assert!(with_session.contains("not run, a play session") && with_session.contains("✓ output: \"ZZTEST fuse armed\""));
        let missing = check_report(&recipe, &build(false, true), Some("nada"));
        assert!(missing.contains("✗ output"));
        assert_eq!(check_report(&find("uefn_import_model").unwrap(), &build(true, false), None), "✓ BuildAll: no errors.");
    }

    #[test]
    fn las_herramientas_se_ofrecen_con_uefn_y_no_piden_permiso() {
        // @spec REQ-129
        for spec in tool_specs() {
            let name = spec["name"].as_str().unwrap();
            assert!(super::super::server::available_in_uefn(name));
            assert_eq!(spec["annotations"]["readOnlyHint"], true);
        }
        assert!(run("uefn_recipe_check", &json!({ "id": "nope" }), "http://127.0.0.1:1/mcp", None).unwrap_err().contains("unknown UEFN recipe"));
    }
}
