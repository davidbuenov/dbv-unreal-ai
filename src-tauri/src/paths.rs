// =============================================================================
// DBV Unreal AI — Carpetas de la app sin AppHandle (modo --mcp)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// El servidor MCP propio (`--mcp`) no arranca Tauri, pero tiene que leer la MISMA biblioteca y
// los mismos ajustes que la ventana. Tauri usa `<dataDir>/<identifier>` y `<configDir>/<identifier>`;
// aquí se calculan igual a partir de las variables del sistema.

use std::path::PathBuf;

/// Identificador del bundle (`tauri.conf.json`).
pub const IDENTIFIER: &str = "com.davidbuenov.dbv-unreal-ai";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Carpeta de datos (biblioteca, copias de seguridad, conversaciones).
pub fn data_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".local").join("share")))
    };
    base.map(|b| b.join(IDENTIFIER))
}

/// Carpeta de configuración (conexiones, ajustes de generación).
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".config")))
    };
    base.map(|b| b.join(IDENTIFIER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_carpetas_acaban_en_el_identificador() {
        if let Some(dir) = data_dir() {
            assert!(dir.ends_with(IDENTIFIER));
        }
        if let Some(dir) = config_dir() {
            assert!(dir.ends_with(IDENTIFIER));
        }
    }
}
