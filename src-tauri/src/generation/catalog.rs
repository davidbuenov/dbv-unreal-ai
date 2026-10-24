// =============================================================================
// DBV Unreal AI — Catálogo de operaciones de la API v3 de Tripo
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-33, REQ-15, REQ-16, RNF-04
//
// Cada punto de la API que crea una tarea (https://developers.tripo3d.ai/es/docs, leída el 2026-10-24)
// es una `Operation`: su ruta, qué produce, qué parámetros admite y cuánto cuesta según la tabla de
// precios oficial (`/es/pricing`, 1 crédito = 0,01 USD). El MCP propio y la interfaz usan este catálogo
// para lanzar cualquier operación con la misma confirmación de coste y la misma consola (REQ-27); así
// la IA no vuelve a responder «esta herramienta no lo expone» (hallazgo del usuario con Claude Code).
//
// Los campos que reciben un modelo o una imagen (`input`, `inputs`, `ref_image`) aceptan además un id de
// la biblioteca o una ruta local: el trabajo los traduce a `task_…` o los sube con `/v3/files`.

use serde::Serialize;
use serde_json::Value;

/// Una operación de Tripo que crea una tarea asíncrona.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// Nombre estable para herramientas e interfaz (`text_to_model`).
    pub name: &'static str,
    /// Ruta relativa a `https://openapi.tripo3d.ai/v3`.
    pub endpoint: &'static str,
    /// Qué deja en la biblioteca: `model`, `rig`, `animation`, `image`, `multiview`, `splat`, `segments`, o `none`.
    pub produces: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    /// Parámetros documentados, para el modelo (`tripo_operations`).
    pub params: &'static str,
    /// Cómo se calcula el coste (texto para el modelo y la tarjeta de coste).
    pub pricing: &'static str,
}

