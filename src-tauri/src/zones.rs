// =============================================================================
// DBV Unreal AI — Zonas del nivel para los personajes
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-57
//
// Una zona es un sitio del nivel con nombre y descripción («la cocina: la mesa larga y el horno de leña») al que un
// personaje Convai puede referirse e ir. En el editor es un `TargetPoint` (existe en Play, al contrario que un actor
// solo de editor) etiquetado `DBV_Zone_<Nombre>`, en la carpeta `DBV/Zones`, con su nombre, descripción y radio en
// tags (`dbv_zone_*`) y, si el plugin está, un `ConvaiObjectComponent` con `object_entry.name`/`description`: cada
// componente se registra solo al empezar Play y todos los chatbots del nivel lo ven (P-11, verificado en vivo el
// 2026-10-31, igual que deshacerlo). El código Python es FIJO de la app y las zonas viajan como dato en hexadecimal,
// como las comprobaciones de tareas (REQ-37, D28): la IA nunca escribe ese código. El nivel es la fuente de verdad.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Zonas como mucho en una propuesta.
pub const MAX_ZONES: usize = 20;
/// Etiqueta común de los actores de zona.
pub const TAG: &str = "dbv_zone";
/// Carpeta del World Outliner.
pub const FOLDER: &str = "DBV/Zones";

/// Una zona validada, con la posición ya en centímetros (unidades de Unreal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub name: String,
    pub description: String,
    /// Etiqueta del actor en el Outliner (`DBV_Zone_<Nombre>`).
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Radio de llegada en metros.
    pub radius: f64,
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Las dos herramientas de zonas, iguales para el asistente integrado y el MCP propio (D10).
pub fn specs() -> Vec<Value> {
    vec![
        tool(
            "level_zones_propose",
            "Propose named zones of the open level (\"Kitchen: the long table and the wood oven\") so characters know these places and can walk to them, and so you can place things \"in the kitchen\". Base them on unreal_scene_summary and, if you can see, a viewport capture. Positions in METERS in world coordinates (Unreal X, Y; Z of the floor). The user reviews and edits the list before anything changes; then the app creates (or updates, by name) one marker per zone in the Outliner folder DBV/Zones, with Convai scene metadata when the Convai plugin is installed. Zones are part of the level: undo works. Write names short (one or two words) and descriptions in the user's language.",
            json!({
                "zones": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_ZONES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "description": { "type": "string", "description": "What is there, so a character can talk about it." },
                            "x": { "type": "number", "description": "Meters." },
                            "y": { "type": "number", "description": "Meters." },
                            "z": { "type": "number", "description": "Meters (floor height; default 0)." },
                            "radius": { "type": "number", "description": "Meters a character must get close to count as arrived (default 2)." }
                        },
                        "required": ["name", "description", "x", "y"]
                    }
                }
            }),
            &["zones"],
        ),
        tool(
            "level_zones_list",
            "List the zones of the open level (name, description, position in meters, radius, and whether Convai sees them). Read-only.",
            json!({}),
            &[],
        ),
    ]
}

/// ¿Es una de las herramientas de zonas?
pub fn is_zone_tool(name: &str) -> bool {
    name == "level_zones_propose" || name == "level_zones_list"
}

/// Etiqueta de actor válida a partir del nombre de la zona.
pub fn label_for(name: &str) -> String {
    let clean: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let trimmed = clean.split('_').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("_");
    format!("DBV_Zone_{}", if trimmed.is_empty() { "Zone".to_string() } else { trimmed })
}

/// Metros a centímetros, redondeado a la décima de centímetro.
fn cm(meters: f64) -> f64 {
    (meters * 1000.0).round() / 10.0
}

fn number(item: &Value, key: &str) -> Option<f64> {
    item.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

/// Comprueba y normaliza una propuesta: nombres únicos, textos acotados, metros → centímetros. @spec REQ-57
pub fn validate(args: &Value) -> Result<Vec<Zone>, String> {
    let items = args.get("zones").and_then(Value::as_array).ok_or("zones must be a list")?;
    if items.is_empty() || items.len() > MAX_ZONES {
        return Err(format!("propose between 1 and {MAX_ZONES} zones"));
    }
    let mut zones: Vec<Zone> = Vec::new();
    let mut problems = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let name: String = item.get("name").and_then(Value::as_str).unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect();
        let description: String = item.get("description").and_then(Value::as_str).unwrap_or("").trim().chars().take(500).collect();
        let (x, y) = (number(item, "x"), number(item, "y"));
        let label = label_for(&name);
        if name.is_empty() || x.is_none() || y.is_none() {
            problems.push(format!("zone {}: name, x and y are required", i + 1));
        } else if zones.iter().any(|z| z.label.eq_ignore_ascii_case(&label)) {
            problems.push(format!("zone {}: the name \"{name}\" is repeated", i + 1));
        } else {
            let radius = number(item, "radius").unwrap_or(2.0).clamp(0.5, 100.0);
            zones.push(Zone { name, description, label, x: cm(x.unwrap_or(0.0)), y: cm(y.unwrap_or(0.0)), z: cm(number(item, "z").unwrap_or(0.0)), radius });
        }
    }
    if problems.is_empty() {
        Ok(zones)
    } else {
        Err(problems.join("; "))
    }
}

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

