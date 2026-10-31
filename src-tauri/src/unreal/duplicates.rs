// =============================================================================
// DBV Unreal AI — Accesos al editor que se saltan la app
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-52
//
// La app lanza los agentes en la carpeta del proyecto. Si esa carpeta tiene su propia configuración MCP apuntando
// al editor (la escribe el motor con `GenerateClientConfig` y la escribe el asistente «Set Up AI Coding» de Convai,
// visto en vivo el 2026-10-31), el agente ve Unreal dos veces: herramientas duplicadas, más tokens y una vía que
// se salta la comprobación del Python (REQ-26) y las aprobaciones de la app (REQ-12). Aquí solo se DETECTA: la app
// no toca esos ficheros (REQ-34).

use std::path::Path;

use serde::Serialize;

/// Ficheros de configuración MCP de los agentes, relativos a la carpeta del proyecto.
const MCP_CONFIGS: [&str; 4] = [".mcp.json", ".cursor/mcp.json", ".gemini/settings.json", ".codex/config.toml"];
/// Ficheros de instrucciones donde Convai mete su bloque.
const PRIMERS: [&str; 5] = ["CLAUDE.md", "AGENTS.md", "GEMINI.md", ".github/copilot-instructions.md", ".cursor/rules/convai.mdc"];
/// Marca del bloque de instrucciones de Convai.
const PRIMER_MARK: &str = "Convai primer";
/// No se leen ficheros más grandes que esto.
const MAX_BYTES: u64 = 1024 * 1024;

/// Un acceso directo encontrado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectAccess {
    /// Ruta relativa al proyecto, con `/`.
    pub file: String,
    /// `mcp`: un servidor MCP que apunta al editor; `primer`: instrucciones de Convai para el agente.
    pub kind: String,
}

fn read_small(path: &Path) -> Option<String> {
    std::fs::metadata(path).ok().filter(|m| m.is_file() && m.len() <= MAX_BYTES).and_then(|_| std::fs::read_to_string(path).ok())
}

/// ¿Apunta este texto al servidor MCP del editor en `port`?
fn points_to_editor(text: &str, port: u16) -> bool {
    let lower = text.to_lowercase();
    ["127.0.0.1", "localhost"].iter().any(|host| lower.contains(&format!("{host}:{port}/mcp")))
}

/// Accesos directos al editor en la carpeta del proyecto. @spec REQ-52
pub fn scan(project_dir: &Path, port: u16) -> Vec<DirectAccess> {
    let mcp = MCP_CONFIGS.iter().filter(|f| read_small(&project_dir.join(f)).is_some_and(|t| points_to_editor(&t, port))).map(|f| DirectAccess { file: (*f).to_string(), kind: "mcp".into() });
    let primers = PRIMERS.iter().filter(|f| read_small(&project_dir.join(f)).is_some_and(|t| t.contains(PRIMER_MARK))).map(|f| DirectAccess { file: (*f).to_string(), kind: "primer".into() });
    mcp.chain(primers).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lo que escribió Convai en `DBVUnrealAITest1` el 2026-10-31. @spec REQ-52
    #[test]
    fn detecta_lo_que_escribe_convai_y_no_toca_nada() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan(dir.path(), 8000).is_empty());
        let mcp = "{\n\t\"mcpServers\":\n\t{\n\t\t\"unreal-mcp\":\n\t\t{\n\t\t\t\"type\": \"http\",\n\t\t\t\"url\": \"http://127.0.0.1:8000/mcp\"\n\t\t}\n\t}\n}";
        std::fs::write(dir.path().join(".mcp.json"), mcp).unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "<!-- >>> Convai primer (managed by Convai; edits between sentinels are overwritten) -->\n# Working with Convai\n<!-- <<< Convai primer -->\n").unwrap();
        std::fs::create_dir_all(dir.path().join(".codex")).unwrap();
        std::fs::write(dir.path().join(".codex/config.toml"), "[mcp_servers.unreal]\nurl = \"http://localhost:8001/mcp\"\n").unwrap();
        let found = scan(dir.path(), 8000);
        assert_eq!(found, vec![DirectAccess { file: ".mcp.json".into(), kind: "mcp".into() }, DirectAccess { file: "CLAUDE.md".into(), kind: "primer".into() }]);
        assert_eq!(scan(dir.path(), 8001)[0].file, ".codex/config.toml", "otro puerto, otro editor");
        assert_eq!(std::fs::read_to_string(dir.path().join(".mcp.json")).unwrap(), mcp, "la app no modifica el fichero");
    }
}
