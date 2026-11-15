// =============================================================================
// DBV Unreal AI — Catálogo de Fab sin sesión (búsqueda y ficha)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-102, REQ-108, REQ-117
//
// La búsqueda y la ficha de Fab no necesitan sesión, así que van desde Rust sin el editor abierto
// (`GET https://www.fab.com/p/agents/listings/...`, la API que usa el FabCatalogClient de UE 5.9). Lo que
// necesita sesión (biblioteca, download-info, instalar) va en el plugin C++, dentro del editor (REQ-103).
//
// Cada listing se reduce a `FabListing`, la misma forma para la vista, la IA y el `--mcp`: con una sola
// miniatura (320 px) y el enlace a fab.com, para que el usuario lo vea antes de instalar (REQ-117), y con los
// formatos que se pueden instalar en la versión del editor (decisión del usuario, 2026-11-15). Fab no filtra
// por versión en el servidor, así que el filtro se aplica aquí.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod commands;
pub mod tools;

pub const API_HOST: &str = "https://www.fab.com";
pub const LISTING_URL_PREFIX: &str = "https://www.fab.com/listings/";
pub const THUMBNAIL_URL_PREFIX: &str = "https://media.fab.com/";
/// Ancho de la miniatura que se ofrece: cabe en una tarjeta y pesa poco (unos 13 KB).
pub const THUMBNAIL_WIDTH: i64 = 320;
pub const MAX_QUERY_CHARS: usize = 500;
pub const MAX_COUNT: u32 = 24;
/// Formatos que se pueden instalar en un proyecto (REQ-102, REQ-104).
pub const INSTALLABLE_FORMATS: [&str; 6] = ["unreal-engine", "glb", "gltf", "fbx", "converted-files", "metahuman"];
/// Formatos de fichero, que no dependen de la versión del motor.
const VERSIONLESS_FORMATS: [&str; 4] = ["glb", "gltf", "fbx", "converted-files"];

/// Error de Fab con un código estable para la IA y la vista, y el mensaje que mandó el servidor.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FabError {
    pub code: String,
    pub message: String,
}

impl FabError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.to_string(), message: message.into() }
    }
}

impl std::fmt::Display for FabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Filtros de la búsqueda. Lo vacío no se envía.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchQuery {
    pub q: String,
    /// Vacío → los formatos instalables (`INSTALLABLE_FORMATS`).
    pub asset_formats: Vec<String>,
    pub listing_types: Vec<String>,
    pub categories: Vec<String>,
    pub sellers: Vec<String>,
    pub licenses: Vec<String>,
    pub sort_by: String,
    pub is_free: bool,
    pub count: Option<u32>,
    pub cursor: String,
    /// Versión del editor (`5.8.4`); sin ella no se filtra por compatibilidad.
    pub engine_version: String,
    /// Enseñar también los listings sin ningún formato compatible (marcados y sin «Instalar»).
    pub include_incompatible: bool,
}

/// Un listing en forma compacta (REQ-102, REQ-117).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FabListing {
    pub uid: String,
    pub title: String,
    pub seller_name: String,
    pub seller_id: String,
    pub is_free: bool,
    /// Está en la biblioteca del usuario (solo se sabe con sesión: `fab_my_library`).
    pub owned: bool,
    /// `None` = «sin valoraciones» (nunca 0).
    pub rating: Option<f64>,
    pub rating_count: u64,
    pub category: String,
    pub listing_type: String,
    pub fab_url: String,
    /// Solo de `https://media.fab.com/`; si no, `None` y la tarjeta pone un marcador.
    pub thumbnail: Option<String>,
    pub mature: bool,
    /// Formatos instalables que trae el listing.
    pub formats: Vec<String>,
    /// De esos, los que valen para la versión del editor.
    pub compatible_formats: Vec<String>,
    pub compatible: bool,
    /// Versiones de Unreal del formato `unreal-engine` (`UE_5.8`…).
    pub ue_versions: Vec<String>,
}

/// Una página de resultados. Nunca lleva un total: Fab no lo da (REQ-102).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub listings: Vec<FabListing>,
    pub next_cursor: Option<String>,
    /// Cuántos de ESTA página se han ocultado por no ser compatibles con el editor.
    pub hidden_incompatible: usize,
    /// La etiqueta de versión que se usó (`UE_5.8`), o vacía si no se filtró.
    pub engine_tag: String,
}

