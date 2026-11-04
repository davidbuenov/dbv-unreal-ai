// =============================================================================
// DBV Unreal AI — Cliente de generación y búsqueda de audio (SFX)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-71

use std::path::Path;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::unreal::UnrealError;

pub const DEFAULT_LOCAL_AUDIO_URL: &str = "http://127.0.0.1:8765";

/// Estado del microservicio de audio local (Stable Audio Open 1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAudioStatus {
    pub ready: bool,
    pub status: String,
    pub device: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
}

/// Comprueba si el microservicio local de audio está respondiendo.
pub fn check_local_service(base_url: Option<&str>) -> LocalAudioStatus {
    let url = format!("{}/health", base_url.unwrap_or(DEFAULT_LOCAL_AUDIO_URL).trim_end_matches('/'));
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .new_agent();

    match agent.get(&url).call() {
        Ok(mut res) if res.status().as_u16() == 200 => {
            let body: Value = res.body_mut().read_json().unwrap_or(Value::Null);
            LocalAudioStatus {
                ready: body.get("status").and_then(Value::as_str) == Some("ready"),
                status: body.get("status").and_then(Value::as_str).unwrap_or("ready").to_string(),
                device: body.get("device").and_then(Value::as_str).map(str::to_string),
                model: body.get("model").and_then(Value::as_str).map(str::to_string),
                version: body.get("version").and_then(Value::as_str).map(str::to_string),
            }
        }
        _ => LocalAudioStatus {
            ready: false,
            status: "offline".to_string(),
            device: None,
            model: None,
            version: None,
        },
    }
}

/// Genera un efecto de sonido usando el microservicio local (`dbv-audio-service`).
pub fn generate_local_sound(
    base_url: Option<&str>,
    prompt: &str,
    duration: f32,
    steps: u32,
) -> Result<Vec<u8>, UnrealError> {
    let url = format!("{}/v1/generate", base_url.unwrap_or(DEFAULT_LOCAL_AUDIO_URL).trim_end_matches('/'));
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .new_agent();

    let body = json!({
        "prompt": prompt,
        "duration": duration,
        "steps": steps
    });

    let mut res = agent
        .post(&url)
        .header("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| UnrealError::Io(format!("error conectando con el servicio de audio local: {e}")))?;

    if res.status().as_u16() != 200 {
        return Err(UnrealError::Protocol(format!("el servicio de audio devolvió HTTP {}", res.status())));
    }

    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut res.body_mut().as_reader(), &mut bytes)
        .map_err(|e| UnrealError::Io(format!("error leyendo audio generado: {e}")))?;

    Ok(bytes)
}

/// Genera un efecto de sonido usando la API de ElevenLabs.
pub fn generate_elevenlabs_sfx(
    api_key: &str,
    prompt: &str,
    duration_seconds: Option<f32>,
    loop_audio: bool,
) -> Result<Vec<u8>, UnrealError> {
    let url = "https://api.elevenlabs.io/v1/sound-generation";
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .new_agent();

    let mut body = json!({
        "text": prompt,
        "loop": loop_audio,
    });
    if let Some(dur) = duration_seconds {
        body["duration_seconds"] = json!(dur);
    }

    let mut res = agent
        .post(url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| {
            if let ureq::Error::StatusCode(code) = &e {
                match *code {
                    401 => UnrealError::Protocol("clave de API de ElevenLabs inválida o no autorizada (HTTP 401)".into()),
                    429 => UnrealError::Protocol("cuota de ElevenLabs superada o límite alcanzado (HTTP 429)".into()),
                    other => UnrealError::Protocol(format!("ElevenLabs SFX devolvió HTTP {other}")),
                }
            } else {
                UnrealError::Io(format!("error en llamada a ElevenLabs SFX: {e}"))
            }
        })?;

    if res.status().as_u16() != 200 {
        return Err(UnrealError::Protocol(format!("ElevenLabs SFX devolvió HTTP {}", res.status())));
    }

    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut res.body_mut().as_reader(), &mut bytes)
        .map_err(|e| UnrealError::Io(format!("error descargando audio de ElevenLabs: {e}")))?;

    Ok(bytes)
}

