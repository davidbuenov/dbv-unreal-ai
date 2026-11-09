// =============================================================================
// DBV Unreal AI — Recetas guiadas
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-23
//
// Una receta es un prompt con pasos y una comprobación final sobre el estado del editor (D43). Se definen UNA vez en
// `recipes.json`, dentro del paquete Python del plugin: de ahí salen el menú de la conversación y las herramientas
// (aquí, compilado en el binario) y las Agent Skills y la comprobación (en el editor, `recipes.py`). Las herramientas
// son de solo lectura y las mismas en la ventana y en el `--mcp` (D10); cada paso que modifica sigue pidiendo permiso.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const RECIPES_JSON: &str = include_str!("../../unreal-plugin/DBVUnrealAI/Content/Python/dbv_unreal_ai/recipes.json");

/// Una receta. `title` y `summary` por idioma (`es`, `en` y, con REQ-64, `zh-CN`); `steps` en inglés: lo lee la IA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub title: BTreeMap<String, String>,
    pub summary: BTreeMap<String, String>,
    /// Qué hace falta: `editor`, `tripo`, `world` (un mundo en la biblioteca), `convai`.
    pub needs: Vec<String>,
    pub steps: String,
}

#[derive(Deserialize)]
struct RecipesFile {
    recipes: Vec<Recipe>,
}

/// Todas las recetas, en el orden del fichero.
pub fn all() -> Vec<Recipe> {
    serde_json::from_str::<RecipesFile>(RECIPES_JSON).map(|file| file.recipes).unwrap_or_default()
}

pub fn find(id: &str) -> Option<Recipe> {
    all().into_iter().find(|recipe| recipe.id == id)
}

/// Texto de un idioma, con el inglés como respaldo.
fn text(texts: &BTreeMap<String, String>, lang: &str) -> String {
    texts.get(lang).or_else(|| texts.get("en")).cloned().unwrap_or_default()
}

/// Lo que se le pide a la IA al elegir una receta en el menú (en inglés; ella responde en el idioma del usuario).
pub fn prompt(recipe: &Recipe) -> String {
    format!(
        "Follow the DBV Unreal AI recipe \"{}\" (id {}). Ask for approval before each change, as usual, and reuse what the level already has.\n\n{}\n\nWhen you finish, call recipe_check with id \"{}\" and tell the user what passed and what is still missing.",
        text(&recipe.title, "en"),
        recipe.id,
        recipe.steps,
        recipe.id
    )
}

/// Código Python FIJO que comprueba la receta en el editor (solo lee). El id ya se validó contra la lista.
pub fn check_code(id: &str) -> Result<String, String> {
    let recipe = find(id).ok_or_else(|| format!("unknown recipe \"{id}\"; call recipes_list"))?;
    Ok(format!("from dbv_unreal_ai import recipes\nresult = recipes.check({})", json!(recipe.id)))
}

/// Resultado de la comprobación para la IA: una línea por punto.
pub fn report_text(value: &Value) -> String {
    let result = value.get("result").cloned().unwrap_or(Value::Null);
    let text = if value.get("success").and_then(Value::as_bool) != Some(true) {
        let error = value.get("error").and_then(Value::as_str).unwrap_or("the editor did not run the check");
        // Un toolset anterior a las recetas no tiene `recipes`: se dice qué hacer.
        if error.contains("recipes") {
            format!("error: the DBV Unreal AI toolset of this project is older than the recipes: use «Prepare project» to update it and restart the editor ({error})")
        } else {
            format!("error: {error}")
        }
    } else {
        let items = result.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
        let lines: Vec<String> = items
            .iter()
            .map(|item| format!("{} {}: {}", if item.get("ok") == Some(&json!(true)) { "✓" } else { "✗" }, item.get("id").and_then(Value::as_str).unwrap_or("?"), item.get("detail").and_then(Value::as_str).unwrap_or("")))
            .collect();
        format!("{}\n{}", result.get("detail").and_then(Value::as_str).unwrap_or(""), lines.join("\n"))
    };
    text
}

