// =============================================================================
// DBV Unreal AI — Herramientas de Fab para la IA integrada y los agentes (MCP)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-103, REQ-105, REQ-117
//
// Una sola definición para la ventana y el `--mcp` (como la caja negra). `fab_search` y `fab_listing_details` van
// por Rust sin sesión; `fab_my_library`, `fab_install` y `fab_sign_in` van por el plugin C++ dentro del editor,
// con la sesión de Fab que el usuario abrió con el botón Fab del Content Browser (el token no sale del editor).
// Las del plugin son trabajos: `fab_start` y, después, `fab_status` hasta que terminan.
//
// El resultado lleva el texto para el modelo y, aparte, los listings para pintar tarjetas con miniatura y enlace
// en la conversación (REQ-117): las tarjetas salen de los datos, no de lo que escriba el modelo.

use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use super::{compact_listing, engine_tag, listing_details, search, FabListing, SearchQuery};

/// Toolset C++ que tiene las funciones `fab_*` (la variante Python no las tiene).
pub const FAB_TOOLSET: &str = "DBVUnrealAICpp.DBVUnrealAICppToolset";
pub const WRITE_TOOLS: [&str; 2] = ["fab_install", "fab_sign_in"];
const POLL_EVERY: Duration = Duration::from_millis(400);
const QUICK_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const NO_SESSION_HINT: &str = "Ask the user to click the Fab button in the Content Browser of the Unreal Editor (it opens the Fab tab and signs them in), then try again. Never say they are signed in without checking again.";

/// Lo que necesitan las herramientas del editor conectado. Lo implementa cada superficie.
pub trait FabEditor {
    /// Llama a una función del toolset C++ (`fab_start`, `fab_status`, `run_python`…) y devuelve su valor JSON.
    fn call_cpp(&mut self, tool: &str, args: Value) -> Result<Value, String>;
}

/// Resultado de una herramienta: texto para el modelo y listings para las tarjetas.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FabToolResult {
    pub text: String,
    pub fab_listings: Vec<FabListing>,
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({ "name": name, "description": description, "inputSchema": { "type": "object", "properties": properties, "required": required } })
}

/// Definiciones (las mismas en la ventana y en el `--mcp`). @spec REQ-105
pub fn specs() -> Vec<Value> {
    let uid = json!({ "type": "string", "description": "Listing uid from fab_search or fab_my_library." });
    vec![
        tool(
            "fab_search",
            "Search the Fab catalog (no sign-in needed). Returns up to 24 listings per page with title, seller, free or paid, rating, the formats that can be installed in THIS editor version, a thumbnail and the fab.com link. Only listings compatible with the editor are returned unless include_incompatible=true. Fab gives no total: never state or guess how many results exist; use next_cursor for more. When you show results to the user, always include the fab.com link of each listing you name (the app also shows them as cards with the thumbnail). To filter by seller use seller_id from a result, never the seller name. Read-only.",
            json!({
                "query": { "type": "string", "description": "What to look for, in English works best (e.g. \"car\", \"medieval house\")." },
                "free_only": { "type": "boolean", "description": "Only free listings. Use it unless the user asks for paid ones or for their library." },
                "formats": { "type": "array", "items": { "type": "string", "enum": ["unreal-engine", "glb", "gltf", "fbx", "converted-files"] }, "description": "Default: all installable formats." },
                "listing_types": { "type": "array", "items": { "type": "string" }, "description": "For example 3d-model, material, environment, game-system." },
                "seller_id": { "type": "string" },
                "sort_by": { "type": "string", "description": "For example -relevance, -createdAt, -averageRating." },
                "cursor": { "type": "string", "description": "next_cursor of the previous page." },
                "include_incompatible": { "type": "boolean", "description": "Also show listings that cannot be installed in this editor (marked, never installable)." }
            }),
            &["query"],
        ),
        tool(
            "fab_listing_details",
            "Details of one Fab listing (no sign-in needed): description, licenses, formats compatible with this editor, thumbnail and fab.com link. If the listing is marked mature, ask the user before opening it. Read-only.",
            json!({ "listing_uid": uid }),
            &["listing_uid"],
        ),
        tool(
            "fab_my_library",
            "Search the user's own Fab library (needs the Fab session of the open editor). Same compact listings as fab_search, marked owned. If there is no session, tell the user to click the Fab button in the Content Browser. Read-only.",
            json!({ "query": { "type": "string" }, "cursor": { "type": "string" } }),
            &[],
        ),
        tool(
            "fab_install",
            "Install one Fab listing into the open Unreal project (asks the user for approval). Only free listings or listings in the user's library, and only a format compatible with this editor: the plugin checks both and answers no_owned or incompatible_engine without downloading. Nothing is ever bought: for a paid listing the user does not own, give the fab.com link. glb/gltf/fbx are imported into /Game/Fab/<Title>/; an unreal-engine pack goes to /Game/<Pack>/. If the answer is choose_file, ask the user which file and call again with artifact_id. If it is eula_required, give the link and do not retry. Can take minutes for big packs.",
            json!({
                "listing_uid": uid,
                "asset_format": { "type": "string", "enum": ["unreal-engine", "glb", "gltf", "fbx", "converted-files"], "description": "One of the listing's compatible formats." },
                "artifact_id": { "type": "string", "description": "Only after a choose_file answer." }
            }),
            &["listing_uid", "asset_format"],
        ),
        tool(
            "fab_sign_in",
            "Check the Fab session of the open editor. If there is none, it tells the user to click the Fab button in the Content Browser; with open_portal=true it opens the Epic account portal instead (asks for approval). It never claims the user is signed in: call it again to check.",
            json!({ "open_portal": { "type": "boolean" } }),
            &[],
        ),
    ]
}

