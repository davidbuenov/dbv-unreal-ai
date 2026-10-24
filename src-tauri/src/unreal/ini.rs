// =============================================================================
// DBV Unreal AI — Lectura y escritura mínima de los .ini de Unreal
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// Unreal usa un `.ini` propio (`[Seccion]`, `Clave=Valor`, prefijos `+`/`-` para
// listas). Aquí solo hace falta leer un valor y cambiar uno SIN reescribir el resto
// del fichero: la preparación (REQ-05) enseña un diff, y un diff que reordena o
// reformatea todo el fichero no se puede revisar. Por eso `set` trabaja por líneas.

/// Valor de `key` en `[section]` (la última aparición gana, como en Unreal), o `None`.
pub fn get(text: &str, section: &str, key: &str) -> Option<String> {
    let mut current: Option<&str> = None;
    let mut found = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = section_name(trimmed) {
            current = Some(name);
        } else if current.is_some_and(|name| name.eq_ignore_ascii_case(section)) {
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    found = Some(v.trim().to_string());
                }
            }
        }
    }
    found
}

/// `text` con `key=value` en `[section]`: sustituye la línea existente, la añade al
/// final de la sección o crea la sección al final del fichero. Conserva el resto
/// de líneas y el salto de línea del fichero (`\r\n` o `\n`).
pub fn set(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let entry = format!("{key}={value}");
    let mut section_start: Option<usize> = None;
    let mut section_end = lines.len();
    let mut existing: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = section_name(trimmed) {
            if section_start.is_some() && section_end == lines.len() {
                section_end = index;
            }
            if name.eq_ignore_ascii_case(section) {
                section_start = Some(index);
                section_end = lines.len();
            }
        } else if section_start.is_some() && section_end == lines.len() {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    existing = Some(index);
                }
            }
        }
    }
    match (section_start, existing) {
        (_, Some(index)) => lines[index] = entry,
        (Some(_), None) => {
            // Detrás de la última línea no vacía de la sección.
            let mut insert_at = section_end;
            while insert_at > 0 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            lines.insert(insert_at, entry);
        }
        (None, None) => {
            if lines.last().is_some_and(|last| !last.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("[{section}]"));
            lines.push(entry);
        }
    }
    let mut result = lines.join(newline);
    result.push_str(newline);
    result
}

fn section_name(trimmed: &str) -> Option<&str> {
    trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[/Script/A.B]\nX=1\n\n[/Script/ModelContextProtocolEngine.ModelContextProtocolSettings]\nServerPortNumber=8001\n\n[Other]\nY=2\n";

    #[test]
    fn lee_un_valor_de_su_seccion() {
        assert_eq!(get(SAMPLE, "/Script/ModelContextProtocolEngine.ModelContextProtocolSettings", "ServerPortNumber").as_deref(), Some("8001"));
        assert_eq!(get(SAMPLE, "Other", "X"), None);
    }

    #[test]
    fn cambia_un_valor_existente_sin_tocar_el_resto() {
        let out = set(SAMPLE, "/Script/ModelContextProtocolEngine.ModelContextProtocolSettings", "ServerPortNumber", "8002");
        assert_eq!(out, SAMPLE.replace("ServerPortNumber=8001", "ServerPortNumber=8002"));
    }

    #[test]
    fn anade_la_clave_al_final_de_su_seccion() {
        let out = set(SAMPLE, "/Script/ModelContextProtocolEngine.ModelContextProtocolSettings", "bAutoStartServer", "True");
        assert!(out.contains("ServerPortNumber=8001\nbAutoStartServer=True\n\n[Other]"), "{out}");
    }

    #[test]
    fn crea_la_seccion_si_no_existe_y_conserva_crlf() {
        let out = set("[A]\r\nX=1\r\n", "B", "Y", "2");
        assert_eq!(out, "[A]\r\nX=1\r\n\r\n[B]\r\nY=2\r\n");
        assert_eq!(set("", "B", "Y", "2"), "[B]\nY=2\n");
    }
}