/// Lista para la IA (`recipes_list`).
pub fn list_text() -> String {
    all()
        .iter()
        .map(|recipe| format!("- {} — {}: {} (needs: {})\n  Steps: {}", recipe.id, text(&recipe.title, "en"), text(&recipe.summary, "en"), recipe.needs.join(", "), recipe.steps))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Las dos herramientas de recetas, iguales para el asistente integrado y el MCP propio (D10).
pub fn specs() -> Vec<Value> {
    let ids: Vec<String> = all().into_iter().map(|recipe| recipe.id).collect();
    vec![
        tool(
            "recipes_list",
            "Guided recipes of DBV Unreal AI (test level, Tripo character with animations, walk through a World Labs world, MetaHuman that talks with Convai): id, what it builds, what it needs and the steps to follow. Use them when the user asks for one of these things. Read-only.",
            json!({}),
            &[],
        ),
        tool(
            "recipe_check",
            "Check in the editor what a recipe has already achieved and what is missing (for example floor, light, sky, PlayerStart and NavMesh for test_level). Run it when you finish a recipe. Read-only.",
            json!({ "id": { "type": "string", "enum": ids } }),
            &["id"],
        ),
    ]
}

/// Ejecuta una herramienta de recetas: la misma para la ventana y el `--mcp`. `run_fixed` ejecuta Python fijo en el
/// editor (solo se usa con `recipe_check`).
pub fn run_tool(name: &str, args: &Value, run_fixed: impl FnOnce(&str) -> Result<Value, String>) -> Result<String, String> {
    let result = match name {
        "recipes_list" => Ok(list_text()),
        "recipe_check" => {
            let code = check_code(args.get("id").and_then(Value::as_str).unwrap_or_default())?;
            run_fixed(&code).map(|value| report_text(&value))
        }
        other => Err(format!("unknown recipe tool {other}")),
    };
    result
}

/// Las recetas para el menú de la conversación. @spec REQ-23
#[tauri::command]
pub fn recipes_all() -> Vec<Recipe> {
    all()
}

/// Lo que se le pide a la IA al elegir una receta. @spec REQ-23
#[tauri::command]
pub fn recipes_prompt(id: String) -> Result<String, crate::unreal::UnrealError> {
    find(&id).map(|recipe| prompt(&recipe)).ok_or_else(|| crate::unreal::UnrealError::NotFound(format!("unknown recipe {id}")))
}

/// Definiciones de las herramientas, las mismas que publica el MCP propio (D10). @spec REQ-23
#[tauri::command]
pub fn recipes_tool_specs() -> Vec<Value> {
    specs()
}

/// Una herramienta de recetas desde el asistente integrado; `endpoint` hace falta para `recipe_check`. @spec REQ-23
#[tauri::command]
pub async fn recipes_run_tool(app: tauri::AppHandle, endpoint: Option<crate::unreal::commands::Endpoint>, name: String, args: Value) -> Result<String, crate::unreal::UnrealError> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        run_tool(&name, &args, |code| {
            let endpoint = endpoint.ok_or("no editor is connected: connect it in the Project view")?;
            crate::unreal::commands::run_fixed_python(&app.state::<crate::unreal::commands::UnrealState>(), &endpoint.url(), code).map_err(|error| error.to_string())
        })
        .map_err(crate::unreal::UnrealError::Format)
    })
    .await
    .map_err(|error| crate::unreal::UnrealError::Io(error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec REQ-23
    #[test]
    fn las_cinco_recetas_con_textos_en_espanol_ingles_y_chino() {
        let recipes = all();
        let ids: Vec<&str> = recipes.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["test_level", "tripo_character", "world_walk", "convai_metahuman", "runtime_diagnosis"]);
        for recipe in &recipes {
            for lang in ["es", "en", "zh-CN"] {
                assert!(!text(&recipe.title, lang).is_empty() && !text(&recipe.summary, lang).is_empty(), "{} sin {lang}", recipe.id);
            }
            assert!(recipe.needs.contains(&"editor".to_string()));
            assert!(recipe.id.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "el id va tal cual al código Python");
        }
        assert_eq!(text(&recipes[0].title, "fr"), "Test level", "sin traducción, inglés");
    }

    /// El código de comprobación es fijo: un id desconocido no llega al editor. @spec REQ-23, R-16
    #[test]
    fn la_comprobacion_es_codigo_fijo_y_rechaza_ids_desconocidos() {
        assert_eq!(check_code("test_level").unwrap(), "from dbv_unreal_ai import recipes\nresult = recipes.check(\"test_level\")");
        assert!(check_code("x\"); import os; os.remove(\"y").unwrap_err().contains("unknown recipe"));
    }

    /// @spec REQ-23
    #[test]
    fn el_informe_dice_que_falta_y_que_hacer_con_un_toolset_viejo() {
        let ok = json!({ "success": true, "result": { "passed": false, "detail": "missing: navmesh", "items": [
            { "id": "floor", "ok": true, "detail": "floor: Floor" }, { "id": "navmesh", "ok": false, "detail": "0 NavMeshBoundsVolume" }
        ] } });
        assert_eq!(report_text(&ok), "missing: navmesh\n✓ floor: floor: Floor\n✗ navmesh: 0 NavMeshBoundsVolume");
        let old = json!({ "success": false, "error": "ImportError: cannot import name 'recipes' from 'dbv_unreal_ai'" });
        assert!(report_text(&old).contains("Prepare project"));
    }

    /// @spec REQ-23
    #[test]
    fn el_prompt_y_las_herramientas_llevan_el_id_y_la_comprobacion() {
        let recipe = find("world_walk").unwrap();
        let text = prompt(&recipe);
        assert!(text.contains("recipe_check with id \"world_walk\"") && text.contains(&recipe.steps));
        let specs = specs();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[1]["inputSchema"]["properties"]["id"]["enum"].as_array().unwrap().len(), 5);
        assert!(list_text().contains("convai_metahuman"));
    }

    /// @spec REQ-23
    #[test]
    fn las_herramientas_se_ejecutan_igual_en_las_dos_superficies() {
        assert!(run_tool("recipes_list", &json!({}), |_| unreachable!()).unwrap().contains("test_level"));
        let ran = run_tool("recipe_check", &json!({ "id": "test_level" }), |code| {
            assert!(code.ends_with("recipes.check(\"test_level\")"));
            Ok(json!({ "success": true, "result": { "detail": "all done", "items": [] } }))
        });
        assert_eq!(ran.unwrap(), "all done
");
        assert!(run_tool("recipe_check", &json!({ "id": "nope" }), |_| unreachable!()).unwrap_err().contains("unknown recipe"));
    }
}
