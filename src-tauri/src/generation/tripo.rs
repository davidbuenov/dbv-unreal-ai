// =============================================================================
// DBV Unreal AI — Cliente de la API de Tripo 3D (v3)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-15, RNF-04
//
// API v3 verificada en Holo-Companion (jupiterBueno2, 2026-10-06); la v2 de Wordsmith no es
// compatible (REJECTED-01):
//   POST /generation/text-to-model → {code: 0, data: {task_id}}
//   GET  /tasks/{id}               → {code: 0, data: {status, progress, output: {model_url, …}}}
// La URL del modelo es un enlace firmado que CADUCA A LOS 5 MINUTOS: se descarga en cuanto la
// tarea termina. `face_limit` es orientativo (pedir 3 000 dio 4 358 triángulos).
// Imagen → modelo NO está verificado en v3: queda fuera hasta probarlo con una clave real.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::unreal::{io_error, UnrealError};

pub const BASE_URL: &str = "https://openapi.tripo3d.ai/v3";
/// Modelo verificado.
pub const DEFAULT_MODEL: &str = "v3.1-20260211";
/// Coste observado de un texto → modelo (créditos). Es una estimación para avisar ANTES de gastar.
pub const TEXT_TO_MODEL_CREDITS: f64 = 30.0;
/// Tope de una descarga (un GLB con texturas 4K ronda las decenas de MB).
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
/// Estados finales que no son éxito.
pub const FAILED_STATES: [&str; 5] = ["failed", "cancelled", "banned", "expired", "unknown"];

/// Parámetros de texto → modelo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextToModel {
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    /// `standard` o `detailed`.
    #[serde(default)]
    pub texture_quality: Option<String>,
    #[serde(default)]
    pub face_limit: Option<u32>,
    #[serde(default)]
    pub smart_low_poly: Option<bool>,
    /// `false` (por defecto): una sola textura base, más ligera.
    #[serde(default)]
    pub pbr: Option<bool>,
}

impl TextToModel {
    /// Cuerpo de la petición con los valores verificados por defecto.
    pub fn body(&self) -> Value {
        let mut body = Map::new();
        body.insert("prompt".into(), json!(self.prompt.trim()));
        body.insert("model".into(), json!(self.model.clone().unwrap_or_else(|| DEFAULT_MODEL.to_string())));
        body.insert("texture_quality".into(), json!(self.texture_quality.clone().unwrap_or_else(|| "standard".to_string())));
        body.insert("pbr".into(), json!(self.pbr.unwrap_or(false)));
        if let Some(limit) = self.face_limit {
            body.insert("face_limit".into(), json!(limit));
        }
        if let Some(flag) = self.smart_low_poly {
            body.insert("smart_low_poly".into(), json!(flag));
        }
        Value::Object(body)
    }
}

/// Estado de una tarea.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TripoTask {
    pub status: String,
    pub progress: Option<f64>,
    pub model_url: Option<String>,
    pub image_url: Option<String>,
    /// Campos que parecen créditos (`consumed_credit`…), para el registro del gasto.
    pub credits: Option<f64>,
}

impl TripoTask {
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }
    pub fn is_failed(&self) -> bool {
        FAILED_STATES.contains(&self.status.as_str())
    }
}

/// `data` de `GET /tasks/{id}` → `TripoTask` (tolerante con los nombres de v2 y v3).
pub fn parse_task(data: &Value) -> TripoTask {
    let output = data.get("output").cloned().unwrap_or(Value::Null);
    let pick = |keys: &[&str]| keys.iter().find_map(|k| output.get(*k).and_then(Value::as_str).map(str::to_string));
    let credits = [data, &output]
        .iter()
        .flat_map(|v| v.as_object().into_iter().flat_map(|m| m.iter()))
        .find(|(k, v)| k.to_lowercase().contains("credit") && v.is_number())
        .and_then(|(_, v)| v.as_f64());
    TripoTask {
        status: data.get("status").and_then(Value::as_str).unwrap_or("unknown").to_string(),
        progress: data.get("progress").and_then(Value::as_f64),
        model_url: pick(&["model_url", "pbr_model", "model", "base_model"]),
        image_url: pick(&["rendered_image_url", "rendered_image", "image_url"]),
        credits,
    }
}

