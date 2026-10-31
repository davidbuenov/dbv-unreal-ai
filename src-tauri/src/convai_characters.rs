// =============================================================================
// DBV Unreal AI — Personajes de Convai registrados en la app
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-58
//
// Como Tripo y World Labs, Convai tiene su tarjeta en Ajustes, pero sin clave: el plugin se autentica solo
// (REJECTED-07). Lo que la persona registra es la lista de personajes del Playground con los que quiere trabajar
// (nombre, Character ID, nota) para que la IA use el ID correcto por su nombre y nunca se lo invente. Se guarda en
// `<config>/convai-characters.json` (D32): no es secreto, vale para cualquier proyecto y el `--mcp` lo lee igual.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Fichero de los personajes dentro de la carpeta de configuración de la app.
pub const FILE: &str = "convai-characters.json";

/// Un personaje de Convai registrado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvaiCharacter {
    pub name: String,
    pub character_id: String,
    #[serde(default)]
    pub notes: String,
}

fn path(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE)
}

/// Personajes guardados (vacío si no hay fichero o no se entiende).
pub fn load(config_dir: &Path) -> Vec<ConvaiCharacter> {
    std::fs::read_to_string(path(config_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// ¿Tiene forma de Character ID de Convai? Hexadecimal y guiones, de 8 a 64 caracteres.
pub fn valid_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && id.chars().any(|c| c.is_ascii_hexdigit())
}

/// Limpia y comprueba una lista: nombres y IDs obligatorios, válidos y sin repetir. @spec REQ-58
pub fn validate(list: &[ConvaiCharacter]) -> Result<Vec<ConvaiCharacter>, String> {
    let mut clean: Vec<ConvaiCharacter> = Vec::new();
    let mut problems = Vec::new();
    for (i, c) in list.iter().enumerate() {
        let item = ConvaiCharacter { name: c.name.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(60).collect(), character_id: c.character_id.trim().to_lowercase(), notes: c.notes.trim().chars().take(200).collect() };
        if item.name.is_empty() {
            problems.push(format!("personaje {}: falta el nombre", i + 1));
        } else if !valid_id(&item.character_id) {
            problems.push(format!("{}: el Character ID no es válido (cópialo del Playground de Convai)", item.name));
        } else if clean.iter().any(|x| x.name.eq_ignore_ascii_case(&item.name)) {
            problems.push(format!("{}: el nombre está repetido", item.name));
        } else if clean.iter().any(|x| x.character_id == item.character_id) {
            problems.push(format!("{}: ese Character ID ya está registrado", item.name));
        } else {
            clean.push(item);
        }
    }
    if problems.is_empty() {
        Ok(clean)
    } else {
        Err(problems.join("; "))
    }
}

/// Guarda la lista validada, con escritura atómica. @spec REQ-58
pub fn save(config_dir: &Path, list: &[ConvaiCharacter]) -> Result<Vec<ConvaiCharacter>, String> {
    let clean = validate(list)?;
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&clean).map_err(|e| e.to_string())?;
    let tmp = config_dir.join(format!("{FILE}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path(config_dir)).map_err(|e| e.to_string())?;
    Ok(clean)
}

/// La herramienta de lectura, igual para el asistente integrado y el MCP propio (D10).
pub fn spec() -> Value {
    json!({
        "name": "convai_characters",
        "description": "List the Convai characters the user registered in DBV Unreal AI (Settings → Convai): name, Character ID and notes. Use the Character ID of the one the user names with SetupConvaiCharacter. If the character is not in the list, ask the user to add it in Settings → Convai or to give you its Character ID from the Convai Playground; never invent an ID. Read-only.",
        "inputSchema": { "type": "object", "properties": {}, "required": [] }
    })
}

/// Texto de la herramienta para la IA.
pub fn text(list: &[ConvaiCharacter]) -> String {
    let lines: Vec<String> = list.iter().map(|c| format!("- {}: {}{}", c.name, c.character_id, if c.notes.is_empty() { String::new() } else { format!(" ({})", c.notes) })).collect();
    if lines.is_empty() {
        "No Convai characters are registered yet. Ask the user to add one in Settings → Convai (name and Character ID from the Convai Playground) or to give you the Character ID.".to_string()
    } else {
        format!("Convai characters registered in the app:\n{}", lines.join("\n"))
    }
}

#[cfg(desktop)]
fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Los personajes registrados, para la tarjeta de Ajustes. @spec REQ-58
#[cfg(desktop)]
#[tauri::command]
pub fn convai_characters_list(app: tauri::AppHandle) -> Result<Vec<ConvaiCharacter>, String> {
    Ok(load(&config_dir(&app)?))
}

/// Guarda la lista entera que dejó la persona en Ajustes. @spec REQ-58
#[cfg(desktop)]
#[tauri::command]
pub fn convai_characters_save(app: tauri::AppHandle, characters: Vec<ConvaiCharacter>) -> Result<Vec<ConvaiCharacter>, String> {
    save(&config_dir(&app)?, &characters)
}

/// El resultado de `convai_characters` para el asistente integrado: el mismo texto que recibe un agente. @spec REQ-58
#[cfg(desktop)]
#[tauri::command]
pub fn convai_characters_text(app: tauri::AppHandle) -> Result<String, String> {
    Ok(text(&load(&config_dir(&app)?)))
}

/// La herramienta tal cual la ve un agente, para el asistente integrado (misma definición).
#[cfg(desktop)]
#[tauri::command]
pub fn convai_characters_spec() -> Value {
    spec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn victoria() -> ConvaiCharacter {
        ConvaiCharacter { name: "  Victoria ".into(), character_id: "C1FCA5CA-03b4-11ef-ae52-42010a7be00e".into(), notes: "habla español".into() }
    }

    /// Se guarda limpio, se relee igual y lo ve la IA. @spec REQ-58
    #[test]
    fn guarda_y_relee_los_personajes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).is_empty());
        assert!(text(&[]).contains("No Convai characters"));
        let saved = save(dir.path(), &[victoria()]).unwrap();
        assert_eq!(saved[0].name, "Victoria");
        assert_eq!(saved[0].character_id, "c1fca5ca-03b4-11ef-ae52-42010a7be00e");
        assert_eq!(load(dir.path()), saved);
        assert!(text(&saved).contains("- Victoria: c1fca5ca-03b4-11ef-ae52-42010a7be00e (habla español)"));
    }

    /// IDs inválidos, nombres vacíos y repetidos se rechazan sin escribir nada. @spec REQ-58
    #[test]
    fn rechaza_ids_invalidos_y_repetidos() {
        let dir = tempfile::tempdir().unwrap();
        let bad = ConvaiCharacter { name: "Otro".into(), character_id: "no-es-un-id!".into(), notes: String::new() };
        assert!(save(dir.path(), &[bad]).unwrap_err().contains("no es válido"));
        assert!(validate(&[victoria(), ConvaiCharacter { name: "victoria".into(), ..ConvaiCharacter { character_id: "abcdef12".into(), name: String::new(), notes: String::new() } }]).unwrap_err().contains("repetido"));
        assert!(validate(&[victoria(), ConvaiCharacter { name: "Ana".into(), ..victoria() }]).unwrap_err().contains("ya está registrado"));
        assert!(validate(&[ConvaiCharacter { name: " ".into(), ..victoria() }]).unwrap_err().contains("falta el nombre"));
        assert!(!dir.path().join(FILE).exists());
        assert!(valid_id("c1fca5ca-03b4-11ef-ae52-42010a7be00e") && !valid_id("--------") && !valid_id("abc"));
    }
}
