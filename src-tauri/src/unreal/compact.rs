// =============================================================================
// DBV Unreal AI — Lo que se devuelve a la IA, acotado
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-51
//
// Un agente reenvía al modelo todo lo acumulado en cada llamada, así que el tamaño de cada resultado se paga muchas
// veces. Medido en vivo el 2026-10-31: `describe_toolset` de `NiagaraToolsets.NiagaraToolset_System` son 306 KB
// (~75.000 tokens) y el de `EditorToolset.EditorAppToolset` 23 KB. Aquí viven el tope por resultado y el formato
// compacto de un toolset. El asistente integrado (`src/unreal/unrealTools.js`) produce EXACTAMENTE el mismo texto: lo
// comprueban los dos contra los ficheros de referencia de `tests/fixtures/compact/` (D22).

use serde_json::Value;

use super::mcp_client::{ToolInfo, ToolsetInfo};

/// Caracteres como mucho en un resultado de herramienta. El mismo valor que `RESULT_LIMIT` de `unrealTools.js`.
pub const RESULT_LIMIT: usize = 12_000;

/// Caracteres como mucho de la descripción de una herramienta en la vista compacta.
const SUMMARY_LIMIT: usize = 160;

/// Recorta `text` a `RESULT_LIMIT` caracteres y, si recorta, lo dice y explica cómo pedir menos.
pub fn clip(text: &str) -> String {
    let total = text.chars().count();
    let result = if total > RESULT_LIMIT {
        let head: String = text.chars().take(RESULT_LIMIT).collect();
        format!("{head}\n… (truncated: {total} characters; ask for less — a filter, a limit or one specific tool)")
    } else {
        text.to_string()
    };
    result
}

/// Tipo de un parámetro en una palabra: `string`, `number`, `enum`, `array<string>`, `object`…
fn type_word(schema: &Value) -> String {
    let base = schema.get("type").and_then(Value::as_str).unwrap_or("any");
    let word = if schema.get("enum").is_some() {
        "enum".to_string()
    } else if base == "array" {
        format!("array<{}>", schema.get("items").map(type_word).unwrap_or_else(|| "any".into()))
    } else {
        base.to_string()
    };
    word
}