/// `5.8.4` → `UE_5.8`, el formato de Fab (con `5.8`, `download-info` responde 400). @spec REQ-104
pub fn engine_tag(version: &str) -> Option<String> {
    let mut parts = version.trim().trim_start_matches("UE_").split('.');
    let major = parts.next().filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    let minor = parts.next().map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>()).filter(|p| !p.is_empty());
    let tag = match (major, minor) {
        (Some(major), Some(minor)) => Some(format!("UE_{major}.{minor}")),
        _ => None,
    };
    tag
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// URL de `listings/search` con los filtros. @spec REQ-102
pub fn search_url(query: &SearchQuery) -> Result<String, FabError> {
    let result = if query.q.chars().count() > MAX_QUERY_CHARS {
        Err(FabError::new("bad_query", format!("The search text is longer than {MAX_QUERY_CHARS} characters.")))
    } else {
        let formats = if query.asset_formats.is_empty() { INSTALLABLE_FORMATS.iter().map(|f| f.to_string()).collect() } else { query.asset_formats.clone() };
        let count = query.count.unwrap_or(MAX_COUNT).clamp(1, MAX_COUNT).to_string();
        let mut pairs: Vec<(&str, String)> = vec![("q", query.q.trim().to_string()), ("asset_formats", formats.join(","))];
        pairs.push(("listing_types", query.listing_types.join(",")));
        pairs.push(("categories", query.categories.join(",")));
        pairs.push(("sellers", query.sellers.join(",")));
        pairs.push(("licenses", query.licenses.join(",")));
        pairs.push(("sort_by", query.sort_by.clone()));
        if query.is_free {
            pairs.push(("is_free", "true".to_string()));
        }
        pairs.push(("count", count));
        pairs.push(("cursor", query.cursor.clone()));
        let params: Vec<String> = pairs.into_iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!("{k}={}", encode(&v))).collect();
        Ok(format!("{API_HOST}/p/agents/listings/search?{}", params.join("&")))
    };
    result
}

/// URL de la ficha de un listing; el uid solo admite letras, números y guiones.
pub fn listing_url(uid: &str) -> Result<String, FabError> {
    let valid = !uid.is_empty() && uid.len() <= 64 && uid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let result = if valid { Ok(format!("{API_HOST}/p/agents/listings/{uid}")) } else { Err(FabError::new("bad_listing", "The listing id is not valid.")) };
    result
}

