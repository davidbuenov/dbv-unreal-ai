// =============================================================================
// DBV Unreal AI — Herramientas de generación para la IA (una sola definición)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-33, REQ-16, REQ-28, REQ-29, REQ-32, REQ-80, REQ-84
//
// Las herramientas de Tripo se definen AQUÍ y las publican dos superficies (D10): el servidor MCP propio
// (agentes como Claude Code) y el asistente integrado (modelos locales y de nube, vía comandos Tauri). Antes
// cada superficie tenía su copia y el agente solo veía `generate_3d_model`: respondió que no podía riggear
// aunque Tripo sí puede. Cada descripción dice qué hace, qué NO hace y qué herramienta usar en su lugar.
//
// `build` traduce los argumentos de una herramienta en una `Launch` (operación + parámetros + nombre +
// variantes + origen) sin gastar nada; quien la lanza confirma antes el coste total (RNF-04).

use serde_json::{json, Map, Value};

use super::catalog::{self, Operation, OPERATIONS};
use super::jobs::MAX_VARIANTS;
use crate::library::Library;

/// Una operación lista para lanzar.
#[derive(Debug, Clone)]
pub struct Launch {
    pub operation: &'static Operation,
    pub params: Value,
    pub name: String,
    pub variants: u32,
    pub parent: Option<String>,
}

impl Launch {
    /// Coste total estimado (cada variante es una tarea).
    pub fn total(&self) -> f64 {
        catalog::estimate(self.operation, &self.params) * f64::from(self.variants)
    }
}

/// Herramientas que lanzan una operación con coste.
pub const LAUNCHERS: [&str; 8] = [
    "generate_3d_model",
    "image_to_3d_model",
    "generate_metahuman_outfit",
    "rig_model",
    "animate_model",
    "tripo_run",
    "generate_world",
    "export_world_mesh",
];

/// Ranuras válidas de ropa en un Actor MetaHuman. @spec REQ-80
pub const METAHUMAN_SLOTS: [&str; 3] = ["Torso", "Legs", "Feet"];

/// Normaliza el nombre de la ranura (`Torso`, `Legs`, `Feet`). @spec REQ-80
pub fn normalize_garment_slot(slot: &str) -> Result<&'static str, String> {
    match slot.trim().to_ascii_lowercase().as_str() {
        "" | "torso" | "top" | "shirt" | "jacket" => Ok("Torso"),
        "legs" | "bottom" | "pants" | "trousers" | "skirt" => Ok("Legs"),
        "feet" | "shoes" | "boots" | "footwear" => Ok("Feet"),
        other => Err(format!("invalid MetaHuman garment slot «{other}»: expected Torso, Legs or Feet")),
    }
}

/// Tipos de prenda de la app. Sustituyen a la ranura en la interfaz. @spec REQ-84
pub const GARMENT_KINDS: [&str; 5] = ["top", "pants", "skirt", "dress", "shoes"];

/// Normaliza el tipo de prenda (admite los sinónimos habituales). @spec REQ-84
pub fn normalize_garment_kind(kind: &str) -> Result<&'static str, String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "top" | "shirt" | "jacket" | "blouse" | "sweater" | "torso" => Ok("top"),
        "pants" | "trousers" | "jeans" | "shorts" => Ok("pants"),
        "skirt" | "kilt" => Ok("skirt"),
        "dress" | "gown" => Ok("dress"),
        "shoes" | "boots" | "footwear" | "feet" => Ok("shoes"),
        other => Err(format!("invalid garment kind «{other}»: expected top, pants, skirt, dress or shoes")),
    }
}

/// Ranura del MetaHuman que ocupa la prenda. Un vestido arranca en `Torso` y, además, retira `Legs` (REQ-84). @spec REQ-84
pub fn slot_of_kind(kind: &str) -> &'static str {
    match kind {
        "pants" | "skirt" => "Legs",
        "shoes" => "Feet",
        _ => "Torso",
    }
}

/// Tipo de una entrada antigua que solo guardó la ranura: `Torso` → top, `Feet` → shoes y `Legs` → falda o pantalón
/// según lo que dijera su descripción. @spec REQ-84
pub fn infer_garment_kind(slot: &str, prompt: &str) -> &'static str {
    let lower = prompt.to_ascii_lowercase();
    match slot {
        "Feet" => "shoes",
        "Legs" if ["skirt", "kilt", "falda"].iter().any(|w| lower.contains(w)) => "skirt",
        "Legs" => "pants",
        _ => "top",
    }
}