/// Todas las operaciones de la API v3 que crean una tarea.
pub const OPERATIONS: [Operation; 18] = [
    Operation {
        name: "text_to_model",
        endpoint: "/generation/text-to-model",
        produces: "model",
        category: "generation",
        summary: "Text → 3D model (GLB). Series H by default; Series P (model P1-20260311 / P2-20260801) for clean low poly.",
        params: "prompt (required, ≤1024 chars, English works best), negative_prompt, model (v3.1-20260211 default | v3.0-20250812 | v2.5-20250123 | P1-20260311 | P2-20260801), face_limit, texture (default true), pbr (default true), texture_quality (standard | detailed | extreme | fast), geometry_quality (standard | detailed), smart_low_poly, quad (forces FBX), generate_parts (needs texture=false and pbr=false), auto_size, model_seed, texture_seed, image_seed, export_orientation",
        pricing: "10 without texture, 20 with texture; +10 detailed texture, +20 extreme texture, +20 detailed geometry, +5 quad, +10 smart_low_poly, +20 generate_parts",
    },
    Operation {
        name: "image_to_model",
        endpoint: "/generation/image-to-model",
        produces: "model",
        category: "generation",
        summary: "One reference image → 3D model. `input` is a local image path, a library id, an http(s) URL, a file_ token or a task_ id.",
        params: "input (required: image), model, face_limit, texture, pbr, texture_quality, geometry_quality, texture_alignment (original_image | geometry), orientation (default | align_image), enable_image_autofix, smart_low_poly, quad, generate_parts, auto_size, texture_seed",
        pricing: "20 without texture, 30 with texture; same add-ons as text_to_model",
    },
    Operation {
        name: "multiview_to_model",
        endpoint: "/generation/multiview-to-model",
        produces: "model",
        category: "generation",
        summary: "Several views of the same object (front required + at least one of left/back/right) → 3D model.",
        params: "inputs (required: array of {\"front\": image}, {\"left\": image}, {\"back\": image}, {\"right\": image}; each image a local path, URL or file_ token; or [{\"task_id\": …}] of an image_to_multiview task), model, face_limit, texture, pbr, texture_quality, geometry_quality, smart_low_poly, quad, generate_parts",
        pricing: "20 without texture, 30 with texture; same add-ons as text_to_model",
    },
    Operation {
        name: "image_to_splat",
        endpoint: "/generation/image-to-splat",
        produces: "splat",
        category: "generation",
        summary: "One image → Gaussian splat (.splat). About 4 minutes.",
        params: "input (required: image), model_seed",
        pricing: "30",
    },
    Operation {
        name: "text_to_image",
        endpoint: "/generation/text-to-image",
        produces: "image",
        category: "image",
        summary: "Text → image (concept art, reference for image_to_model). Templates such as t_pose or asset_extraction prepare an image for 3D.",
        params: "prompt (required), model (seedream_v4 default | seedream_v5 | banana | banana2 | banana_pro | chat_image_2 | chat_image_2.5_flare | chat_image_2.5_sunburst), quality (low | medium | high | xhigh | max), template (asset_extraction | character_completion | t_pose | head_extraction | 3d_enhance | variants | print_clay | figure), t_pose, sketch_to_render, background",
        pricing: "5 (seedream, banana); 10 banana2 / chat_image_2*; 15 banana_pro; high quality 20+",
    },
    Operation {
        name: "image_to_image",
        endpoint: "/generation/image-to-image",
        produces: "image",
        category: "image",
        summary: "Edit or restyle an image (or several references) with a prompt or a template (t_pose, asset_extraction…).",
        params: "input (required: image), inputs (more references), prompt (required without template), model (seedream_v5 default …), quality, template, t_pose, sketch_to_render, background",
        pricing: "5 (seedream, banana); 10 banana2 / chat_image_2*; 15 banana_pro; high quality 20+",
    },
    Operation {
        name: "image_to_multiview",
        endpoint: "/generation/image-to-multiview",
        produces: "multiview",
        category: "image",
        summary: "One image → front/left/back/right views (then multiview_to_model with inputs=[{\"task_id\": …}]).",
        params: "input (required: image)",
        pricing: "10",
    },
    Operation {
        name: "edit_multiview",
        endpoint: "/generation/edit-multiview",
        produces: "multiview",
        category: "image",
        summary: "Edit a multiview image.",
        params: "input (required: image)",
        pricing: "5 per edited image",
    },
    Operation {
        name: "texture",
        endpoint: "/models/texture",
        produces: "model",
        category: "processing",
        summary: "Re-texture an existing model.",
        params: "input (required: model), model (v3.0-20250812 default | v3.5-20260815 | v2.5-20250123), texture_quality (standard | detailed | extreme | fast), pbr, texture_seed, delight",
        pricing: "10 standard/fast, 20 detailed, 30 extreme",
    },
    Operation {
        name: "convert",
        endpoint: "/models/convert",
        produces: "model",
        category: "processing",
        summary: "Convert a model to another format (FBX for Unreal, USDZ, OBJ, STL, GLTF, 3MF), with optional quad remesh, pivot to bottom center, scale, texture size.",
        params: "input (required: model), format (required: GLTF | USDZ | FBX | OBJ | STL | 3MF), quad, force_symmetry, face_limit, flatten_bottom, texture_size, texture_format, bake, pack_uv, pivot_to_center_bottom, scale_factor, with_animation, animate_in_place, part_names, export_orientation, fbx_preset (blender | 3dsmax | mixamo | bake_scale)",
        pricing: "5 basic; 10 with any of quad, face_limit, flatten_bottom, texture_size, texture_format, pivot_to_center_bottom, scale_factor",
    },
    Operation {
        name: "retopology",
        endpoint: "/mesh/decimate",
        produces: "model",
        category: "processing",
        summary: "Retopology / decimation to a polygon budget, baking the textures.",
        params: "input (required: model), model (v2.0 default | v3.0 | v1.0), face_limit (v2/v3: 500-20000 triangles, 500-10000 quads; required for v1.0), quad, bake (default true), smart_uv (v3.0 only), part_names",
        pricing: "30 v2.0; 35 v3.0 triangles; 40 v3.0 quads; 10 v1.0",
    },
    Operation {
        name: "smart_uv",
        endpoint: "/mesh/uv",
        produces: "model",
        category: "processing",
        summary: "New UV layout, baking the existing textures onto it.",
        params: "input (required: model), bake (default true), export_orientation",
        pricing: "20",
    },
    Operation {
        name: "segment",
        endpoint: "/mesh/segment",
        produces: "segments",
        category: "processing",
        summary: "Split a model into named parts (then part_completion or convert with part_names).",
        params: "input (required: model), model (v1.0-20250506 default | v2.0-20260430), segmentation_granularity (simple | balanced | detailed, v2), split_by_connectivity (v2), ref_image (v2)",
        pricing: "40",
    },
    Operation {
        name: "part_completion",
        endpoint: "/mesh/complete",
        produces: "model",
        category: "processing",
        summary: "Complete the hidden geometry of the parts of a segment task.",
        params: "input (required: task id of a segment result), part_names, completion_mode (ai_completion default | quick_cap)",
        pricing: "50 ai_completion; 30 quick_cap",
    },
    Operation {
        name: "smart_segment",
        endpoint: "/mesh/smartsegment",
        produces: "segments",
        category: "processing",
        summary: "End to end: an image or a GLB → segmented parts.",
        params: "seg_type (required: image | model), input (required), granularity (coarse | medium | fine), hint, transform (16 floats, required for seg_type=model)",
        pricing: "85 image; 55 model",
    },
    Operation {
        name: "rig_check",
        endpoint: "/animations/rig-check",
        produces: "none",
        category: "animation",
        summary: "Can this model be rigged, and with which rig_type? Free. Call it before rig_model when unsure.",
        params: "input (required: model)",
        pricing: "free",
    },
    Operation {
        name: "rig",
        endpoint: "/animations/rig",
        produces: "rig",
        category: "animation",
        summary: "Automatic skeleton + skinning. For the 101 biped presets (dances, emotions, sports…) use model v1.0-20240301 with spec tripo; for an Unreal-named humanoid skeleton use model v3.0-20260909 with spec unreal (16 generic presets).",
        params: "input (required: model), model (v1.0-20240301 default, bipeds | v2.5-20260210 creatures | v3.0-20260909 humanoids), rig_type (biped | quadruped | hexapod | octopod | avian | serpentine | aquatic | others), spec (tripo | mixamo | actorcore | unity | unreal | vrm; bipeds only), out_format (glb | fbx)",
        pricing: "25",
    },
    Operation {
        name: "retarget",
        endpoint: "/animations/retarget",
        produces: "animation",
        category: "animation",
        summary: "Apply preset animations to a rigged model (task of a rig). Rig v1.0 has 101 biped presets (preset:biped:dance_01, …); rigs v2/v3 have 16 (preset:idle, preset:walk, …).",
        params: "input (required: rig), animation (one preset) or animations (up to 5), out_format (glb | fbx), export_with_geometry (default true), animate_in_place",
        pricing: "10 per animation",
    },
];