fn text(value: &Value, pointer: &str) -> String {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// La miniatura más cercana a 320 px, solo si es de `media.fab.com`.
fn thumbnail(listing: &Value) -> Option<String> {
    let images = listing.pointer("/thumbnails/0/images").and_then(Value::as_array).cloned().unwrap_or_default();
    let best = images
        .iter()
        .filter_map(|image| Some((image.get("width")?.as_i64()?, image.get("url")?.as_str()?.to_string())))
        .filter(|(_, url)| url.starts_with(THUMBNAIL_URL_PREFIX))
        .min_by_key(|(width, _)| (width - THUMBNAIL_WIDTH).abs())
        .map(|(_, url)| url);
    best
}

/// ¿Vale este formato en la versión del editor? Sin versión, todo vale.
fn format_compatible(code: &str, ue_versions: &[String], engine: Option<&str>) -> bool {
    let compatible = match engine {
        None => true,
        Some(tag) if code == "unreal-engine" => ue_versions.iter().any(|v| v == tag),
        Some(_) => VERSIONLESS_FORMATS.contains(&code),
    };
    compatible
}

/// Reduce un listing de Fab (`listings/search` o `listings/{uid}`) a `FabListing`. @spec REQ-102, REQ-117
pub fn compact_listing(listing: &Value, engine: Option<&str>) -> FabListing {
    let uid = text(listing, "/uid");
    let mut formats = Vec::new();
    let mut ue_versions = Vec::new();
    for format in listing.get("assetFormats").and_then(Value::as_array).cloned().unwrap_or_default() {
        let code = text(&format, "/assetFormatType/code");
        if code == "unreal-engine" {
            ue_versions = format.pointer("/technicalSpecs/unrealEngineEngineVersions").and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
        }
        if INSTALLABLE_FORMATS.contains(&code.as_str()) && !formats.contains(&code) {
            formats.push(code);
        }
    }
    let compatible_formats: Vec<String> = formats.iter().filter(|code| format_compatible(code, &ue_versions, engine)).cloned().collect();
    let fab_url = Some(text(listing, "/fabUrl")).filter(|url| url.starts_with(LISTING_URL_PREFIX)).unwrap_or_else(|| format!("{LISTING_URL_PREFIX}{uid}"));
    let rating_count = listing.pointer("/ratings/total").and_then(Value::as_u64).unwrap_or(0);
    let rating = listing.pointer("/ratings/averageRating").and_then(Value::as_f64).filter(|_| rating_count > 0);
    let compact = FabListing {
        title: text(listing, "/title"),
        // El catálogo lo trae en `user`; la biblioteca, en `publisher`.
        seller_name: Some(text(listing, "/user/sellerName")).filter(|s| !s.is_empty()).unwrap_or_else(|| text(listing, "/publisher/sellerName")),
        seller_id: Some(text(listing, "/user/sellerId")).filter(|s| !s.is_empty()).unwrap_or_else(|| text(listing, "/publisher/sellerId")),
        is_free: listing.get("isFree").and_then(Value::as_bool).unwrap_or(false),
        owned: false,
        rating,
        rating_count,
        category: text(listing, "/category/name"),
        listing_type: text(listing, "/listingType"),
        fab_url,
        thumbnail: thumbnail(listing),
        mature: listing.get("isMature").and_then(Value::as_bool).unwrap_or(false),
        compatible: !compatible_formats.is_empty(),
        compatible_formats,
        formats,
        ue_versions,
        uid,
    };
    compact
}

/// Respuesta de `listings/search` → página, con el filtro de compatibilidad. @spec REQ-102
pub fn parse_search(body: &Value, engine_version: &str, include_incompatible: bool) -> SearchPage {
    let tag = engine_tag(engine_version);
    let all: Vec<FabListing> = body.get("results").and_then(Value::as_array).map(|list| list.iter().map(|item| compact_listing(item, tag.as_deref())).collect()).unwrap_or_default();
    let total = all.len();
    let listings: Vec<FabListing> = all.into_iter().filter(|listing| include_incompatible || listing.compatible).collect();
    let page = SearchPage {
        hidden_incompatible: total - listings.len(),
        listings,
        next_cursor: body.pointer("/cursors/next").and_then(Value::as_str).filter(|c| !c.is_empty()).map(String::from),
        engine_tag: tag.unwrap_or_default(),
    };
    page
}

/// Error de Fab a partir del estado y del cuerpo, con el mensaje que manda el servidor. @spec REQ-102
pub fn error_from(status: u16, body: &str) -> FabError {
    let server = serde_json::from_str::<Value>(body).ok().and_then(|v| v.get("detail").or_else(|| v.get("message")).cloned()).map(|d| match d {
        Value::String(s) => s,
        other => other.to_string(),
    });
    let detail = server.unwrap_or_else(|| body.chars().take(300).collect());
    let error = match status {
        400 => FabError::new("bad_request", format!("Fab rejected the search: {detail}")),
        403 => FabError::new("forbidden", format!("Fab does not show this listing (it may be mature content hidden by your Fab settings): {detail}")),
        404 => FabError::new("not_found", format!("This listing does not exist or is not published: {detail}")),
        429 => FabError::new("rate_limited", "Fab is receiving too many searches. Wait a minute and try again."),
        0 => FabError::new("network", format!("Could not reach Fab: {detail}")),
        _ => FabError::new("http_error", format!("Fab answered HTTP {status}: {detail}")),
    };
    error
}

fn get_json(url: &str) -> Result<Value, FabError> {
    let agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_connect(Some(Duration::from_secs(10))).timeout_global(Some(Duration::from_secs(30))).build().new_agent();
    let result = match agent.get(url).header("accept", "application/json").call() {
        Err(error) => Err(error_from(0, &error.to_string())),
        Ok(mut response) => {
            let status = response.status().as_u16();
            let body = response.body_mut().read_to_string().unwrap_or_default();
            if (200..300).contains(&status) {
                serde_json::from_str(&body).map_err(|e| FabError::new("bad_response", format!("Fab answered something that is not JSON: {e}")))
            } else {
                Err(error_from(status, &body))
            }
        }
    };
    result
}

/// La miniatura de un listing para la Biblioteca (REQ-107): solo de `https://media.fab.com/` y como mucho 2 MB.
pub fn fetch_thumbnail(url: &str) -> Option<Vec<u8>> {
    let agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(15))).build().new_agent();
    let bytes = url.starts_with(THUMBNAIL_URL_PREFIX).then(|| agent.get(url).call().ok()).flatten().filter(|r| r.status().is_success()).and_then(|mut r| r.body_mut().with_config().limit(2 * 1024 * 1024).read_to_vec().ok());
    bytes
}