/// Corte de la prenda (`male`, `female`, `neutral`); vacío = sin indicar. @spec REQ-84
pub fn normalize_garment_cut(cut: &str) -> Result<Option<&'static str>, String> {
    match cut.trim().to_ascii_lowercase().as_str() {
        "" => Ok(None),
        "male" | "man" | "men" | "masculine" | "hombre" => Ok(Some("male")),
        "female" | "woman" | "women" | "feminine" | "mujer" => Ok(Some("female")),
        "neutral" | "unisex" => Ok(Some("neutral")),
        other => Err(format!("invalid garment cut «{other}»: expected male, female or neutral")),
    }
}

/// Enriquece el prompt de una prenda para que Tripo genere solo la ropa hueca en A-pose simétrica, sin maniquí.
/// Cada tipo pide lo suyo (un vestido es una prenda entera sin cuerpo) y el corte entra en el texto. @spec REQ-80, REQ-84
pub fn build_garment_prompt(prompt: &str, kind: &str, cut: Option<&str>) -> String {
    let base = prompt.trim();
    let body = match kind {
        "pants" => format!("3D standalone pair of trousers ({base}) in symmetrical A-pose, hollow interior waist and two separate leg openings, no mannequin, no human body, no legs, no feet, clean quad-friendly topology"),
        "skirt" => format!("3D standalone skirt ({base}), a single layer of fabric, symmetrical, hollow waist opening and open hem, no lining, no mannequin, no human body, no legs, no torso, clean quad-friendly topology"),
        "dress" => format!("3D standalone full-length dress ({base}) in symmetrical A-pose, a single layer of fabric, hollow interior with open neckline, armholes or sleeve cuffs and open hem, no mannequin, no human body, no head, no hands, no legs, no feet, clean quad-friendly topology"),
        "shoes" => format!("3D standalone pair of footwear ({base}), symmetrical, hollow ankle opening, no mannequin, no human feet or legs, clean quad-friendly topology"),
        _ => format!("3D standalone upper-body garment ({base}) in symmetrical A-pose, hollow interior, open neck Collar and open sleeve cuffs, no mannequin, no human body, no head, no hands, clean quad-friendly topology"),
    };
    match cut {
        Some("male") => format!("{body}, tailored for a male body shape"),
        Some("female") => format!("{body}, tailored for a female body shape"),
        _ => body,
    }
}

/// Prenda pedida por una llamada: tipo, ranura que ocupa y corte. @spec REQ-84
struct Garment {
    kind: &'static str,
    slot: &'static str,
    cut: Option<&'static str>,
}

/// La prenda de una llamada: por `kind_key` o, si no está, por la ranura (compatibilidad con entradas y llamadas
/// anteriores). `None` si no pide ninguna. @spec REQ-84
fn garment_of(args: &Value, keys: [&str; 3], prompt: &str) -> Result<Option<Garment>, String> {
    let [kind_key, slot_key, cut_key] = keys;
    let kind_arg = text(args, kind_key);
    let slot_arg = text(args, slot_key);
    let cut = normalize_garment_cut(&text(args, cut_key))?;
    let garment = if !kind_arg.is_empty() {
        let kind = normalize_garment_kind(&kind_arg)?;
        Some(Garment { kind, slot: slot_of_kind(kind), cut })
    } else if !slot_arg.is_empty() {
        let slot = normalize_garment_slot(&slot_arg)?;
        Some(Garment { kind: infer_garment_kind(slot, prompt), slot, cut })
    } else {
        None
    };
    Ok(garment)
}

/// Marca en los parámetros del trabajo que es una prenda: tipo, ranura, corte y retopología a quads. @spec REQ-80, REQ-84
fn insert_garment(own: &mut Map<String, Value>, garment: &Garment, quad_limit: u64) {
    own.insert("garment_slot".into(), json!(garment.slot));
    own.insert("garment_kind".into(), json!(garment.kind));
    if let Some(cut) = garment.cut {
        own.insert("garment_cut".into(), json!(cut));
    }
    own.insert("quad_remesh".into(), json!(true));
    own.insert("quad_face_limit".into(), json!(quad_limit));
}

/// Prompt negativo anatómico por defecto para prendas de MetaHuman. @spec REQ-80
pub const GARMENT_NEGATIVE_PROMPT: &str = "human body, skin, head, face, neck, hands, fingers, legs, feet, mannequin, dummy, stand, hanger, asymmetrical pose, folded clothes";

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

fn with_confirm(mut spec: Value, confirm: bool) -> Value {
    if confirm {
        if let Some(props) = spec.pointer_mut("/inputSchema/properties").and_then(Value::as_object_mut) {
            props.insert("confirm_cost".into(), json!({ "type": "boolean", "description": "true only after the user accepted the total cost shown by a first call without it." }));
        }
        if let Some(required) = spec.pointer_mut("/inputSchema/required").and_then(Value::as_array_mut) {
            required.push(json!("confirm_cost"));
        }
    }
    spec
}