/// Mensaje de error de Tripo (`message`, `error`, `msg`) o el principio del cuerpo.
fn tripo_error(status: u16, text: &str) -> UnrealError {
    let parsed: Option<Value> = serde_json::from_str(text).ok();
    let detail = parsed
        .as_ref()
        .and_then(|v| ["message", "error", "msg"].iter().find_map(|k| v.get(*k).and_then(Value::as_str)).map(str::to_string))
        .unwrap_or_else(|| text.chars().take(300).collect());
    UnrealError::Protocol(format!("Tripo respondió HTTP {status}: {detail}"))
}

/// Cliente de Tripo. `base_url` se puede cambiar en los tests (servidor simulado).
pub struct TripoClient {
    pub base_url: String,
    key: String,
}

impl TripoClient {
    pub fn new(key: &str) -> Self {
        Self { base_url: BASE_URL.to_string(), key: key.to_string() }
    }

    pub fn with_base_url(key: &str, base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string(), key: key.to_string() }
    }

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder().http_status_as_error(false).timeout_connect(Some(Duration::from_secs(15))).timeout_global(Some(Duration::from_secs(60))).build().new_agent()
    }

    /// `data` de una respuesta `{code: 0, data}`; cualquier otra cosa es un error explicado.
    fn data(status: u16, text: &str) -> Result<Value, UnrealError> {
        let parsed: Option<Value> = serde_json::from_str(text).ok();
        let ok = (200..300).contains(&status) && parsed.as_ref().is_some_and(|v| v.get("code").and_then(Value::as_i64).unwrap_or(0) == 0);
        let result = match parsed {
            Some(value) if ok => Ok(value.get("data").cloned().unwrap_or(value)),
            _ => Err(tripo_error(status, text)),
        };
        result
    }

    /// Lanza un texto → modelo y devuelve el `task_id`.
    pub fn text_to_model(&self, request: &TextToModel) -> Result<String, UnrealError> {
        if request.prompt.trim().is_empty() {
            return Err(UnrealError::Format("el prompt está vacío".into()));
        }
        let url = format!("{}/generation/text-to-model", self.base_url);
        let mut response = Self::agent()
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send_json(request.body())
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo conectar con Tripo: {error}")))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        let data = Self::data(status, &text)?;
        data.get("task_id").and_then(Value::as_str).map(str::to_string).ok_or_else(|| UnrealError::Protocol("Tripo respondió sin task_id".into()))
    }

    /// Estado de una tarea.
    pub fn task(&self, task_id: &str) -> Result<TripoTask, UnrealError> {
        let url = format!("{}/tasks/{}", self.base_url, task_id.replace(['/', '?', '#'], ""));
        let mut response = Self::agent()
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.key))
            .call()
            .map_err(|error| UnrealError::Unreachable(format!("no se pudo consultar la tarea de Tripo: {error}")))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        Ok(parse_task(&Self::data(status, &text)?))
    }
}

/// Descarga un enlace firmado (sin cabecera de autorización) a `target`, con tope de tamaño.
pub fn download(url: &str, target: &Path) -> Result<u64, UnrealError> {
    // Solo HTTPS: el enlace lo da Tripo, y uno manipulado no debe poder apuntar a un servicio local.
    // Los tests sirven el fichero desde un servidor simulado en 127.0.0.1.
    let allowed = url.starts_with("https://") || (cfg!(test) && url.starts_with("http://127.0.0.1"));
    if !allowed {
        return Err(UnrealError::Protocol(format!("enlace de descarga no admitido: {url}")));
    }
    let agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(300))).build().new_agent();
    let response = agent.get(url).call().map_err(|error| UnrealError::Unreachable(format!("no se pudo descargar el modelo: {error}")))?;
    let status = response.status().as_u16();
    let bytes = if status == 200 { read_limited(response, target) } else { Err(UnrealError::Protocol(format!("la descarga respondió HTTP {status} (¿caducó el enlace de 5 minutos?)"))) }?;
    std::fs::write(target, &bytes).map_err(|error| io_error(target, &error))?;
    Ok(bytes.len() as u64)
}