/// Parámetros de una herramienta en una línea: `name*: string, other: number` (`*` = obligatorio).
pub fn params_line(schema: &Value) -> String {
    let required: Vec<&str> = schema.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
    let props = schema.get("properties").and_then(Value::as_object);
    props
        .into_iter()
        .flatten()
        .map(|(name, prop)| format!("{name}{}: {}", if required.contains(&name.as_str()) { "*" } else { "" }, type_word(prop)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Primera frase de una descripción: del primer párrafo (las de Epic parten las frases en varias líneas), con los
/// espacios normalizados, hasta el primer punto seguido de espacio.
pub fn first_sentence(text: &str) -> String {
    let paragraph = text.trim().replace("\r\n", "\n");
    let first = paragraph.split("\n\n").next().unwrap_or("");
    let line = first.split_whitespace().collect::<Vec<_>>().join(" ");
    let sentence = line.find(". ").map_or(line.as_str(), |i| &line[..=i]);
    let clipped: String = sentence.chars().take(SUMMARY_LIMIT).collect();
    if sentence.chars().count() > SUMMARY_LIMIT {
        format!("{clipped}…")
    } else {
        clipped
    }
}

/// Un toolset en formato compacto: una línea por herramienta con sus parámetros y su primera frase.
pub fn compact_toolset(info: &ToolsetInfo) -> String {
    let mut lines = vec![
        format!("toolset: {}", info.name),
        first_sentence(&info.description),
        format!("{} tools. For the full input schema of one tool, call unreal_describe_toolset again with tool=\"<name>\".", info.tools.len()),
    ];
    lines.extend(info.tools.iter().map(|t| format!("- {}({}) — {}", t.name, params_line(&t.input_schema), first_sentence(&t.description))));
    clip(&lines.join("\n"))
}

/// El esquema completo de UNA herramienta del toolset (la busca sin distinguir mayúsculas).
pub fn tool_detail(info: &ToolsetInfo, tool: &str) -> Result<String, String> {
    let wanted = tool.trim().to_lowercase();
    let found: Option<&ToolInfo> = info.tools.iter().find(|t| t.name.to_lowercase() == wanted);
    let detail = found.map(|t| clip(&format!("toolset: {}\n## {}\n{}\ninput: {}", info.name, t.name, t.description.trim(), t.input_schema)));
    detail.ok_or_else(|| format!("{} has no tool named {tool}; its tools are: {}", info.name, info.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unreal::mcp_client::parse_toolset_info;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures");

    /// Texto que devolvió el editor real a `describe_toolset` (respuesta JSON-RPC grabada el 2026-10-31).
    fn recorded(toolset: &str) -> ToolsetInfo {
        let path = format!("{FIXTURES}/unreal-mcp/ue-5.8.4/describe_toolset.{toolset}.response.json");
        let response: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        parse_toolset_info(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    /// El formato compacto es el de los ficheros de referencia, los mismos que comprueba el test de JS. @spec REQ-51
    #[test]
    fn el_formato_compacto_coincide_con_la_referencia_comun() {
        for toolset in ["ConvaiToolset.ConvaiSetupToolset", "ConvaiToolset.ConvaiActionToolset", "EditorToolset.EditorAppToolset", "NiagaraToolsets.NiagaraToolset_System"] {
            let golden = format!("{FIXTURES}/compact/{toolset}.txt");
            // Regenerar la referencia a propósito: DBV_UPDATE_GOLDEN=1 cargo test compact (y revisar el diff).
            if std::env::var_os("DBV_UPDATE_GOLDEN").is_some() {
                std::fs::write(&golden, format!("{}\n", compact_toolset(&recorded(toolset)))).unwrap();
            }
            let expected = std::fs::read_to_string(&golden).unwrap().replace("\r\n", "\n");
            assert_eq!(compact_toolset(&recorded(toolset)), expected.trim_end_matches('\n'), "{toolset}");
        }
    }

    /// Niagara pasaba de 306 KB; ahora cabe en el tope y la herramienta concreta sigue disponible entera. @spec REQ-51
    #[test]
    fn un_toolset_enorme_cabe_en_el_tope_y_su_herramienta_se_pide_aparte() {
        let niagara = recorded("NiagaraToolsets.NiagaraToolset_System");
        let compact = compact_toolset(&niagara);
        assert!(compact.chars().count() <= RESULT_LIMIT + 120, "{}", compact.len());
        let first = &niagara.tools[0].name;
        assert!(tool_detail(&niagara, &first.to_lowercase()).unwrap().contains("input: {"));
        assert!(tool_detail(&niagara, "NoSuchTool").unwrap_err().contains("its tools are"));
    }

    /// Las dos superficies usan el mismo tope y el mismo máximo de entradas de memoria (D22, REQ-50). @spec REQ-50, REQ-51
    #[test]
    fn las_constantes_coinciden_con_las_de_js() {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../src");
        let tools = std::fs::read_to_string(format!("{src}/unreal/unrealTools.js")).unwrap();
        assert!(tools.contains(&format!("export const RESULT_LIMIT = {RESULT_LIMIT};")), "RESULT_LIMIT de unrealTools.js");
        assert!(tools.contains(&format!("const SUMMARY_LIMIT = {SUMMARY_LIMIT};")), "SUMMARY_LIMIT de unrealTools.js");
        let memory = std::fs::read_to_string(format!("{src}/memory/memory.js")).unwrap();
        assert!(memory.contains(&format!("export const MAX_ENTRIES = {};", crate::project_memory::tools::MAX_ENTRIES)), "MAX_ENTRIES de memory.js");
    }

    /// @spec REQ-51
    #[test]
    fn recorta_y_lo_dice() {
        assert_eq!(clip("hola"), "hola");
        let long = "é".repeat(RESULT_LIMIT + 5);
        let clipped = clip(&long);
        assert!(clipped.starts_with(&"é".repeat(RESULT_LIMIT)) && clipped.contains(&format!("truncated: {} characters", RESULT_LIMIT + 5)));
        assert_eq!(first_sentence("Spawns an actor. Then more.\nSecond line"), "Spawns an actor.");
        assert_eq!(first_sentence("Wires a\nMetaHuman up.\n\nAll paths are object paths."), "Wires a MetaHuman up.");
        assert_eq!(params_line(&serde_json::json!({ "type": "object", "properties": { "a": { "type": "string" }, "b": { "type": "array", "items": { "type": "number" } }, "c": { "type": "string", "enum": ["x"] } }, "required": ["a"] })), "a*: string, b: array<number>, c: enum");
    }
}