/// Definición de las herramientas. `confirm`: el agente confirma el coste con un parámetro (MCP); el asistente
/// integrado lo hace con la tarjeta de coste de la app y no lo necesita.
pub fn specs(confirm: bool) -> Vec<Value> {
    let ask = if confirm {
        " Call it first WITHOUT confirm_cost to get the exact cost, tell the user, and call again with confirm_cost=true only if they accept."
    } else {
        " The app shows the cost to the user and asks for confirmation."
    };
    let name = json!({ "type": "string", "description": "SHORT name of the object in the user's language (\"Pirata\", \"Linterna de hierro\"); never the prompt." });
    let variants = json!({ "type": "integer", "minimum": 1, "maximum": MAX_VARIANTS, "description": "How many different results from the same request (default 1); each one costs the same." });
    let options = json!({ "type": "object", "description": "Extra Tripo parameters (see tripo_operations), e.g. {\"texture_quality\": \"detailed\", \"negative_prompt\": \"…\"}." });
    let launchers = vec![
        tool(
            "generate_3d_model",
            &format!("Generate 3D models from a text description with Tripo (≈20 credits each with texture; smart_low_poly +10; garment_slot +10 for quad retopology). The models go to the app library and the user sees the progress. Write `prompt` in English describing ONE object; for a character that will be rigged ask for a T-pose; for MetaHuman clothing prefer generate_metahuman_outfit. It does NOT rig, animate or import into Unreal: use rig_model, animate_model and library_send_to_unreal.{ask}"),
            json!({ "prompt": { "type": "string" }, "name": name, "variants": variants, "face_limit": { "type": "integer", "description": "Approximate triangle/quad budget (a hint)." }, "smart_low_poly": { "type": "boolean" }, "garment_kind": { "type": "string", "enum": ["top", "pants", "skirt", "dress", "shoes"], "description": "Optional MetaHuman garment type: enriches the prompt for a hollow garment without body and chains quad retopology (+10 credits). A dress is a whole garment; skirts and dresses get cloth simulation when equipped." }, "garment_cut": { "type": "string", "enum": ["male", "female", "neutral"], "description": "Garment cut, to match the body it will be worn on." }, "garment_slot": { "type": "string", "enum": ["Torso", "Legs", "Feet"], "description": "Legacy: use garment_kind." }, "options": options }),
            &["prompt", "name"],
        ),
        tool(
            "image_to_3d_model",
            &format!("Generate 3D models from ONE reference image with Tripo (≈30 credits each; garment_slot +10 for quad retopology). `image` is a local file path (an image the user attached), a library id of an image, or an http(s) URL. For several views of the same object use tripo_run with multiview_to_model.{ask}"),
            json!({ "image": { "type": "string" }, "name": name, "variants": variants, "garment_kind": { "type": "string", "enum": ["top", "pants", "skirt", "dress", "shoes"], "description": "Optional MetaHuman garment type: chains quad retopology (+10 credits)." }, "garment_cut": { "type": "string", "enum": ["male", "female", "neutral"] }, "garment_slot": { "type": "string", "enum": ["Torso", "Legs", "Feet"], "description": "Legacy: use garment_kind." }, "face_limit": { "type": "integer" }, "options": options }),
            &["image", "name"],
        ),
        tool(
            "generate_metahuman_outfit",
            &format!("Generate a 3D garment for a MetaHuman character (`kind`: top, pants, skirt, dress or shoes; `cut`: male, female or neutral, to match the body) with Tripo (30 credits: 20 text_to_model + 10 quad convert). Automatically applies anatomical prompt engineering (hollow A-pose garment without mannequin/limbs) and chains a symmetrical quad retopology (`convert` with `quad: true`, `force_symmetry: true`, `face_limit: 8000`) so Unreal's GeometryScript InpaintWeights can transfer bone weights cleanly when sent or equipped with unreal_equip_metahuman_outfit.{ask}"),
            json!({ "prompt": { "type": "string", "description": "Garment description in English (e.g. 'cyberpunk leather jacket with neon collar')." }, "name": name, "kind": { "type": "string", "enum": ["top", "pants", "skirt", "dress", "shoes"], "description": "Garment type (default top). A dress is a whole garment without body; skirts and dresses are simulated as cloth when equipped." }, "cut": { "type": "string", "enum": ["male", "female", "neutral"], "description": "Garment cut. A female cut does not fit a male body well and the other way round: use the build of the MetaHuman it is for." }, "slot": { "type": "string", "enum": ["Torso", "Legs", "Feet"], "description": "Legacy: use kind." }, "face_limit": { "type": "integer", "description": "Quad polygon budget (default 8000)." }, "variants": variants, "options": options }),
            &["prompt", "name"],
        ),
        tool(
            "rig_model",
            &format!("Add a skeleton and skinning to a library model with Tripo (25 credits) so it can be animated. `preset`: \"tripo_animations\" (default; biped rig v1.0 that supports the 101 Tripo animations: dances, emotions, sports, combat…) or \"unreal_skeleton\" (humanoid rig v3.0 with Unreal bone names, for Unreal's own animations; only 16 Tripo animations). For animals or creatures pass rig_type (quadruped, hexapod, octopod, avian, serpentine, aquatic, others). If unsure whether it can be rigged, first call tripo_run with operation rig_check (free). The result is a new library asset of kind \"rig\".{ask}"),
            json!({ "id": { "type": "string", "description": "Library id of the model (library_list)." }, "preset": { "type": "string", "enum": ["tripo_animations", "unreal_skeleton"] }, "rig_type": { "type": "string" }, "out_format": { "type": "string", "enum": ["glb", "fbx"] }, "name": { "type": "string" } }),
            &["id"],
        ),
        tool(
            "animate_model",
            &format!("Apply up to 5 animations to a RIGGED library asset (kind \"rig\", from rig_model) with Tripo (10 credits per animation). Names like \"dance_01\", \"wave_goodbye_01\", \"walk\". {} {} The result is a new library asset of kind \"animation\".{ask}", catalog::preset_help(true), catalog::preset_help(false)),
            json!({ "id": { "type": "string", "description": "Library id of the rig." }, "animations": { "type": "array", "items": { "type": "string" }, "maxItems": 5 }, "in_place": { "type": "boolean", "description": "Animate without moving forward (default true)." }, "name": { "type": "string" } }),
            &["id", "animations"],
        ),
        tool(
            "tripo_run",
            &format!("Run ANY other Tripo operation: {}. Call tripo_operations to see the parameters and price of one. Sources (`input`, `inputs`, `ref_image`) accept a library id, a local file path, an http(s) URL or a task_/file_ token.{ask}", OPERATIONS.iter().map(|op| op.name).collect::<Vec<_>>().join(", ")),
            json!({ "operation": { "type": "string" }, "params": { "type": "object" }, "name": { "type": "string" }, "variants": variants, "parent": { "type": "string", "description": "Library id this result derives from." } }),
            &["operation", "params"],
        ),
        tool(
            "generate_world",
            &format!("Generate a photorealistic 3D world with World Labs Marble (1580 World Labs credits for marble-1.1; 230 for marble-1.0-draft). World Labs is a separate account from Tripo: check its credits with worldlabs_balance, not tripo_balance. Produces splats (.spz), collider mesh (.glb), panorama and scale metadata saved in the library as kind \"world\". Does NOT export HQ mesh: use export_world_mesh.{ask}"),
            json!({ "prompt": { "type": "string" }, "name": name, "model": { "type": "string", "enum": ["marble-1.1", "marble-1.0-draft"], "description": "marble-1.1 (full, 1580 credits) or marble-1.0-draft (draft, 230 credits)" }, "image": { "type": "string", "description": "Optional local reference image path" } }),
            &["prompt", "name"],
        ),
        tool(
            "export_world_mesh",
            &format!("Export high-resolution 3D mesh (GLB) from an existing World Labs world in the library (3500 World Labs credits; check them with worldlabs_balance).{ask}"),
            json!({ "id": { "type": "string", "description": "Library id of the world (kind \"world\")" }, "name": { "type": "string" } }),
            &["id"],
        ),
    ];

    let mut all: Vec<Value> = launchers.into_iter().map(|spec| with_confirm(spec, confirm)).collect();
    all.push(tool(
        "tripo_operations",
        "List every Tripo operation (generation, images, multiview, splats, texture, convert to FBX/USDZ, retopology, UV, segmentation, rig check, rig, animations) with its price; with `operation`, its full parameters. Free, no credits.",
        json!({ "operation": { "type": "string" } }),
        &[],
    ));
    all.push(tool("tripo_balance", "Credits available in the user's Tripo account (and credits reserved by running tasks). Only for Tripo tools, not for World Labs. Free.", json!({}), &[]));
    all.push(tool("worldlabs_balance", "Credits available in the user's World Labs account (for generate_world and export_world_mesh; separate from Tripo). Free.", json!({}), &[]));
    all
}

