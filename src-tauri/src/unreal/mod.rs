// =============================================================================
// DBV Unreal AI — Conexión con Unreal Engine: módulo raíz
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Todo lo que sabe la app de Unreal sin pasar por la IA:
//   · `discovery`: instalaciones del motor, proyectos `.uproject`, editores abiertos.
//   · `ini`: lectura y escritura mínima de los `.ini` de Unreal.
//   · `mcp_client`: cliente del servidor MCP del editor (el ÚNICO sitio que conoce
//     su protocolo, para que un cambio en la 5.9 se arregle aquí — RNF-02).
//   · `prepare`: diagnóstico y preparación de un proyecto (REQ-04, REQ-05).
//   · `commands`: frontera con el frontend.

pub mod capture;
pub mod commands;
pub mod discovery;
pub mod ini;
pub mod mcp_client;
pub mod prepare;

use serde::Serialize;

/// Errores de la conexión con Unreal, con un `kind` que la interfaz sabe explicar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum UnrealError {
    /// Fichero ilegible o con un formato inesperado (`.uproject`, `.ini`, lista del launcher).
    Format(String),
    /// Error de disco al leer o escribir.
    Io(String),
    /// El servidor MCP del editor no responde (apagado, puerto equivocado, editor cerrado).
    Unreachable(String),
    /// El editor respondió, pero con un error de protocolo o de la herramienta.
    Protocol(String),
    /// La llamada superó su tiempo límite (el editor puede estar ocupado: RNF-03).
    Timeout(String),
    /// Lo pedido no existe (proyecto, toolset, herramienta, copia de seguridad).
    NotFound(String),
    /// La sesión MCP ya no existe en el editor (se reinició): se reabre sola una vez.
    Expired(String),
    /// Una ruta que se sale de la carpeta del proyecto.
    Outside(String),
    /// El fichero cambió desde que se leyó (a mano, por `git pull` o desde otro proceso): no se pisa.
    Conflict(String),
}

impl std::fmt::Display for UnrealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Format(m)
            | Self::Io(m)
            | Self::Unreachable(m)
            | Self::Protocol(m)
            | Self::Timeout(m)
            | Self::NotFound(m)
            | Self::Expired(m)
            | Self::Outside(m)
            | Self::Conflict(m) => m,
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for UnrealError {}

/// Convierte un error de disco en `UnrealError::Io` con la ruta afectada.
pub fn io_error(path: &std::path::Path, error: &std::io::Error) -> UnrealError {
    UnrealError::Io(format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_error_viaja_con_su_tipo() {
        let raw = serde_json::to_string(&UnrealError::Unreachable("puerto 8000".into())).unwrap();
        assert_eq!(raw, r#"{"kind":"unreachable","message":"puerto 8000"}"#);
    }
}