/// Animaciones de un rig `v1.0-20240301` (bípedo, `spec: tripo`): 101 presets `preset:biped:<nombre>`, por grupos.
pub const BIPED_PRESETS: [(&str, &str); 7] = [
    ("basic", "idle walk run run_upstairs turn jump jump_down jump_rope_01 jump_rope_02 fall climb dive swim surf flip sit standing_relax look_around wait swagger"),
    ("dance", "dance_01 dance_02 dance_03 dance_04 dance_05 dance_06 freaky"),
    ("performance", "sing_01 sing_02 sing_03 sing_04 cheer"),
    ("emotion", "clap bow greet_01 greet_02 greet_03 greet_04 wave_goodbye_01 wave_goodbye_02 agree heart_pose hug fold_arms laugh_01 laugh_02 cry sob afraid frightened scared_01 scared_02 angry_01 angry_02 angry_03 depressed frustrated_01 frustrated_02 complain_01 complain_02 victory_celebration defeat_02 defeat_03"),
    ("daily", "scratch make_a_call_01 make_a_call_02 play_mobile_game play_video_game dig chop shovel lift_heavy"),
    ("sports", "warm_up press-up cross_body_crunch basketball_shot crossover_dribble dribble pitch_baseball golf volleyball football_catch football_save football_pass"),
    ("combat", "box_01 box_02 box_03 front_kick_01 front_kick_02 slash shoot fire cast_a_spell hurt hit_to_body_01 hit_to_body_02 hit_to_head hit_to_side hit_to_stomach flee_01 flee_02"),
];