/// Código que crea o actualiza las zonas en el editor. FIJO: las zonas son dato. @spec REQ-57
pub fn apply_code(zones: &[Zone]) -> String {
    let data = hex(&serde_json::to_string(zones).unwrap_or_else(|_| "[]".into()));
    format!(
        r#"import json
import unreal
_zones = json.loads(bytes.fromhex("{data}").decode("utf-8"))
_eas = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
_tag = unreal.Name("{TAG}")
_existing = {{a.get_actor_label(): a for a in _eas.get_all_level_actors() if _tag in a.tags}}
_has_convai = hasattr(unreal, "ConvaiObjectComponent")
_sds = unreal.get_engine_subsystem(unreal.SubobjectDataSubsystem) if _has_convai else None
_lib = unreal.SubobjectDataBlueprintFunctionLibrary
_report = []
for _z in _zones:
    _actor = _existing.get(_z["label"])
    _created = _actor is None
    _where = unreal.Vector(_z["x"], _z["y"], _z["z"])
    if _created:
        _actor = _eas.spawn_actor_from_class(unreal.TargetPoint, _where)
        _actor.set_actor_label(_z["label"])
    else:
        _actor.set_actor_location(_where, False, False)
    _actor.set_folder_path("{FOLDER}")
    _actor.tags = [_tag, unreal.Name("{TAG}_name=" + _z["name"]), unreal.Name("{TAG}_desc=" + _z["description"][:900]), unreal.Name("{TAG}_radius=" + str(_z["radius"]))]
    _seen = False
    if _has_convai:
        _comps = _actor.get_components_by_class(unreal.ConvaiObjectComponent)
        _comp = _comps[0] if _comps else None
        if _comp is None:
            _handles = _sds.k2_gather_subobject_data_for_instance(_actor)
            _added = _sds.add_new_subobject(unreal.AddNewSubobjectParams(parent_handle=_handles[0], new_class=unreal.ConvaiObjectComponent, blueprint_context=None))
            _handle = _added[0] if isinstance(_added, tuple) else _added
            _comp = _lib.get_associated_object(_lib.get_data(_handle))
        if _comp is not None:
            _entry = _comp.get_editor_property("object_entry")
            _entry.set_editor_property("name", _z["name"])
            _entry.set_editor_property("description", _z["description"])
            _entry.set_editor_property("acceptance_radius", float(_z["radius"]) * 100.0)
            _comp.set_editor_property("object_entry", _entry)
            _seen = True
    _report.append({{"name": _z["name"], "label": _z["label"], "created": _created, "convai": _seen}})
result = {{"zones": _report, "convai": _has_convai}}
"#
    )
}

/// Código de solo lectura que lista las zonas del nivel abierto. @spec REQ-57
pub fn list_code() -> String {
    format!(
        r#"import unreal
_tag = unreal.Name("{TAG}")
_has_convai = hasattr(unreal, "ConvaiObjectComponent")
_zones = []
for _a in unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors():
    if _tag in _a.tags:
        _meta = dict(str(_t).split("=", 1) for _t in _a.tags if "=" in str(_t))
        _loc = _a.get_actor_location()
        _convai = bool(_has_convai and _a.get_components_by_class(unreal.ConvaiObjectComponent))
        _zones.append({{"label": _a.get_actor_label(), "name": _meta.get("{TAG}_name", _a.get_actor_label()), "description": _meta.get("{TAG}_desc", ""), "x_m": round(_loc.x / 100.0, 2), "y_m": round(_loc.y / 100.0, 2), "z_m": round(_loc.z / 100.0, 2), "radius_m": float(_meta.get("{TAG}_radius", "2")), "convai": _convai}})
result = {{"zones": _zones, "convai_installed": _has_convai}}
"#
    )
}

/// Texto para la IA con el resultado de crear o listar (lo que devolvió `run_python`).
pub fn report_text(value: &Value) -> String {
    let result = value.get("result").cloned().unwrap_or(Value::Null);
    let text = match value.get("success").and_then(Value::as_bool) {
        Some(true) => result.to_string(),
        _ => format!("error: {}", value.get("error").and_then(Value::as_str).unwrap_or("the editor did not run the zones code")),
    };
    text
}

