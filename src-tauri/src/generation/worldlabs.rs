// =============================================================================
// DBV Unreal AI — Cliente de la API de World Labs (Marble v1)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-18, RNF-04
//
// API Marble según la referencia de docs.worldlabs.ai/api/reference (revisada el 2026-11-01: la versión anterior
// de este cliente usaba una cabecera y unos cuerpos supuestos y World Labs respondía «No API key found»).
// Autenticación: cabecera `WLT-Api-Key: <clave>`.
//   POST /marble/v1/worlds:generate     {display_name, model, world_prompt: {type: "text"|"image", text_prompt, image_prompt}}
//                                       → {operation_id, done, metadata: {progress}}
//   GET  /marble/v1/operations/{id}     → {done, metadata: {progress, world_id}, response: {world_id, ...}, error: {code, message}}
//   GET  /marble/v1/worlds/{id}         → {world_id, display_name, assets: {thumbnail_url, imagery, mesh, splats: {spz_urls, semantics_metadata}}}
//   POST /marble/v1/worlds/{id}:export  {asset_type: "mesh", format: "glb"} → {operation_id, done} (malla HQ, 3500 créditos)
//   GET  /marble/v1/credits             → {remaining_credits}

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::unreal::UnrealError;

pub const BASE_URL: &str = "https://api.worldlabs.ai";
pub const DEFAULT_MODEL: &str = "marble-1.1";
pub const DRAFT_MODEL: &str = "marble-1.0-draft";
/// Cabecera de la clave (no `Authorization: Bearer`).
pub const API_KEY_HEADER: &str = "WLT-Api-Key";
/// Límite de `display_name` en la API.
const MAX_DISPLAY_NAME: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldLabsOperation {
    pub id: String,
    pub done: bool,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub world_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldLabsCredits {
    pub balance: f64,
}

pub struct WorldLabsClient {
    pub base_url: String,
    key: String,
}

impl WorldLabsClient {
    pub fn new(key: &str) -> Self {
        Self { base_url: BASE_URL.to_string(), key: key.to_string() }
    }

    pub fn with_base_url(key: &str, base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string(), key: key.to_string() }
    }

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .new_agent()
    }

    fn error(status: u16, text: &str) -> UnrealError {
        let parsed: Option<Value> = serde_json::from_str(text).ok();
        let detail = parsed
            .as_ref()
            .and_then(|v| {
                v.pointer("/error/message")
                    .or_else(|| v.get("message"))
                    .or_else(|| v.get("error"))
                    .and_then(Value::as_str)
            })
            .map(str::to_string)
            .unwrap_or_else(|| text.chars().take(300).collect());
        UnrealError::Protocol(format!("World Labs respondió HTTP {status}: {detail}"))
    }

    /// POST /marble/v1/worlds:generate. `image`: la imagen de referencia en base64 y su extensión (`jpg`, `png`…).
    pub fn generate_world(&self, prompt: &str, model: Option<&str>, image: Option<(&str, &str)>, display_name: Option<&str>) -> Result<String, UnrealError> {
        let body = generate_body(prompt, model, image, display_name);
        let url = format!("{}/marble/v1/worlds:generate", self.base_url);
        let mut response = Self::agent()
            .post(&url)
            .header(API_KEY_HEADER, &self.key)
            .header("Content-Type", "application/json")
            .send_json(body)
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo conectar con World Labs: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON de World Labs: {e}")))?;
            let op_name = json.get("operation_id")
                .or_else(|| json.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| UnrealError::Protocol("World Labs respondió sin operation_id".into()))?;
            let cleaned = op_name.strip_prefix("operations/").unwrap_or(op_name).to_string();
            Ok(cleaned)
        } else {
            Err(Self::error(status, &text))
        };
        result
    }

    /// GET /marble/v1/operations/{id}
    pub fn operation(&self, operation_id: &str) -> Result<WorldLabsOperation, UnrealError> {
        let cleaned = operation_id.strip_prefix("operations/").unwrap_or(operation_id);
        let url = format!("{}/marble/v1/operations/{cleaned}", self.base_url);
        let mut response = Self::agent()
            .get(&url)
            .header(API_KEY_HEADER, &self.key)
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo consultar la operación {operation_id}: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON de World Labs: {e}")))?;
            let done = json.get("done").and_then(Value::as_bool).unwrap_or(false);
            let progress = json.pointer("/metadata/progress").and_then(Value::as_f64);
            let world_id = json.pointer("/response/world_id")
                .or_else(|| json.pointer("/metadata/world_id"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let error = json.pointer("/error/message")
                .or_else(|| json.get("error"))
                .and_then(Value::as_str)
                .map(str::to_string);
            Ok(WorldLabsOperation {
                id: cleaned.to_string(),
                done,
                progress,
                world_id,
                error,
            })
        } else {
            Err(Self::error(status, &text))
        };
        result
    }

    /// GET /marble/v1/worlds/{id}
    pub fn world(&self, world_id: &str) -> Result<Value, UnrealError> {
        let url = format!("{}/marble/v1/worlds/{world_id}", self.base_url);
        let mut response = Self::agent()
            .get(&url)
            .header(API_KEY_HEADER, &self.key)
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo obtener el mundo {world_id}: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON del mundo: {e}")))?;
            Ok(json)
        } else {
            Err(Self::error(status, &text))
        };
        result
    }

    /// POST /marble/v1/worlds/{id}:export
    pub fn export_mesh(&self, world_id: &str) -> Result<String, UnrealError> {
        let url = format!("{}/marble/v1/worlds/{world_id}:export", self.base_url);
        let body = json!({ "asset_type": "mesh", "format": "glb" });
        let mut response = Self::agent()
            .post(&url)
            .header(API_KEY_HEADER, &self.key)
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo exportar la malla de {world_id}: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON: {e}")))?;
            let op_name = json.get("operation_id")
                .or_else(|| json.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| UnrealError::Protocol("sin id de operación al exportar".into()))?;
            let cleaned = op_name.strip_prefix("operations/").unwrap_or(op_name).to_string();
            Ok(cleaned)
        } else {
            Err(Self::error(status, &text))
        };
        result
    }

    /// GET /marble/v1/credits
    pub fn credits(&self) -> Result<WorldLabsCredits, UnrealError> {
        let url = format!("{}/marble/v1/credits", self.base_url);
        let mut response = Self::agent()
            .get(&url)
            .header(API_KEY_HEADER, &self.key)
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo consultar el saldo de World Labs: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON: {e}")))?;
            let amount = json.get("remaining_credits")
                .and_then(Value::as_f64)
                .ok_or_else(|| UnrealError::Protocol(format!("World Labs no devolvió remaining_credits: {}", text.chars().take(200).collect::<String>())))?;
            Ok(WorldLabsCredits { balance: amount })
        } else {
            Err(Self::error(status, &text))
        };
        result
    }
}