/// Animaciones de los rigs v2.0/v2.5/v3.0 (cualquier tipo): 16 presets `preset:<nombre>`.
pub const GENERIC_PRESETS: [&str; 16] = ["idle", "walk", "run", "dive", "climb", "jump", "slash", "shoot", "hurt", "fall", "turn", "quadruped:walk", "hexapod:walk", "octopod:walk", "serpentine:march", "aquatic:march"];

/// ¿El rig se hizo con el modelo v1.0 (el de los 101 presets bípedos)? Sin dato, se supone que sí (es el defecto).
pub fn is_v1_rig(rig_request: &Value) -> bool {
    rig_request.get("model").and_then(Value::as_str).is_none_or(|m| m.starts_with("v1.0"))
}

/// Nombre completo de una animación para un rig: acepta `dance_01`, `preset:biped:dance_01` o `preset:walk`.
pub fn preset_id(name: &str, v1_rig: bool) -> Result<String, String> {
    let short = name.trim().trim_start_matches("preset:").trim_start_matches("biped:");
    let valid = if v1_rig { BIPED_PRESETS.iter().any(|(_, list)| list.split(' ').any(|n| n == short)) } else { GENERIC_PRESETS.contains(&short) };
    let id = if v1_rig { format!("preset:biped:{short}") } else { format!("preset:{short}") };
    if valid {
        Ok(id)
    } else {
        Err(format!("«{name}» is not an animation of this rig. {}", preset_help(v1_rig)))
    }
}

/// Lista de animaciones para el modelo.
pub fn preset_help(v1_rig: bool) -> String {
    if v1_rig {
        let groups: Vec<String> = BIPED_PRESETS.iter().map(|(group, list)| format!("{group}: {list}")).collect();
        format!("Rig v1.0 biped animations ({}): {}", 101, groups.join(" | "))
    } else {
        format!("Rig v2/v3 animations: {}", GENERIC_PRESETS.join(", "))
    }
}

/// Operación por nombre.
pub fn find(name: &str) -> Option<&'static Operation> {
    OPERATIONS.iter().find(|op| op.name == name)
}