/// ¿Es una herramienta de Fab?
pub fn is_tool(name: &str) -> bool {
    let out = specs().iter().any(|s| s["name"].as_str() == Some(name));
    out
}

fn text_arg(args: &Value, key: &str) -> String {
    args.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

fn list_arg(args: &Value, key: &str) -> Vec<String> {
    args.get(key).and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// Lanza un trabajo del plugin y espera a que termine. Devuelve el `result` del trabajo (también si es un error).
pub fn run_job(editor: &mut dyn FabEditor, operation: &str, args: &Value, timeout: Duration) -> Result<Value, String> {
    let started = editor.call_cpp("fab_start", json!({ "operation": operation, "args_json": args.to_string() }))?;
    let job = started.get("job_id").and_then(Value::as_str).map(String::from).ok_or_else(|| format!("the editor did not start the Fab job: {started}. Is the DBVUnrealAICpp plugin compiled with Fab support (v0.6.1)?"))?;
    let deadline = Instant::now() + timeout;
    let mut outcome = Err(format!("the Fab job did not finish in {} s", timeout.as_secs()));
    while Instant::now() < deadline {
        let status = editor.call_cpp("fab_status", json!({ "job_id": job }))?;
        if status.get("state").and_then(Value::as_str) != Some("running") {
            outcome = Ok(status.get("result").cloned().unwrap_or(status));
            break;
        }
        std::thread::sleep(POLL_EVERY);
    }
    if outcome.is_err() {
        let _ = editor.call_cpp("fab_cancel", json!({ "job_id": job }));
    }
    outcome
}

/// Versión del editor conectado en el formato de Fab (`UE_5.8`), o vacía si no se sabe.
fn editor_engine_tag(editor: Option<&mut dyn FabEditor>) -> String {
    let tag = editor.and_then(|e| run_job(e, "session", &json!({}), QUICK_TIMEOUT).ok()).and_then(|v| v.get("engine_tag").and_then(Value::as_str).map(String::from)).unwrap_or_default();
    tag
}

/// Una línea compacta por listing para el modelo (el resto va a las tarjetas).
fn listings_text(listings: &[FabListing], next_cursor: Option<&str>, hidden: usize, engine: &str) -> String {
    let items: Vec<Value> = listings
        .iter()
        .map(|l| {
            json!({
                "uid": l.uid, "title": l.title, "seller": l.seller_name, "seller_id": l.seller_id,
                "free": l.is_free, "owned": l.owned, "rating": l.rating, "type": l.listing_type,
                "installable_formats": l.compatible_formats, "compatible": l.compatible, "mature": l.mature,
                "fab_url": l.fab_url
            })
        })
        .collect();
    let mut out = json!({ "listings": items, "next_cursor": next_cursor, "engine": engine });
    if hidden > 0 {
        out["hidden_incompatible_on_this_page"] = json!(hidden);
    }
    let text = format!("{out}\nNever state a total. Show the fab.com link of each listing you mention. Install only free or owned listings, in an installable format.");
    text
}

fn fab_search(args: &Value, engine: &str) -> Result<FabToolResult, String> {
    let query = SearchQuery {
        q: text_arg(args, "query"),
        asset_formats: list_arg(args, "formats"),
        listing_types: list_arg(args, "listing_types"),
        sellers: Some(text_arg(args, "seller_id")).filter(|s| !s.is_empty()).into_iter().collect(),
        sort_by: text_arg(args, "sort_by"),
        is_free: args.get("free_only").and_then(Value::as_bool).unwrap_or(false),
        cursor: text_arg(args, "cursor"),
        engine_version: engine.to_string(),
        include_incompatible: args.get("include_incompatible").and_then(Value::as_bool).unwrap_or(false),
        ..Default::default()
    };
    let result = search(&query).map_err(|e| e.to_string()).map(|page| FabToolResult {
        text: listings_text(&page.listings, page.next_cursor.as_deref(), page.hidden_incompatible, &page.engine_tag),
        fab_listings: page.listings,
    });
    result
}

fn fab_details(args: &Value, engine: &str) -> Result<FabToolResult, String> {
    let result = listing_details(&text_arg(args, "listing_uid"), engine)
        .map_err(|e| e.to_string())
        .map(|(card, details)| FabToolResult { text: format!("{details}
Show the user the fab_url."), fab_listings: vec![card] });
    result
}

/// `library/search` del plugin → listings compactos marcados `owned`. @spec REQ-103
pub fn library_page(result: &Value, engine: &str) -> (Vec<FabListing>, Option<String>) {
    let tag = engine_tag(engine);
    let listings = result
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let mut listing = compact_listing(entry.get("listing")?, tag.as_deref());
                    listing.owned = entry.get("canRequestDownloadUrl").and_then(Value::as_bool).unwrap_or(true);
                    Some(listing)
                })
                .collect()
        })
        .unwrap_or_default();
    let next = result.get("next_cursor").and_then(Value::as_str).filter(|c| !c.is_empty()).map(String::from);
    (listings, next)
}

fn job_error_text(result: &Value) -> Option<String> {
    let failed = result.get("success").and_then(Value::as_bool) == Some(false);
    let text = failed.then(|| {
        let code = result.get("error").and_then(Value::as_str).unwrap_or("error");
        let hint = if code == "no_session" || code == "ambiguous_session" { format!(" {NO_SESSION_HINT}") } else { String::new() };
        format!("error: {result}{hint}")
    });
    text
}

fn fab_my_library(editor: &mut dyn FabEditor, args: &Value) -> Result<FabToolResult, String> {
    let engine = editor_engine_tag(Some(editor));
    let job_args = json!({ "q": text_arg(args, "query"), "cursor": text_arg(args, "cursor"), "count": "24" });
    let result = run_job(editor, "library_search", &job_args, QUICK_TIMEOUT)?;
    let out = match job_error_text(&result) {
        Some(text) => FabToolResult { text, fab_listings: Vec::new() },
        None => {
            let (listings, next) = library_page(&result, &engine);
            FabToolResult { text: listings_text(&listings, next.as_deref(), 0, &engine), fab_listings: listings }
        }
    };
    Ok(out)
}

fn fab_install(editor: &mut dyn FabEditor, args: &Value) -> Result<FabToolResult, String> {
    let job_args = json!({ "listing_uid": text_arg(args, "listing_uid"), "asset_format": text_arg(args, "asset_format"), "artifact_id": text_arg(args, "artifact_id") });
    let result = run_job(editor, "install", &job_args, INSTALL_TIMEOUT)?;
    let text = job_error_text(&result).unwrap_or_else(|| format!("{result}\nInstalled. Tell the user where it is (folder) and offer to place it in the level."));
    Ok(FabToolResult { text, fab_listings: Vec::new() })
}

fn fab_sign_in(editor: &mut dyn FabEditor, args: &Value) -> Result<FabToolResult, String> {
    let session = run_job(editor, "session", &json!({}), QUICK_TIMEOUT)?;
    let signed_in = session.get("signed_in").and_then(Value::as_bool).unwrap_or(false);
    let open_portal = args.get("open_portal").and_then(Value::as_bool).unwrap_or(false);
    let text = if signed_in {
        "The Fab session of the editor is active.".to_string()
    } else if open_portal {
        let code = "import unreal\nunreal.SystemLibrary.execute_console_command(None, 'Fab.Login')\nresult = 'ok'\n";
        editor.call_cpp("run_python", json!({ "code": code }))?;
        "The Epic account portal was opened for Fab. The user is NOT signed in yet: when they finish, call fab_sign_in again to check.".to_string()
    } else {
        format!("There is no Fab session in the editor. {NO_SESSION_HINT}")
    };
    Ok(FabToolResult { text, fab_listings: Vec::new() })
}

/// Ejecuta una herramienta `fab_*`. `editor` falta si no hay editor conectado; `engine_version` es la del proyecto
/// (se usa la del editor si está conectado). @spec REQ-105
pub fn run_tool(name: &str, args: &Value, editor: Option<&mut dyn FabEditor>, engine_version: &str) -> Result<FabToolResult, String> {
    let needs_editor = !matches!(name, "fab_search" | "fab_listing_details");
    let result = match (name, editor) {
        (_, None) if needs_editor => Err(format!("no Unreal Editor is connected: {name} needs the editor open with its Fab session. Connect it in the Project view.")),
        ("fab_search", editor) => {
            let tag = editor_engine_tag(editor);
            fab_search(args, if tag.is_empty() { engine_version } else { &tag })
        }
        ("fab_listing_details", editor) => {
            let tag = editor_engine_tag(editor);
            fab_details(args, if tag.is_empty() { engine_version } else { &tag })
        }
        ("fab_my_library", Some(editor)) => fab_my_library(editor, args),
        ("fab_install", Some(editor)) => fab_install(editor, args),
        ("fab_sign_in", Some(editor)) => fab_sign_in(editor, args),
        _ => Err(format!("unknown Fab tool {name}")),
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Editor falso: responde en orden lo que se le prepara y apunta lo que se le pide.
    struct FakeEditor {
        replies: VecDeque<Value>,
        calls: Vec<(String, Value)>,
    }

    impl FabEditor for FakeEditor {
        fn call_cpp(&mut self, tool: &str, args: Value) -> Result<Value, String> {
            self.calls.push((tool.to_string(), args));
            self.replies.pop_front().ok_or_else(|| "no more replies".to_string())
        }
    }

    fn job(result: Value) -> Vec<Value> {
        vec![json!({ "success": true, "job_id": "j1" }), json!({ "state": "running", "progress": 0.5 }), json!({ "state": "done", "result": result })]
    }

    /// @spec REQ-105 Cinco herramientas, dos de escritura, y las reglas en las descripciones.
    #[test]
    fn las_herramientas_de_fab_llevan_sus_reglas() {
        let names: Vec<String> = specs().iter().map(|s| s["name"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, vec!["fab_search", "fab_listing_details", "fab_my_library", "fab_install", "fab_sign_in"]);
        let all = serde_json::to_string(&specs()).unwrap();
        assert!(all.contains("never state or guess how many") && all.contains("Nothing is ever bought") && all.contains("seller_id"));
        assert!(all.contains("Fab button in the Content Browser"));
        assert!(WRITE_TOOLS.iter().all(|t| is_tool(t)));
    }

    /// @spec REQ-104 El trabajo se sigue hasta que termina y devuelve su resultado, también el rechazo.
    #[test]
    fn instalar_sigue_el_trabajo_y_cuenta_el_rechazo() {
        let rejected = json!({ "success": false, "error": "no_owned", "fab_url": "https://www.fab.com/listings/x" });
        let mut editor = FakeEditor { replies: job(rejected).into(), calls: Vec::new() };
        let out = run_tool("fab_install", &json!({ "listing_uid": "x", "asset_format": "unreal-engine" }), Some(&mut editor), "5.8.4").unwrap();
        assert!(out.text.starts_with("error:") && out.text.contains("no_owned") && out.text.contains("https://www.fab.com/listings/x"));
        assert_eq!(editor.calls[0].0, "fab_start");
        assert!(editor.calls[0].1["args_json"].as_str().unwrap().contains("\"listing_uid\":\"x\""));
        assert_eq!(editor.calls.iter().filter(|(t, _)| t == "fab_status").count(), 2);
    }

    /// @spec REQ-103 Sin sesión se dice lo del botón Fab, y sin editor no se intenta.
    #[test]
    fn sin_sesion_se_pide_pulsar_el_boton_fab() {
        let mut editor = FakeEditor { replies: job(json!({ "success": true, "signed_in": false, "reason": "no_session" })).into(), calls: Vec::new() };
        let out = run_tool("fab_sign_in", &json!({}), Some(&mut editor), "").unwrap();
        assert!(out.text.contains("Fab button in the Content Browser"));
        assert!(editor.calls.iter().all(|(t, _)| t != "run_python"), "sin open_portal no se abre nada");
        let err = run_tool("fab_my_library", &json!({}), None, "5.8.4").unwrap_err();
        assert!(err.contains("no Unreal Editor is connected"));
    }

    /// @spec REQ-103, REQ-117 La biblioteca sale como listings compactos marcados owned, con miniatura y enlace.
    #[test]
    fn la_biblioteca_sale_con_tarjetas_marcadas_owned() {
        let entry = json!({
            "canRequestDownloadUrl": true,
            "listing": {
                "uid": "l1", "title": "Mi coche", "isFree": false, "publisher": { "sellerName": "Epic Games", "sellerId": "o-epic" },
                "thumbnails": [{ "images": [{ "url": "https://media.fab.com/a.jpg", "width": 320 }] }],
                "assetFormats": [{ "assetFormatType": { "code": "unreal-engine" }, "technicalSpecs": { "unrealEngineEngineVersions": ["UE_5.8"] } }]
            }
        });
        let (listings, next) = library_page(&json!({ "entries": [entry], "next_cursor": "c2" }), "5.8.4");
        assert_eq!(listings.len(), 1);
        assert!(listings[0].owned && listings[0].compatible);
        assert_eq!(listings[0].seller_name, "Epic Games", "la biblioteca trae el vendedor en publisher");
        assert_eq!(listings[0].fab_url, "https://www.fab.com/listings/l1", "sin fabUrl, el enlace sale del uid");
        assert_eq!(listings[0].thumbnail.as_deref(), Some("https://media.fab.com/a.jpg"));
        assert_eq!(next.as_deref(), Some("c2"));
        let text = listings_text(&listings, next.as_deref(), 0, "UE_5.8");
        assert!(text.contains("https://www.fab.com/listings/l1") && text.contains("Never state a total"));
        assert!(!text.contains("media.fab.com"), "la miniatura va a la tarjeta, no al modelo");
    }

    /// @spec REQ-104 Si el trabajo no acaba a tiempo, se cancela.
    #[test]
    fn un_trabajo_que_no_acaba_se_cancela() {
        let mut replies: VecDeque<Value> = vec![json!({ "success": true, "job_id": "j9" })].into();
        replies.extend(std::iter::repeat_n(json!({ "state": "running" }), 50));
        let mut editor = FakeEditor { replies, calls: Vec::new() };
        let out = run_job(&mut editor, "install", &json!({}), Duration::from_millis(500));
        assert!(out.is_err());
        assert_eq!(editor.calls.last().unwrap().0, "fab_cancel");
    }
}
