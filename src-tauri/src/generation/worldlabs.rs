// =============================================================================
// DBV Unreal AI — Cliente de la API de World Labs (Marble v1)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-18, RNF-04
//
// API Marble verificada en docs.worldlabs.ai (2026-10-26):
//   POST /marble/v1/worlds:generate     → {name: "operations/op_..."}
//   GET  /marble/v1/operations/{id}     → {done: true, response: {world_id: "..."}}
//   GET  /marble/v1/worlds/{id}         → {id, assets: {thumbnail_url, mesh, splats}, semantics_metadata}
//   POST /marble/v1/worlds/{id}:export  → {name: "operations/op_..."} (HQ mesh, 3500 créditos)
//   GET  /marble/v1/credits             → {credits: ...}

use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::unreal::UnrealError;

pub const BASE_URL: &str = "https://api.worldlabs.ai";
pub const DEFAULT_MODEL: &str = "marble-1.1";
pub const DRAFT_MODEL: &str = "marble-1.0-draft";

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

    /// POST /marble/v1/worlds:generate
    pub fn generate_world(&self, prompt: &str, model: Option<&str>, image_base64: Option<&str>) -> Result<String, UnrealError> {
        let mut body = Map::new();
        body.insert("prompt".into(), json!(prompt.trim()));
        body.insert("model".into(), json!(model.unwrap_or(DEFAULT_MODEL)));
        if let Some(b64) = image_base64 {
            body.insert("image_prompt".into(), json!({ "data_base64": b64 }));
        }

        let url = format!("{}/marble/v1/worlds:generate", self.base_url);
        let mut response = Self::agent()
            .post(&url)
            .header("Wl-Api-Key", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send_json(&Value::Object(body))
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo conectar con World Labs: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON de World Labs: {e}")))?;
            let op_name = json.get("name")
                .or_else(|| json.get("operation_id"))
                .or_else(|| json.get("id"))
                .and_then(Value::as_str)
                .ok_or_else(|| UnrealError::Protocol("World Labs respondió sin name/operation_id".into()))?;
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
            .header("Wl-Api-Key", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo consultar la operación {operation_id}: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON de World Labs: {e}")))?;
            let done = json.get("done").and_then(Value::as_bool).unwrap_or(false);
            let progress = json.get("progress").and_then(Value::as_f64);
            let world_id = json.pointer("/response/world_id")
                .or_else(|| json.pointer("/response/id"))
                .or_else(|| json.get("world_id"))
                .or_else(|| json.get("id"))
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
            .header("Wl-Api-Key", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
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
        let body = json!({ "type": "mesh", "format": "glb" });
        let mut response = Self::agent()
            .post(&url)
            .header("Wl-Api-Key", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo exportar la malla de {world_id}: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON: {e}")))?;
            let op_name = json.get("name")
                .or_else(|| json.get("operation_id"))
                .or_else(|| json.get("id"))
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
            .header("Wl-Api-Key", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo consultar el saldo de World Labs: {error}")))?;

        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let result = if (200..300).contains(&status) {
            let json: Value = serde_json::from_str(&text).map_err(|e| UnrealError::Protocol(format!("respuesta no JSON: {e}")))?;
            let amount = json.get("credits")
                .or_else(|| json.get("balance"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            Ok(WorldLabsCredits { balance: amount })
        } else {
            Err(Self::error(status, &text))
        };
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::mcp_client::tests::{fake_server, http};

    #[test]
    fn lanza_consulta_y_obtiene_mundo_simulado() {
        let (url, handle) = fake_server(vec![
            http("200 OK", "", r#"{"name":"operations/op_world_42"}"#),
            http("200 OK", "", r#"{"done":true,"response":{"world_id":"w_88"}}"#),
            http("200 OK", "", r#"{"id":"w_88","assets":{"splats":{"spz_urls":{"full_res":"https://cdn/w.spz"}},"mesh":{"collider_mesh_url":"https://cdn/c.glb"}},"semantics_metadata":{"metric_scale_factor":1.2}}"#),
            http("200 OK", "", r#"{"credits":4200.0}"#),
        ]);
        let base = url.trim_end_matches("/mcp");
        let client = WorldLabsClient::with_base_url("wl-test-key", base);

        let op = client.generate_world("Sci-Fi colony", Some("marble-1.1"), None).unwrap();
        assert_eq!(op, "op_world_42");

        let status = client.operation(&op).unwrap();
        assert!(status.done);
        assert_eq!(status.world_id.as_deref(), Some("w_88"));

        let world = client.world("w_88").unwrap();
        assert_eq!(world.get("id").and_then(Value::as_str), Some("w_88"));

        let creds = client.credits().unwrap();
        assert_eq!(creds.balance, 4200.0);

        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("POST /marble/v1/worlds:generate"));
        assert!(seen[0].contains("wl-test-key"));
        assert!(seen[1].starts_with("GET /marble/v1/operations/op_world_42"));
        assert!(seen[2].starts_with("GET /marble/v1/worlds/w_88"));
        assert!(seen[3].starts_with("GET /marble/v1/credits"));
    }
}