/// Ejecuta código fijo de zonas en el editor conectado con el `run_python` del toolset DBV.
fn run_in_editor(app: &tauri::AppHandle, endpoint: &crate::unreal::commands::Endpoint, code: &str) -> Result<Value, crate::unreal::UnrealError> {
    use tauri::Manager;
    crate::unreal::commands::run_fixed_python(&app.state::<crate::unreal::commands::UnrealState>(), &endpoint.url(), code)
}

/// Definiciones de las herramientas de zonas, las mismas que publica el MCP propio (D10). @spec REQ-57
#[tauri::command]
pub fn zones_tool_specs() -> Vec<Value> {
    specs()
}

/// La propuesta de la IA validada y en centímetros, para la tarjeta de zonas (no toca el editor). @spec REQ-57
#[tauri::command]
pub fn zones_preview(args: Value) -> Result<Vec<Zone>, crate::unreal::UnrealError> {
    validate(&args).map_err(crate::unreal::UnrealError::Format)
}

/// Crea o actualiza las zonas que la persona aceptó en la tarjeta (D29). @spec REQ-57
#[tauri::command]
pub async fn zones_apply(app: tauri::AppHandle, endpoint: crate::unreal::commands::Endpoint, zones: Vec<Zone>) -> Result<Value, crate::unreal::UnrealError> {
    tauri::async_runtime::spawn_blocking(move || run_in_editor(&app, &endpoint, &apply_code(&zones))).await.map_err(|error| crate::unreal::UnrealError::Io(error.to_string()))?
}

/// Las zonas del nivel abierto (solo lectura). @spec REQ-57
#[tauri::command]
pub async fn zones_list(app: tauri::AppHandle, endpoint: crate::unreal::commands::Endpoint) -> Result<Value, crate::unreal::UnrealError> {
    tauri::async_runtime::spawn_blocking(move || run_in_editor(&app, &endpoint, &list_code())).await.map_err(|error| crate::unreal::UnrealError::Io(error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Metros a centímetros, nombres únicos y acotados, radio por defecto. @spec REQ-57
    #[test]
    fn valida_y_normaliza_las_zonas() {
        let zones = validate(&json!({ "zones": [
            { "name": "  La   cocina ", "description": "La mesa larga y el horno de leña", "x": 3.5, "y": -2, "z": 0.1 },
            { "name": "Puerta norte", "description": "Salida al bosque", "x": 0, "y": 12, "radius": 500 }
        ] }))
        .unwrap();
        assert_eq!(zones[0].name, "La cocina");
        assert_eq!(zones[0].label, "DBV_Zone_La_cocina");
        assert_eq!((zones[0].x, zones[0].y, zones[0].z, zones[0].radius), (350.0, -200.0, 10.0, 2.0));
        assert_eq!(zones[1].radius, 100.0, "radio acotado");
        assert!(validate(&json!({ "zones": [{ "name": "A", "description": "", "x": 0, "y": 0 }, { "name": "a", "description": "", "x": 1, "y": 1 }] })).unwrap_err().contains("repeated"));
        assert!(validate(&json!({ "zones": [{ "name": "", "x": 0, "y": 0 }] })).unwrap_err().contains("required"));
        assert!(validate(&json!({ "zones": [] })).is_err());
    }

    /// Las zonas viajan como dato: un nombre con comillas o código no se cuela en el Python. @spec REQ-57
    #[test]
    fn las_zonas_viajan_como_dato_y_no_como_codigo() {
        let zones = validate(&json!({ "zones": [{ "name": "x\")\nimport os; os.system(\"rm\")", "description": "\"\"\"", "x": 0, "y": 0 }] })).unwrap();
        let code = apply_code(&zones);
        assert!(code.contains("bytes.fromhex(\""));
        assert!(!code.contains("os.system") && !code.contains("rm\""), "{code}");
        assert!(code.contains("unreal.TargetPoint") && code.contains("ConvaiObjectComponent") && code.contains("DBV/Zones"));
        assert!(list_code().contains("get_all_level_actors") && !list_code().contains("spawn_actor"));
        assert!(specs().iter().all(|s| s["description"].as_str().unwrap().len() > 40));
    }

    /// El código para probarlo en el editor real a mano: `cargo test codigo_de_muestra -- --ignored --nocapture`.
    #[test]
    #[ignore = "imprime el código de muestra para el spike en vivo"]
    fn codigo_de_muestra() {
        let zones = validate(&json!({ "zones": [{ "name": "Cocina", "description": "La mesa larga y el horno de leña", "x": 3, "y": 1 }, { "name": "Puerta norte", "description": "Salida al bosque", "x": 0, "y": 8, "radius": 3 }] })).unwrap();
        println!("{}", apply_code(&zones));
    }
}