/// Resultado de búsqueda en Freesound.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreesoundItem {
    pub id: u64,
    pub name: String,
    pub tags: Vec<String>,
    pub duration: f32,
    pub preview_url: Option<String>,
    pub download_url: Option<String>,
}

/// Busca clips de sonido en Freesound usando un token de API.
pub fn search_freesound(
    token: &str,
    query: &str,
    page_size: usize,
) -> Result<Vec<FreesoundItem>, UnrealError> {
    let encoded_query = query.replace(' ', "+");
    let url = format!(
        "https://freesound.org/apiv2/search/text/?query={}&token={}&fields=id,name,tags,duration,previews,download&page_size={}",
        encoded_query, token, page_size
    );
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent();

    let mut res = agent
        .get(&url)
        .call()
        .map_err(|e| UnrealError::Io(format!("error buscando en Freesound: {e}")))?;

    if res.status().as_u16() != 200 {
        return Err(UnrealError::Protocol(format!("Freesound devolvió HTTP {}", res.status())));
    }

    let body: Value = res.body_mut().read_json().map_err(|e| UnrealError::Format(e.to_string()))?;
    let results = body.get("results").and_then(Value::as_array).cloned().unwrap_or_default();

    let items = results
        .into_iter()
        .filter_map(|r| {
            let id = r.get("id")?.as_u64()?;
            let name = r.get("name").and_then(Value::as_str).unwrap_or("sound").to_string();
            let duration = r.get("duration").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let tags = r
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let preview_url = r
                .get("previews")
                .and_then(|p| p.get("preview-hq-mp3").or_else(|| p.get("preview-lq-mp3")))
                .and_then(Value::as_str)
                .map(str::to_string);
            let download_url = r.get("download").and_then(Value::as_str).map(str::to_string);

            Some(FreesoundItem {
                id,
                name,
                tags,
                duration,
                preview_url,
                download_url,
            })
        })
        .collect();

    Ok(items)
}

/// Guarda un buffer de audio en la biblioteca como un asset `kind: "sound"`.
pub fn save_audio_to_library(
    library_dir: &Path,
    filename: &str,
    audio_bytes: &[u8],
    prompt: &str,
    duration: f32,
    source: &str,
) -> Result<crate::library::LibraryEntry, UnrealError> {
    use std::fs;
    let safe_stem = filename.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
    let final_name = if safe_stem.ends_with(".wav") || safe_stem.ends_with(".mp3") || safe_stem.ends_with(".ogg") {
        safe_stem
    } else {
        format!("{safe_stem}.wav")
    };

    let target_path = library_dir.join(&final_name);
    fs::create_dir_all(library_dir).map_err(|e| UnrealError::Io(e.to_string()))?;
    fs::write(&target_path, audio_bytes).map_err(|e| UnrealError::Io(e.to_string()))?;

    let entry = crate::library::LibraryEntry {
        id: format!("sound_{}", crate::library::now_secs()),
        name: final_name.clone(),
        provider: source.to_string(),
        kind: "sound".to_string(),
        prompt: prompt.to_string(),
        parameters: json!({
            "duration": duration,
            "source": source
        }),
        task_id: String::new(),
        created_at: crate::library::now_secs(),
        credits: None,
        files: vec![final_name],
        thumbnail: None,
        sent_to: Vec::new(),
        parent: None,
        output: Value::Null,
    };

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_service_devuelve_offline_si_puerto_no_escucha() {
        let status = check_local_service(Some("http://127.0.0.1:59999"));
        assert!(!status.ready);
        assert_eq!(status.status, "offline");
    }

    #[test]
    fn guarda_audio_en_biblioteca_con_metadatos_sound() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = b"RIFFfakeaudioWAVE";
        let entry = save_audio_to_library(temp.path(), "laser_blast", bytes, "disparo laser", 2.5, "local").unwrap();

        assert_eq!(entry.kind, "sound");
        assert_eq!(entry.name, "laser_blast.wav");
        assert!(temp.path().join("laser_blast.wav").exists());
        assert_eq!(entry.parameters["duration"], 2.5);
    }
}