fn flag(params: &Value, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn text(params: &Value, key: &str) -> String {
    params.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Suplementos de las generaciones de modelo (tabla «Add-ons» de los precios).
fn model_addons(params: &Value) -> f64 {
    let quality = match text(params, "texture_quality").as_str() {
        "detailed" => 10.0,
        "extreme" => 20.0,
        _ => 0.0,
    };
    let geometry = if text(params, "geometry_quality") == "detailed" { 20.0 } else { 0.0 };
    let quad = if flag(params, "quad") { 5.0 } else { 0.0 };
    let low_poly = if flag(params, "smart_low_poly") { 10.0 } else { 0.0 };
    let parts = if flag(params, "generate_parts") { 20.0 } else { 0.0 };
    quality + geometry + quad + low_poly + parts
}

/// ¿Lleva textura? Por defecto sí; `pbr` la fuerza; `generate_parts` exige quitarla.
fn textured(params: &Value) -> bool {
    params.get("texture").and_then(Value::as_bool).unwrap_or(true) || flag(params, "pbr")
}

fn image_model_cost(model: &str, quality: &str) -> f64 {
    let base: f64 = match model {
        m if m.starts_with("banana_pro") => 15.0,
        m if m.starts_with("banana2") || m.starts_with("chat_image_2") || m == "chat_image_1.5" => 10.0,
        _ => 5.0,
    };
    let high = match quality {
        "high" => 20.0,
        "xhigh" => 25.0,
        "max" => 30.0,
        _ => 0.0,
    };
    base.max(high)
}

/// Créditos estimados de una operación con estos parámetros (tabla de precios oficial). Es una estimación
/// para avisar ANTES de gastar; el real llega con la tarea (`credits_consumed`) y el saldo lo confirma.
pub fn estimate(op: &Operation, params: &Value) -> f64 {
    let animations = params.get("animations").and_then(Value::as_array).map(|a| a.len()).unwrap_or(1).max(1);
    let advanced_convert = ["quad", "flatten_bottom", "pivot_to_center_bottom"].iter().any(|k| flag(params, k)) || ["face_limit", "texture_size", "texture_format", "scale_factor"].iter().any(|k| params.get(*k).is_some());
    match op.name {
        "text_to_model" => (if textured(params) { 20.0 } else { 10.0 }) + model_addons(params),
        "image_to_model" | "multiview_to_model" => (if textured(params) { 30.0 } else { 20.0 }) + model_addons(params),
        "image_to_splat" => 30.0,
        "text_to_image" | "image_to_image" => image_model_cost(&text(params, "model"), &text(params, "quality")),
        "image_to_multiview" => 10.0,
        "edit_multiview" => 5.0,
        "texture" => match text(params, "texture_quality").as_str() {
            "detailed" => 20.0,
            "extreme" => 30.0,
            _ => 10.0,
        },
        "convert" => {
            if advanced_convert {
                10.0
            } else {
                5.0
            }
        }
        "retopology" => match (text(params, "model").as_str(), flag(params, "quad")) {
            ("v3.0", true) => 40.0,
            ("v3.0", false) => 35.0,
            ("v1.0", _) => 10.0,
            _ => 30.0,
        },
        "smart_uv" => 20.0,
        "segment" => 40.0,
        "part_completion" => {
            if text(params, "completion_mode") == "quick_cap" {
                30.0
            } else {
                50.0
            }
        }
        "smart_segment" => {
            if text(params, "seg_type") == "model" {
                55.0
            } else {
                85.0
            }
        }
        "rig" => 25.0,
        "retarget" => 10.0 * animations as f64,
        _ => 0.0,
    }
}

/// Campos que llevan una fuente (modelo o imagen) y se resuelven antes de enviar.
pub const SOURCE_FIELDS: [&str; 3] = ["input", "inputs", "ref_image"];

/// Comprueba lo mínimo antes de gastar: los campos obligatorios. Devuelve el error para el modelo.
pub fn validate(op: &Operation, params: &Value) -> Result<(), String> {
    let has = |key: &str| params.get(key).is_some_and(|v| !v.is_null() && v.as_str() != Some(""));
    let missing: Vec<&str> = match op.name {
        "text_to_model" | "text_to_image" => ["prompt"].into_iter().filter(|k| !has(k)).collect(),
        "multiview_to_model" => ["inputs"].into_iter().filter(|k| !has(k)).collect(),
        "convert" => ["input", "format"].into_iter().filter(|k| !has(k)).collect(),
        "smart_segment" => ["input", "seg_type"].into_iter().filter(|k| !has(k)).collect(),
        "retarget" => {
            if has("input") && (has("animation") || has("animations")) {
                vec![]
            } else {
                vec!["input", "animation|animations"]
            }
        }
        _ => ["input"].into_iter().filter(|k| !has(k)).collect(),
    };
    let too_many = op.name == "retarget" && params.get("animations").and_then(Value::as_array).is_some_and(|a| a.len() > 5);
    if !missing.is_empty() {
        return Err(format!("{} needs: {}. Parameters: {}", op.name, missing.join(", "), op.params));
    }
    if too_many {
        return Err("retarget accepts at most 5 animations per task".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Los precios de la tabla oficial; el caso de Holo-Companion (texto + smart_low_poly) costó 30 de verdad.
    /// @spec RNF-04, REQ-33
    #[test]
    fn el_coste_sigue_la_tabla_oficial() {
        let text = find("text_to_model").unwrap();
        assert_eq!(estimate(text, &json!({ "prompt": "x" })), 20.0);
        assert_eq!(estimate(text, &json!({ "prompt": "x", "smart_low_poly": true })), 30.0, "medido en Holo-Companion");
        assert_eq!(estimate(text, &json!({ "prompt": "x", "texture": false })), 10.0);
        assert_eq!(estimate(text, &json!({ "prompt": "x", "texture_quality": "detailed", "geometry_quality": "detailed", "quad": true })), 55.0);
        assert_eq!(estimate(find("image_to_model").unwrap(), &json!({ "input": "a.png" })), 30.0);
        assert_eq!(estimate(find("rig").unwrap(), &json!({})), 25.0);
        assert_eq!(estimate(find("retarget").unwrap(), &json!({ "animations": ["preset:biped:dance_01", "preset:biped:wave_goodbye_01", "preset:biped:idle"] })), 30.0);
        assert_eq!(estimate(find("rig_check").unwrap(), &json!({})), 0.0);
        assert_eq!(estimate(find("convert").unwrap(), &json!({ "format": "FBX" })), 5.0);
        assert_eq!(estimate(find("convert").unwrap(), &json!({ "format": "FBX", "pivot_to_center_bottom": true })), 10.0);
    }

    /// Las 101 animaciones del rig v1.0 (bailes incluidos) y las 16 de los rigs nuevos. @spec REQ-16
    #[test]
    fn traduce_y_valida_los_nombres_de_animacion() {
        assert_eq!(BIPED_PRESETS.iter().map(|(_, l)| l.split(' ').count()).sum::<usize>(), 101);
        assert_eq!(preset_id("dance_01", true).unwrap(), "preset:biped:dance_01");
        assert_eq!(preset_id("preset:biped:wave_goodbye_01", true).unwrap(), "preset:biped:wave_goodbye_01");
        assert_eq!(preset_id("walk", false).unwrap(), "preset:walk");
        assert!(preset_id("dance_01", false).unwrap_err().contains("Rig v2/v3"), "un rig v3 no tiene bailes");
        assert!(preset_id("moonwalk", true).is_err());
        assert!(is_v1_rig(&json!({ "model": "v1.0-20240301" })) && is_v1_rig(&json!({})) && !is_v1_rig(&json!({ "model": "v3.0-20260909" })));
    }

    /// @spec REQ-33
    #[test]
    fn cubre_toda_la_api_que_crea_tareas_y_valida_lo_obligatorio() {
        let names: Vec<&str> = OPERATIONS.iter().map(|op| op.name).collect();
        for expected in ["text_to_model", "image_to_model", "multiview_to_model", "rig", "retarget", "rig_check", "convert", "texture", "retopology", "segment"] {
            assert!(names.contains(&expected), "{expected}");
        }
        assert_eq!(names.len(), names.iter().collect::<std::collections::HashSet<_>>().len(), "nombres únicos");
        assert!(validate(find("convert").unwrap(), &json!({ "input": "task_1" })).unwrap_err().contains("format"));
        assert!(validate(find("retarget").unwrap(), &json!({ "input": "task_1", "animations": ["a", "b", "c", "d", "e", "f"] })).unwrap_err().contains("5"));
        assert!(validate(find("rig").unwrap(), &json!({ "input": "pirata-1" })).is_ok());
    }
}