/// Cuerpo de `worlds:generate`: texto, o imagen con texto opcional (el `world_prompt` de la API).
fn generate_body(prompt: &str, model: Option<&str>, image: Option<(&str, &str)>, display_name: Option<&str>) -> Value {
    let prompt = prompt.trim();
    let mut world_prompt = Map::new();
    match image {
        Some((data, extension)) => {
            world_prompt.insert("type".into(), json!("image"));
            world_prompt.insert("image_prompt".into(), json!({ "source": "data_base64", "data_base64": data, "extension": extension }));
            if !prompt.is_empty() {
                world_prompt.insert("text_prompt".into(), json!(prompt));
            }
        }
        None => {
            world_prompt.insert("type".into(), json!("text"));
            world_prompt.insert("text_prompt".into(), json!(prompt));
        }
    }
    let mut body = Map::new();
    body.insert("model".into(), json!(model.unwrap_or(DEFAULT_MODEL)));
    body.insert("world_prompt".into(), Value::Object(world_prompt));
    if let Some(name) = display_name.map(str::trim).filter(|n| !n.is_empty()) {
        body.insert("display_name".into(), json!(name.chars().take(MAX_DISPLAY_NAME).collect::<String>()));
    }
    Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::mcp_client::tests::{fake_server, http};

    #[test]
    fn lanza_consulta_y_obtiene_mundo_simulado() {
        let (url, handle) = fake_server(vec![
            http("200 OK", "", r#"{"operation_id":"op_world_42","done":false,"metadata":{"progress":0}}"#),
            http("200 OK", "", r#"{"operation_id":"op_world_42","done":true,"metadata":{"progress":100,"world_id":"w_88"},"response":{"world_id":"w_88","display_name":"Colonia","world_marble_url":"https://marble/w_88"},"error":null}"#),
            http("200 OK", "", r#"{"world_id":"w_88","display_name":"Colonia","assets":{"splats":{"spz_urls":{"full_res":"https://cdn/w.spz"},"semantics_metadata":{"metric_scale_factor":1.2,"ground_plane_offset":0.1}},"mesh":{"collider_mesh_url":"https://cdn/c.glb"}}}"#),
            http("200 OK", "", r#"{"remaining_credits":4200.0}"#),
        ]);
        let base = url.trim_end_matches("/mcp");
        let client = WorldLabsClient::with_base_url("wl-test-key", base);

        let op = client.generate_world("Sci-Fi colony", Some("marble-1.1"), None, Some("Colonia")).unwrap();
        assert_eq!(op, "op_world_42");

        let status = client.operation(&op).unwrap();
        assert!(status.done);
        assert_eq!(status.world_id.as_deref(), Some("w_88"));

        let world = client.world("w_88").unwrap();
        assert_eq!(world.get("world_id").and_then(Value::as_str), Some("w_88"));

        let creds = client.credits().unwrap();
        assert_eq!(creds.balance, 4200.0);

        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("POST /marble/v1/worlds:generate"));
        // Regresión (2026-11-01): con `Wl-Api-Key`/`Bearer` World Labs respondía «No API key found» (ureq la manda en minúsculas).
        assert!(seen.iter().all(|request| request.contains("wlt-api-key: wl-test-key")), "{seen:?}");
        let body: Value = serde_json::from_str(seen[0].split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body, json!({ "model": "marble-1.1", "display_name": "Colonia", "world_prompt": { "type": "text", "text_prompt": "Sci-Fi colony" } }));
        assert!(seen[1].starts_with("GET /marble/v1/operations/op_world_42"));
        assert!(seen[2].starts_with("GET /marble/v1/worlds/w_88"));
        assert!(seen[3].starts_with("GET /marble/v1/credits"));
    }

    /// El `world_prompt` con la forma de la API: texto solo, o imagen en base64 con su extensión y el texto opcional.
    #[test]
    fn cuerpo_de_generacion_con_la_forma_de_la_api() {
        let text = generate_body("  ciudad futurista ", None, None, Some("Ciudad"));
        assert_eq!(text, json!({ "model": DEFAULT_MODEL, "display_name": "Ciudad", "world_prompt": { "type": "text", "text_prompt": "ciudad futurista" } }));
        let image = generate_body("", Some(DRAFT_MODEL), Some(("QUJD", "png")), Some(&"x".repeat(80)));
        assert_eq!(image["world_prompt"], json!({ "type": "image", "image_prompt": { "source": "data_base64", "data_base64": "QUJD", "extension": "png" } }));
        assert_eq!(image["display_name"].as_str().unwrap().len(), MAX_DISPLAY_NAME);
    }

    /// Un saldo sin `remaining_credits` es un error, no «0 créditos».
    #[test]
    fn saldo_sin_remaining_credits_es_error() {
        let (url, handle) = fake_server(vec![http("200 OK", "", r#"{"credits":10}"#)]);
        let client = WorldLabsClient::with_base_url("k", url.trim_end_matches("/mcp"));
        assert!(client.credits().unwrap_err().to_string().contains("remaining_credits"));
        handle.join().unwrap();
    }
}