/// Busca en el catálogo de Fab sin sesión. @spec REQ-102
pub fn search(query: &SearchQuery) -> Result<SearchPage, FabError> {
    let result = search_url(query).and_then(|url| get_json(&url)).map(|body| parse_search(&body, &query.engine_version, query.include_incompatible));
    result
}

/// Ficha de un listing: la forma compacta (para la tarjeta) y la misma con la descripción (sin HTML) y las licencias. @spec REQ-102
pub fn listing_details(uid: &str, engine_version: &str) -> Result<(FabListing, Value), FabError> {
    let result = listing_url(uid).and_then(|url| get_json(&url)).map(|body| {
        let tag = engine_tag(engine_version);
        let compact = compact_listing(&body, tag.as_deref());
        let mut value = serde_json::to_value(&compact).unwrap_or(Value::Null);
        if let Some(object) = value.as_object_mut() {
            object.insert("description".into(), Value::String(strip_html(&text(&body, "/description")).chars().take(1500).collect()));
            let licenses: Vec<Value> = body.get("licenses").and_then(Value::as_array).map(|list| list.iter().filter_map(|l| l.get("name").cloned()).collect()).unwrap_or_default();
            object.insert("licenses".into(), Value::Array(licenses));
            object.remove("thumbnail");
        }
        (compact, value)
    });
    result
}