fn text(args: &Value, key: &str) -> String {
    args.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

fn variants_of(args: &Value) -> u32 {
    args.get("variants").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).unwrap_or(1).clamp(1, MAX_VARIANTS)
}

/// `options` mezclado con los parámetros propios (los propios mandan).
fn merged(args: &Value, own: Map<String, Value>) -> Value {
    let mut params = args.get("options").and_then(Value::as_object).cloned().unwrap_or_default();
    params.extend(own);
    Value::Object(params)
}

fn required_name(args: &Value) -> Result<String, String> {
    let name = text(args, "name");
    if name.is_empty() {
        Err("`name` is required: a SHORT name for the object in the user's language (e.g. \"Pirata\"), not the prompt".into())
    } else {
        Ok(name)
    }
}

fn op(name: &str) -> Result<&'static Operation, String> {
    catalog::find(name).ok_or_else(|| format!("unknown Tripo operation «{name}». Operations: {}", OPERATIONS.iter().map(|o| o.name).collect::<Vec<_>>().join(", ")))
}

/// Traduce una llamada a una herramienta lanzadora en la operación que hará, sin gastar nada.
pub fn build(tool: &str, args: &Value, library: &Library) -> Result<Launch, String> {
    let launch = match tool {
        "generate_3d_model" => {
            let prompt = text(args, "prompt");
            if prompt.is_empty() {
                return Err("the prompt is empty".into());
            }
            let garment = garment_of(args, ["garment_kind", "garment_slot", "garment_cut"], &prompt)?;
            let mut own = Map::new();
            if let Some(garment) = &garment {
                let quad_limit = args.get("face_limit").and_then(Value::as_u64).unwrap_or(8000).clamp(1000, 20000);
                own.insert("prompt".into(), json!(build_garment_prompt(&prompt, garment.kind, garment.cut)));
                own.insert("original_prompt".into(), json!(prompt));
                own.insert("negative_prompt".into(), json!(GARMENT_NEGATIVE_PROMPT));
                insert_garment(&mut own, garment, quad_limit);
            } else {
                own.insert("prompt".into(), json!(prompt));
                for key in ["face_limit", "smart_low_poly"] {
                    if let Some(value) = args.get(key).filter(|v| !v.is_null()) {
                        own.insert(key.into(), value.clone());
                    }
                }
            }
            Launch { operation: op("text_to_model")?, params: merged(args, own), name: required_name(args)?, variants: variants_of(args), parent: None }
        }
        "image_to_3d_model" => {
            let image = text(args, "image");
            if image.is_empty() {
                return Err("`image` is required: a local file path, a library id or an http(s) URL".into());
            }
            let mut own = Map::new();
            own.insert("input".into(), json!(image));
            if let Some(garment) = garment_of(args, ["garment_kind", "garment_slot", "garment_cut"], "")? {
                let quad_limit = args.get("face_limit").and_then(Value::as_u64).unwrap_or(8000).clamp(1000, 20000);
                insert_garment(&mut own, &garment, quad_limit);
            }
            Launch { operation: op("image_to_model")?, params: merged(args, own), name: required_name(args)?, variants: variants_of(args), parent: library.get(&image).map(|e| e.id) }
        }
        "generate_metahuman_outfit" => {
            let prompt = text(args, "prompt");
            if prompt.is_empty() {
                return Err("the prompt is empty".into());
            }
            let garment = garment_of(args, ["kind", "slot", "cut"], &prompt)?.unwrap_or(Garment { kind: "top", slot: "Torso", cut: None });
            let quad_limit = args.get("face_limit").and_then(Value::as_u64).unwrap_or(8000).clamp(1000, 20000);
            let mut own = Map::new();
            own.insert("prompt".into(), json!(build_garment_prompt(&prompt, garment.kind, garment.cut)));
            own.insert("original_prompt".into(), json!(prompt));
            own.insert("negative_prompt".into(), json!(GARMENT_NEGATIVE_PROMPT));
            insert_garment(&mut own, &garment, quad_limit);
            Launch { operation: op("text_to_model")?, params: merged(args, own), name: required_name(args)?, variants: variants_of(args), parent: None }
        }
        "rig_model" => {
            let id = text(args, "id");
            let source = library.get(&id).ok_or_else(|| format!("no library asset with id «{id}». Call library_list."))?;
            let rig_type = text(args, "rig_type");
            let mut params = Map::new();
            params.insert("input".into(), json!(id));
            if !rig_type.is_empty() && rig_type != "biped" {
                params.insert("model".into(), json!("v2.5-20260210"));
                params.insert("rig_type".into(), json!(rig_type));
            } else if text(args, "preset") == "unreal_skeleton" {
                params.insert("model".into(), json!("v3.0-20260909"));
                params.insert("rig_type".into(), json!("biped"));
                params.insert("spec".into(), json!("unreal"));
            } else {
                // Lo verificado en Holo-Companion: v1.0 + spec tripo es lo único que acepta los 101 presets.
                params.insert("model".into(), json!("v1.0-20240301"));
                params.insert("rig_type".into(), json!("biped"));
                params.insert("spec".into(), json!("tripo"));
            }
            params.insert("out_format".into(), json!(if text(args, "out_format") == "fbx" { "fbx" } else { "glb" }));
            let name = Some(text(args, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{} (rig)", source.name));
            Launch { operation: op("rig")?, params: Value::Object(params), name, variants: 1, parent: Some(id) }
        }
        "animate_model" => {
            let id = text(args, "id");
            let rig = library.get(&id).ok_or_else(|| format!("no library asset with id «{id}». Call library_list."))?;
            if rig.kind != "rig" {
                return Err(format!("«{}» is a {}, not a rig: call rig_model on it first", rig.name, rig.kind));
            }
            let v1 = catalog::is_v1_rig(rig.parameters.get("request").unwrap_or(&Value::Null));
            let names: Vec<String> = args.get("animations").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
            if names.is_empty() || names.len() > 5 {
                return Err(format!("give between 1 and 5 animations. {}", catalog::preset_help(v1)));
            }
            let ids = names.iter().map(|n| catalog::preset_id(n, v1)).collect::<Result<Vec<_>, _>>()?;
            let short: Vec<&str> = ids.iter().map(|i| i.rsplit(':').next().unwrap_or(i)).collect();
            let params = json!({ "input": id, "animations": ids, "out_format": "glb", "animate_in_place": args.get("in_place").and_then(Value::as_bool).unwrap_or(true) });
            let name = Some(text(args, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{} · {}", rig.name.trim_end_matches(" (rig)"), short.join(", ")));
            Launch { operation: op("retarget")?, params, name, variants: 1, parent: Some(id) }
        }
        "tripo_run" => {
            let operation = op(&text(args, "operation"))?;
            let params = args.get("params").cloned().filter(Value::is_object).ok_or("`params` must be an object with the operation parameters (see tripo_operations)")?;
            catalog::validate(operation, &params)?;
            let name = Some(text(args, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| operation.name.replace('_', " "));
            let parent = Some(text(args, "parent")).filter(|p| !p.is_empty()).or_else(|| params.get("input").and_then(Value::as_str).and_then(|i| library.get(i)).map(|e| e.id));
            Launch { operation, params, name, variants: variants_of(args), parent }
        }
        "generate_world" => {
            let prompt = text(args, "prompt");
            if prompt.is_empty() {
                return Err("the prompt is empty".into());
            }
            let name = required_name(args)?;
            let model_str = text(args, "model");
            let model = if model_str.is_empty() { "marble-1.1" } else { &model_str };
            let mut params = Map::new();
            params.insert("prompt".into(), json!(prompt));
            params.insert("model".into(), json!(model));
            if let Some(img) = args.get("image").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
                params.insert("image".into(), json!(img.trim()));
            }
            Launch { operation: op("generate_world")?, params: Value::Object(params), name, variants: 1, parent: None }
        }
        "export_world_mesh" => {
            let id = text(args, "id");
            let entry = library.get(&id).ok_or_else(|| format!("no library asset with id «{id}». Call library_list."))?;
            if entry.kind != "world" {
                return Err(format!("«{}» is a {}, not a world: export_world_mesh only works on world assets", entry.name, entry.kind));
            }
            let name = Some(text(args, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("{} (HQ mesh)", entry.name));
            let params = json!({ "id": id, "world_id": entry.parameters.get("world_id").and_then(Value::as_str).unwrap_or(&id) });
            Launch { operation: op("export_world_mesh")?, params, name, variants: 1, parent: Some(id) }
        }
        other => return Err(format!("{other} does not launch a generation operation")),
    };

    Ok(launch)
}

/// Texto de `tripo_operations`: la lista con precios o los parámetros de una.
pub fn operations_text(args: &Value) -> Result<String, String> {
    let wanted = text(args, "operation");
    let text = if wanted.is_empty() {
        OPERATIONS.iter().map(|op| format!("- {} [{}]: {} Price: {}", op.name, op.category, op.summary, op.pricing)).collect::<Vec<_>>().join("\n")
    } else {
        let op = op(&wanted)?;
        let presets = if op.name == "retarget" { format!("\n{}\n{}", catalog::preset_help(true), catalog::preset_help(false)) } else { String::new() };
        format!("{} (POST {}): {}\nParameters: {}\nPrice (credits): {}\nProduces: {}{presets}", op.name, op.endpoint, op.summary, op.params, op.pricing, op.produces)
    };
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryEntry;

    fn library_with(entries: &[(&str, &str, Value)]) -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let library = Library::new(dir.path().join("library"));
        for (id, kind, request) in entries {
            library
                .add(LibraryEntry { id: (*id).into(), name: "Pirata".into(), provider: "tripo".into(), kind: (*kind).into(), prompt: String::new(), parameters: json!({ "request": request }), task_id: "task_1".into(), created_at: 1, credits: None, files: vec!["model.glb".into()], thumbnail: None, sent_to: vec![], parent: None, output: Value::Null })
                .unwrap();
        }
        (dir, library)
    }

    /// El agente ve las mismas herramientas que el asistente integrado, y todas las que lanzan algo piden coste.
    /// @spec REQ-33
    #[test]
    fn las_dos_superficies_publican_las_mismas_herramientas() {
        let mcp: Vec<String> = specs(true).iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        let app: Vec<String> = specs(false).iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(mcp, app);
        for launcher in LAUNCHERS {
            let spec = specs(true).into_iter().find(|s| s["name"] == launcher).unwrap();
            assert!(spec["inputSchema"]["required"].as_array().unwrap().contains(&json!("confirm_cost")), "{launcher}");
        }
        let rig = specs(false).into_iter().find(|s| s["name"] == "rig_model").unwrap();
        assert!(rig["description"].as_str().unwrap().contains("101"), "explica qué rig admite los bailes");
    }

    /// @spec REQ-16, REQ-33
    #[test]
    fn rig_y_animaciones_eligen_el_modelo_y_validan_los_nombres() {
        let (_dir, library) = library_with(&[("pirata-1", "model", json!({})), ("pirata-rig", "rig", json!({ "model": "v1.0-20240301" })), ("ue-rig", "rig", json!({ "model": "v3.0-20260909", "spec": "unreal" }))]);
        let rig = build("rig_model", &json!({ "id": "pirata-1" }), &library).unwrap();
        assert_eq!(rig.params["model"], "v1.0-20240301");
        assert_eq!(rig.params["spec"], "tripo");
        assert_eq!(rig.parent.as_deref(), Some("pirata-1"));
        assert_eq!(rig.name, "Pirata (rig)");
        assert_eq!(rig.total(), 25.0);
        let unreal = build("rig_model", &json!({ "id": "pirata-1", "preset": "unreal_skeleton" }), &library).unwrap();
        assert_eq!((unreal.params["model"].as_str(), unreal.params["spec"].as_str()), (Some("v3.0-20260909"), Some("unreal")));
        let dog = build("rig_model", &json!({ "id": "pirata-1", "rig_type": "quadruped" }), &library).unwrap();
        assert_eq!(dog.params["model"], "v2.5-20260210");

        let dance = build("animate_model", &json!({ "id": "pirata-rig", "animations": ["dance_01", "wave_goodbye_01"] }), &library).unwrap();
        assert_eq!(dance.params["animations"], json!(["preset:biped:dance_01", "preset:biped:wave_goodbye_01"]));
        assert_eq!(dance.total(), 20.0);
        assert!(build("animate_model", &json!({ "id": "ue-rig", "animations": ["dance_01"] }), &library).unwrap_err().contains("Rig v2/v3"));
        assert!(build("animate_model", &json!({ "id": "pirata-1", "animations": ["walk"] }), &library).unwrap_err().contains("not a rig"));
    }

    /// @spec REQ-28, REQ-29, REQ-32
    #[test]
    fn generar_exige_nombre_y_suma_variantes_e_imagenes() {
        let (_dir, library) = library_with(&[]);
        assert!(build("generate_3d_model", &json!({ "prompt": "pirate" }), &library).unwrap_err().contains("`name` is required"));
        let text = build("generate_3d_model", &json!({ "prompt": "pirate", "name": "Pirata", "variants": 3, "smart_low_poly": true, "options": { "texture_quality": "detailed" } }), &library).unwrap();
        assert_eq!(text.params, json!({ "prompt": "pirate", "smart_low_poly": true, "texture_quality": "detailed" }));
        assert_eq!(text.total(), 120.0, "3 × (20 + 10 low poly + 10 detailed)");
        let image = build("image_to_3d_model", &json!({ "image": "C:/refs/barco.png", "name": "Barco" }), &library).unwrap();
        assert_eq!(image.params["input"], "C:/refs/barco.png");
        assert_eq!(image.total(), 30.0);
        assert!(build("tripo_run", &json!({ "operation": "convert", "params": { "input": "x" } }), &library).unwrap_err().contains("format"));
        assert!(operations_text(&json!({})).unwrap().contains("retopology"));
        assert!(operations_text(&json!({ "operation": "retarget" })).unwrap().contains("dance_01"));
    }

    /// @spec REQ-80
    #[test]
    fn prenda_metahuman_enriquece_prompt_por_ranura_y_suma_retopologia_quads() {
        let (_dir, library) = library_with(&[]);
        let outfit = build(
            "generate_metahuman_outfit",
            &json!({ "prompt": "cyberpunk leather jacket", "name": "Chaqueta Cyberpunk", "slot": "Torso", "face_limit": 8000 }),
            &library,
        )
        .unwrap();
        assert_eq!(outfit.operation.name, "text_to_model");
        assert_eq!(outfit.params["garment_slot"], "Torso");
        assert_eq!(outfit.params["garment_kind"], "top", "una llamada antigua con solo la ranura recibe su tipo");
        assert_eq!(outfit.params["quad_remesh"], true);
        assert_eq!(outfit.params["quad_face_limit"], 8000);
        assert!(outfit.params["prompt"].as_str().unwrap().contains("symmetrical A-pose"));
        assert!(outfit.params["negative_prompt"].as_str().unwrap().contains("mannequin"));
        assert_eq!(outfit.total(), 30.0, "20 base + 10 convert quad");
        assert!(build("generate_metahuman_outfit", &json!({ "prompt": "hat", "name": "Sombrero", "slot": "Head" }), &library).is_err());
    }

    /// @spec REQ-84
    #[test]
    fn cada_tipo_de_prenda_ocupa_su_ranura_y_pide_lo_suyo() {
        let (_dir, library) = library_with(&[]);
        for (kind, slot) in [("top", "Torso"), ("pants", "Legs"), ("skirt", "Legs"), ("dress", "Torso"), ("shoes", "Feet")] {
            let launch = build("generate_metahuman_outfit", &json!({ "prompt": "red thing", "name": "Prenda", "kind": kind }), &library).unwrap();
            assert_eq!(launch.params["garment_kind"], kind);
            assert_eq!(launch.params["garment_slot"], slot, "{kind}");
            assert_eq!(launch.total(), 30.0, "{kind}: 20 base + 10 quads");
            assert!(launch.params["prompt"].as_str().unwrap().contains("red thing"));
        }
        let dress = build("generate_3d_model", &json!({ "prompt": "red evening dress", "name": "Vestido", "garment_kind": "dress", "garment_cut": "female" }), &library).unwrap();
        let prompt = dress.params["prompt"].as_str().unwrap();
        assert!(prompt.contains("full-length dress") && prompt.contains("no legs") && prompt.ends_with("tailored for a female body shape"), "{prompt}");
        assert_eq!(dress.params["garment_cut"], "female");
        assert_eq!(dress.params["original_prompt"], "red evening dress");
        let man = build("image_to_3d_model", &json!({ "image": "C:/refs/chaqueta.png", "name": "Chaqueta", "garment_kind": "top", "garment_cut": "male" }), &library).unwrap();
        assert_eq!((man.params["garment_kind"].as_str(), man.params["garment_cut"].as_str(), man.params["garment_slot"].as_str()), (Some("top"), Some("male"), Some("Torso")));
        let neutral = build("generate_metahuman_outfit", &json!({ "prompt": "hoodie", "name": "Sudadera", "cut": "neutral" }), &library).unwrap();
        assert!(!neutral.params["prompt"].as_str().unwrap().contains("tailored for"), "neutro no añade nada");
        assert!(build("generate_metahuman_outfit", &json!({ "prompt": "hat", "name": "Sombrero", "kind": "hat" }), &library).unwrap_err().contains("top, pants, skirt, dress or shoes"));
        assert!(build("generate_metahuman_outfit", &json!({ "prompt": "x", "name": "X", "cut": "robot" }), &library).unwrap_err().contains("male, female or neutral"));
    }

    /// Una entrada antigua con solo `garment_slot` sigue valiendo y recibe su tipo. @spec REQ-84
    #[test]
    fn una_entrada_antigua_con_solo_la_ranura_recibe_su_tipo() {
        assert_eq!(infer_garment_kind("Torso", "leather jacket"), "top");
        assert_eq!(infer_garment_kind("Feet", "boots"), "shoes");
        assert_eq!(infer_garment_kind("Legs", "tartan kilt"), "skirt");
        assert_eq!(infer_garment_kind("Legs", "falda plisada"), "skirt");
        assert_eq!(infer_garment_kind("Legs", "denim jeans"), "pants");
        let (_dir, library) = library_with(&[]);
        let old = build("generate_3d_model", &json!({ "prompt": "tartan kilt", "name": "Kilt", "garment_slot": "Legs" }), &library).unwrap();
        assert_eq!((old.params["garment_slot"].as_str(), old.params["garment_kind"].as_str()), (Some("Legs"), Some("skirt")));
        assert!(old.params["prompt"].as_str().unwrap().contains("standalone skirt"));
    }
}