/// Cuerpo de una descarga, como mucho `MAX_DOWNLOAD_BYTES`.
fn read_limited(response: ureq::http::Response<ureq::Body>, target: &Path) -> Result<Vec<u8>, UnrealError> {
    let mut bytes = Vec::new();
    response.into_body().into_reader().take(MAX_DOWNLOAD_BYTES + 1).read_to_end(&mut bytes).map_err(|error| io_error(target, &error))?;
    let result = if bytes.len() as u64 > MAX_DOWNLOAD_BYTES { Err(UnrealError::Protocol("el fichero descargado supera el tope de 512 MB".into())) } else { Ok(bytes) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::mcp_client::tests::{fake_server, http};

    /// @spec REQ-15
    #[test]
    fn el_cuerpo_lleva_los_valores_verificados() {
        let body = TextToModel { prompt: "  cofre  ".into(), model: None, texture_quality: None, face_limit: Some(3000), smart_low_poly: Some(true), pbr: None }.body();
        assert_eq!(body, json!({"prompt": "cofre", "model": "v3.1-20260211", "texture_quality": "standard", "pbr": false, "face_limit": 3000, "smart_low_poly": true}));
    }

    /// @spec REQ-15
    #[test]
    fn interpreta_tareas_de_v3_y_v2_y_sus_creditos() {
        let v3 = parse_task(&json!({"status": "success", "progress": 100, "output": {"model_url": "https://x/m.glb", "rendered_image_url": "https://x/i.webp"}, "consumed_credit": 30}));
        assert!(v3.is_success());
        assert_eq!(v3.model_url.as_deref(), Some("https://x/m.glb"));
        assert_eq!(v3.image_url.as_deref(), Some("https://x/i.webp"));
        assert_eq!(v3.credits, Some(30.0));
        let v2 = parse_task(&json!({"status": "running", "output": {"pbr_model": "https://x/p.glb"}}));
        assert_eq!(v2.model_url.as_deref(), Some("https://x/p.glb"));
        assert!(!v2.is_success() && !v2.is_failed());
        assert!(parse_task(&json!({"status": "banned"})).is_failed());
    }

    /// @spec REQ-15
    #[test]
    fn lanza_y_consulta_contra_un_servidor_simulado() {
        let (url, handle) = fake_server(vec![
            http("200 OK", "", r#"{"code":0,"data":{"task_id":"t-1"}}"#),
            http("200 OK", "", r#"{"code":0,"data":{"status":"success","progress":100,"output":{"model_url":"https://cdn/x.glb"}}}"#),
            http("401 Unauthorized", "", r#"{"code":1002,"message":"invalid api key"}"#),
        ]);
        let base = url.trim_end_matches("/mcp");
        let client = TripoClient::with_base_url("sk-test", base);
        let request = TextToModel { prompt: "cofre".into(), model: None, texture_quality: None, face_limit: None, smart_low_poly: None, pbr: None };
        assert_eq!(client.text_to_model(&request).unwrap(), "t-1");
        assert!(client.task("t-1").unwrap().is_success());
        let error = client.task("t-1").unwrap_err();
        assert!(error.to_string().contains("invalid api key"), "{error}");
        let seen = handle.join().unwrap();
        assert!(seen[0].starts_with("POST /generation/text-to-model"));
        assert!(seen[0].to_lowercase().contains("authorization: bearer sk-test"));
        assert!(seen[1].starts_with("GET /tasks/t-1"));
    }

    /// @spec REQ-15
    #[test]
    fn rechaza_prompts_vacios_y_enlaces_no_https() {
        let client = TripoClient::new("k");
        let empty = TextToModel { prompt: "  ".into(), model: None, texture_quality: None, face_limit: None, smart_low_poly: None, pbr: None };
        assert!(matches!(client.text_to_model(&empty), Err(UnrealError::Format(_))));
        assert!(matches!(download("ftp://x/y.glb", Path::new("y.glb")), Err(UnrealError::Protocol(_))));
    }
}