/// Quita las etiquetas HTML de la descripción de Fab (la vista la pinta como texto).
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    let collapsed = out.replace("&nbsp;", " ").replace("&amp;", "&").split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/fab/search_car.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// @spec REQ-104 La etiqueta que pide Fab.
    #[test]
    fn la_version_del_editor_pasa_al_formato_de_fab() {
        assert_eq!(engine_tag("5.8.4").as_deref(), Some("UE_5.8"));
        assert_eq!(engine_tag("5.9.0-60351404+++UE5").as_deref(), Some("UE_5.9"));
        assert_eq!(engine_tag("UE_5.8").as_deref(), Some("UE_5.8"));
        assert_eq!(engine_tag(""), None);
        assert_eq!(engine_tag("abc"), None);
    }

    /// @spec REQ-102 URL con filtros, formatos instalables por defecto y tope de 24.
    #[test]
    fn la_busqueda_construye_la_url_de_fab() {
        let url = search_url(&SearchQuery { q: "coche rojo".into(), is_free: true, count: Some(99), ..Default::default() }).unwrap();
        assert!(url.starts_with("https://www.fab.com/p/agents/listings/search?q=coche%20rojo&asset_formats=unreal-engine%2Cglb%2Cgltf%2Cfbx%2Cconverted-files%2Cmetahuman"));
        assert!(url.contains("&is_free=true") && url.contains("&count=24"));
        assert!(!url.contains("cursor=") && !url.contains("sellers="));
        let paged = search_url(&SearchQuery { q: "car".into(), cursor: "bz0xMg==".into(), sellers: vec!["o-abc".into()], ..Default::default() }).unwrap();
        assert!(paged.contains("&sellers=o-abc") && paged.contains("&cursor=bz0xMg%3D%3D"));
        assert_eq!(search_url(&SearchQuery { q: "x".repeat(501), ..Default::default() }).unwrap_err().code, "bad_query");
    }

    /// @spec REQ-102, REQ-117 Miniatura de 320 px, enlace y valoración con los datos reales grabados.
    #[test]
    fn el_listing_compacto_trae_miniatura_y_enlace() {
        let page = parse_search(&fixture(), "", true);
        let first = &page.listings[0];
        assert_eq!(first.title, "Camo muscle car");
        assert_eq!(first.fab_url, "https://www.fab.com/listings/c78fc5db-a5ea-4586-970a-5d279a3908e7");
        let thumb = first.thumbnail.as_deref().unwrap();
        assert!(thumb.starts_with("https://media.fab.com/"));
        assert_eq!(first.formats, vec!["fbx"]);
        assert_eq!(first.rating, Some(5.0));
        assert!(first.is_free && !first.mature);
        assert_eq!(page.next_cursor.as_deref(), Some("bz0xMg=="));
        assert!(!page.listings[3].is_free);
    }

    /// @spec REQ-102 Solo lo compatible con UE 5.8: `unreal-engine` con `UE_5.8`, o un formato de fichero.
    #[test]
    fn en_5_8_solo_quedan_los_compatibles() {
        let page = parse_search(&fixture(), "5.8.4", false);
        let titles: Vec<&str> = page.listings.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(titles, vec!["Camo muscle car", "Lowpoly Car", "Car Asset Sedan"]);
        assert_eq!(page.hidden_incompatible, 1, "Driveable Car solo llega a UE_4.27");
        let lowpoly = &page.listings[1];
        assert!(lowpoly.formats.contains(&"unreal-engine".to_string()));
        assert_eq!(lowpoly.compatible_formats, vec!["fbx", "converted-files"], "UE solo en 5.6 y 5.7: en 5.8 vale el fbx");
        assert_eq!(page.listings[2].compatible_formats, vec!["unreal-engine"]);
        assert_eq!(page.engine_tag, "UE_5.8");
        let all = parse_search(&fixture(), "5.8.4", true);
        let driveable = all.listings.iter().find(|l| l.title.starts_with("Driveable")).unwrap();
        assert!(!driveable.compatible && driveable.compatible_formats.is_empty());
    }

    /// @spec REQ-117 Una miniatura de otro dominio no se usa y el enlace se rehace desde el uid.
    #[test]
    fn miniatura_y_enlace_solo_de_fab() {
        let listing = json!({
            "uid": "abc-123", "title": "X", "fabUrl": "https://evil.example/listings/abc-123", "isMature": true,
            "ratings": {"averageRating": 0.0, "total": 0},
            "thumbnails": [{"images": [{"url": "https://evil.example/a.jpg", "width": 320}]}],
            "assetFormats": [{"assetFormatType": {"code": "glb"}}]
        });
        let compact = compact_listing(&listing, Some("UE_5.8"));
        assert_eq!(compact.thumbnail, None);
        assert_eq!(compact.fab_url, "https://www.fab.com/listings/abc-123");
        assert_eq!(compact.rating, None, "sin valoraciones, nunca 0");
        assert!(compact.mature && compact.compatible);
    }

    /// @spec REQ-102 Errores con el mensaje del servidor.
    #[test]
    fn los_errores_de_fab_se_explican() {
        let bad = error_from(400, r#"{"detail":{"sort_by":["Unknown value."]}}"#);
        assert_eq!(bad.code, "bad_request");
        assert!(bad.message.contains("sort_by"));
        assert_eq!(error_from(429, "").code, "rate_limited");
        assert_eq!(error_from(404, r#"{"detail":"Not found."}"#).code, "not_found");
        assert_eq!(error_from(403, "").code, "forbidden");
        assert_eq!(listing_url("../x").unwrap_err().code, "bad_listing");
    }

    /// @spec REQ-102 En vivo contra fab.com (sin sesión): `cargo test fab:: -- --ignored`.
    #[test]
    #[ignore]
    fn en_vivo_un_coche_gratis_compatible_con_5_8() {
        let page = search(&SearchQuery { q: "car".into(), is_free: true, count: Some(12), engine_version: "5.8.4".into(), ..Default::default() }).unwrap();
        assert!(!page.listings.is_empty());
        assert!(page.listings.iter().all(|l| l.compatible && l.fab_url.starts_with(LISTING_URL_PREFIX)));
        assert!(page.listings.iter().any(|l| l.thumbnail.is_some()));
        let (card, details) = listing_details(&page.listings[0].uid, "5.8.4").unwrap();
        assert!(details.get("description").is_some());
        assert_eq!(card.uid, page.listings[0].uid);
    }

    #[test]
    fn la_descripcion_pierde_el_html() {
        assert_eq!(strip_html("<p>Hola&nbsp;<strong>mundo</strong></p><ul><li>uno</li></ul>"), "Hola mundo uno");
    }
}
