// =============================================================================
// DBV Unreal AI — Adjuntos de la conversación (imágenes y documentos de referencia)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-32, REQ-68
//
// A diferencia de `files.rs` (lo que la IA lee del proyecto, confinado a su carpeta), un adjunto lo ELIGE EL
// USUARIO en el diálogo del sistema o lo pega desde el portapapeles (`Ctrl+V`), puede estar en cualquier sitio y ninguna herramienta de la IA llega aquí.
// Imágenes (PNG, JPG, WebP, ≤ 10 MB) vuelven en base64 para los modelos con visión; documentos de texto
// (≤ 256 KB) vuelven como texto. La ruta va siempre, para que la IA pueda usar la imagen con Tripo.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::Serialize;

use super::AiError;

/// Tope de una imagen adjunta (los proveedores rechazan las mayores).
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
/// Tope de un documento adjunto: más no cabría en el contexto de un modelo local.
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;
/// Cuántas imágenes pegadas desde el portapapeles se conservan como máximo en la carpeta temporal.
const MAX_CLIPBOARD_FILES: usize = 20;

static CLIPBOARD_SEQ: AtomicU64 = AtomicU64::new(0);

/// Un fichero adjuntado por el usuario.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Attachment {
    #[serde(rename_all = "camelCase")]
    Image {
        path: String,
        name: String,
        mime: String,
        base64: String,
        bytes: u64,
    },
    #[serde(rename_all = "camelCase")]
    Text {
        path: String,
        name: String,
        content: String,
        truncated: bool,
    },
}

fn mime_of(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Lee un adjunto: imagen (base64) o texto. Un binario que no es imagen se rechaza.
pub fn read_attachment(path: &Path) -> Result<Attachment, AiError> {
    let meta =
        std::fs::metadata(path).map_err(|e| AiError::Config(format!("{}: {e}", path.display())))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let shown = path.to_string_lossy().to_string();
    let attachment = if let Some(mime) = mime_of(&ext) {
        if meta.len() > MAX_IMAGE_BYTES {
            return Err(AiError::Config(format!("{name} pasa de 10 MB")));
        }
        let bytes = std::fs::read(path).map_err(|e| AiError::Config(e.to_string()))?;
        Attachment::Image {
            path: shown,
            name,
            mime: mime.into(),
            base64: BASE64.encode(&bytes),
            bytes: meta.len(),
        }
    } else {
        let mut bytes = std::fs::read(path).map_err(|e| AiError::Config(e.to_string()))?;
        let truncated = bytes.len() as u64 > MAX_TEXT_BYTES;
        bytes.truncate(MAX_TEXT_BYTES as usize);
        // Un cero en el primer bloque es un binario (un .uasset, un .fbx): no es texto que un modelo pueda leer.
        if bytes.iter().take(8192).any(|b| *b == 0) {
            return Err(AiError::Config(format!(
                "{name} no es una imagen ni un documento de texto"
            )));
        }
        Attachment::Text {
            path: shown,
            name,
            content: String::from_utf8_lossy(&bytes).into_owned(),
            truncated,
        }
    };
    Ok(attachment)
}

fn ext_of_mime(mime: &str) -> Option<&'static str> {
    match mime.trim().to_ascii_lowercase().as_str() {
        "image/png" => Some("png"),
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

/// Nombre de fichero seguro (sin extensión, solo `[A-Za-z0-9_-]`, ≤ 32) a partir del que trae el portapapeles.
fn sanitize_stem(name: Option<&str>) -> String {
    let raw = name.unwrap_or("portapapeles").trim();
    let without_ext = raw.rsplit_once('.').map_or(raw, |(stem, _)| stem);
    let clean: String = without_ext
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    if clean.is_empty() {
        "portapapeles".into()
    } else {
        clean
    }
}

/// Deja en `dir` solo las `MAX_CLIPBOARD_FILES` imágenes pegadas más recientes.
fn prune_clipboard_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, std::path::PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok().filter(std::fs::Metadata::is_file)?;
            Some((meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH), e.path()))
        })
        .collect();
    files.sort_by_key(|(modified, _)| *modified);
    let remove_count = files.len().saturating_sub(MAX_CLIPBOARD_FILES);
    for (_, old) in files.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(old);
    }
}

/// Guarda una imagen pegada del portapapeles (`Ctrl+V`) o arrastrada sin ruta en `dir` y la devuelve como `Attachment::Image`. @spec REQ-68
pub fn save_clipboard_image(
    dir: &Path,
    mime: &str,
    base64_data: &str,
    name: Option<&str>,
) -> Result<Attachment, AiError> {
    let ext = ext_of_mime(mime)
        .ok_or_else(|| AiError::Config(format!("formato de imagen no soportado: {mime}")))?;
    let clean_b64 = base64_data
        .trim()
        .split_once(";base64,")
        .map(|(_, b)| b)
        .unwrap_or_else(|| base64_data.trim());
    let bytes = BASE64
        .decode(clean_b64)
        .map_err(|e| AiError::Config(format!("imagen en base64 no válida: {e}")))?;
    if bytes.is_empty() {
        return Err(AiError::Config(
            "la imagen del portapapeles está vacía".into(),
        ));
    }
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(AiError::Config(
            "la imagen del portapapeles pasa de 10 MB".into(),
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| AiError::Config(format!("{}: {e}", dir.display())))?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = CLIPBOARD_SEQ.fetch_add(1, Ordering::Relaxed);
    let stem = sanitize_stem(name);
    let file_path = dir.join(format!("{stem}-{millis}-{seq}.{ext}"));
    std::fs::write(&file_path, &bytes)
        .map_err(|e| AiError::Config(format!("{}: {e}", file_path.display())))?;
    prune_clipboard_dir(dir);
    read_attachment(&file_path)
}

/// Lee el fichero que el usuario eligió en el diálogo. Las imágenes se pueden además ver en la interfaz. @spec REQ-32
#[tauri::command]
pub async fn ai_read_attachment(
    app: tauri::AppHandle,
    path: String,
) -> Result<Attachment, AiError> {
    let file = std::path::PathBuf::from(&path);
    let read = tauri::async_runtime::spawn_blocking(move || read_attachment(&file))
        .await
        .map_err(|e| AiError::Config(e.to_string()))??;
    if matches!(read, Attachment::Image { .. }) {
        use tauri::Manager;
        let _ = app.asset_protocol_scope().allow_file(&path);
    }
    Ok(read)
}

/// Guarda en disco una imagen pegada desde el portapapeles (`Ctrl+V`) y la devuelve con ruta real y base64. @spec REQ-68
#[tauri::command]
pub async fn ai_save_clipboard_image(
    app: tauri::AppHandle,
    mime: String,
    data: String,
    name: Option<String>,
) -> Result<Attachment, AiError> {
    let dir = std::env::temp_dir().join("dbv-unreal-ai-clipboard");
    let read = tauri::async_runtime::spawn_blocking(move || {
        save_clipboard_image(&dir, &mime, &data, name.as_deref())
    })
    .await
    .map_err(|e| AiError::Config(e.to_string()))??;
    if let Attachment::Image { ref path, .. } = read {
        use tauri::Manager;
        let _ = app.asset_protocol_scope().allow_file(path);
    }
    Ok(read)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec REQ-32
    #[test]
    fn lee_imagenes_y_textos_y_rechaza_binarios() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("barco.PNG");
        std::fs::write(&image, [0x89, b'P', b'N', b'G']).unwrap();
        match read_attachment(&image).unwrap() {
            Attachment::Image {
                mime, base64, name, ..
            } => assert_eq!(
                (mime.as_str(), base64.as_str(), name.as_str()),
                ("image/png", "iVBORw==", "barco.PNG")
            ),
            other => panic!("{other:?}"),
        }
        let notes = dir.path().join("brief.md");
        std::fs::write(&notes, "# Nivel\nUn puerto pirata al atardecer").unwrap();
        assert!(
            matches!(read_attachment(&notes).unwrap(), Attachment::Text { content, truncated: false, .. } if content.contains("puerto pirata"))
        );
        let binary = dir.path().join("modelo.uasset");
        std::fs::write(&binary, [1, 0, 2, 3]).unwrap();
        assert!(read_attachment(&binary).is_err());
        let big = dir.path().join("enorme.txt");
        std::fs::write(&big, "a".repeat(MAX_TEXT_BYTES as usize + 10)).unwrap();
        assert!(matches!(
            read_attachment(&big).unwrap(),
            Attachment::Text {
                truncated: true,
                ..
            }
        ));
    }

    /// @spec REQ-68
    #[test]
    fn guarda_imagen_del_portapapeles_con_ruta_en_disco_y_rechaza_formatos_invalidos() {
        let dir = tempfile::tempdir().unwrap();
        let saved = save_clipboard_image(
            dir.path(),
            "image/png",
            "data:image/png;base64,iVBORw==",
            Some("captura.png"),
        )
        .unwrap();
        match saved {
            Attachment::Image {
                path,
                name,
                mime,
                base64,
                bytes,
            } => {
                assert_eq!(mime, "image/png");
                assert_eq!(base64, "iVBORw==");
                assert_eq!(bytes, 4);
                assert!(name.starts_with("captura-"));
                assert!(name.ends_with(".png"));
                assert!(std::path::Path::new(&path).is_file());
            }
            other => panic!("{other:?}"),
        }
        assert!(save_clipboard_image(dir.path(), "application/pdf", "iVBORw==", None).is_err());
        assert!(save_clipboard_image(dir.path(), "image/png", "", None).is_err());
    }

    /// @spec REQ-68
    #[test]
    fn la_carpeta_del_portapapeles_no_pasa_de_su_tope() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..MAX_CLIPBOARD_FILES + 5 {
            std::fs::write(dir.path().join(format!("vieja-{i}.png")), [0x89]).unwrap();
        }
        save_clipboard_image(dir.path(), "image/png", "iVBORw==", Some("nueva.png")).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), MAX_CLIPBOARD_FILES);
        assert_eq!(sanitize_stem(Some("../../etc/pas swd.png")), "etcpasswd");
        assert_eq!(sanitize_stem(Some(".png")), "portapapeles");
    }
}
