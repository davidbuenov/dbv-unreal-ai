// =============================================================================
// DBV Unreal AI — Detección de IAs disponibles
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
//
// Al abrir «Conectar una IA» (nunca al arrancar la aplicación) se mira qué hay
// en el equipo: servidores locales escuchando (Ollama, LM Studio) y agentes
// instalados en el PATH (Claude Code, Gemini CLI, Codex, Copilot) junto con
// Node.js, que necesitan sus adaptadores ACP.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Un modelo disponible en un agente CLI, con su nombre legible y niveles de esfuerzo admitidos.
/// @spec REQ-47
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelOption {
    pub id: String,
    pub label: String,
    pub efforts: Vec<String>,
    pub default_effort: String,
}

/// Un servidor local de modelos.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalServer {
    pub provider: &'static str,
    pub running: bool,
    pub models: Vec<String>,
}

/// Un programa encontrado (o no) en el PATH.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: &'static str,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub servers: Vec<LocalServer>,
    pub tools: Vec<Tool>,
}

/// Programas que interesan: los agentes y lo que necesitan sus adaptadores.
pub const TOOLS: &[&str] = &["claude", "gemini", "antigravity", "agy", "agy_acp_server", "codex", "copilot", "node", "npx"];

/// Busca `name` en las carpetas de `path_var` (respeta `PATHEXT` en Windows).
pub fn find_in(name: &str, path_var: &str, extensions: &[String]) -> Option<PathBuf> {
    std::env::split_paths(path_var).find_map(|dir| {
        extensions.iter().map(|ext| dir.join(format!("{name}{ext}"))).find(|candidate| candidate.is_file())
    })
}

fn extensions() -> Vec<String> {
    if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        pathext.split(';').filter(|ext| !ext.is_empty()).map(|ext| ext.to_lowercase()).collect()
    } else {
        vec![String::new()]
    }
}

/// Rutas de búsqueda combinando PATH y carpetas estándar de usuario (Windows / Unix).
pub fn extended_path() -> String {
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&std::env::var("PATH").unwrap_or_default()).collect();

    if cfg!(windows) {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(&local);
            dirs.push(p.join("agy").join("bin"));
            dirs.push(p.join("Programs").join("Antigravity").join("bin"));
            dirs.push(p.join("Programs").join("Antigravity IDE").join("bin"));
            dirs.push(p.join("Programs").join("OpenAI").join("Codex").join("bin"));
            dirs.push(p.join("Microsoft").join("WinGet").join("Links"));
        }
        if let Ok(user) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(&user);
            dirs.push(p.join(".antigravity").join("bin"));
            dirs.push(p.join(".local").join("bin"));
            dirs.push(p.join("AppData").join("Roaming").join("npm"));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(&appdata);
            dirs.push(p.join("npm"));
        }
    } else {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(&home);
            dirs.push(p.join(".local").join("bin"));
            dirs.push(p.join(".antigravity").join("bin"));
            dirs.push(p.join(".npm-global").join("bin"));
        }
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
    }

    dirs.retain(|d| d.is_dir());
    std::env::join_paths(dirs).unwrap_or_default().to_string_lossy().to_string()
}

/// Ruta de un programa del PATH (o carpetas conocidas), o `None`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path_var = extended_path();
    let exts = extensions();
    let mut result = find_in(name, &path_var, &exts);
    if result.is_none() && name == "antigravity" {
        result = find_in("agy", &path_var, &exts);
    }
    result
}

fn get_json(url: &str) -> Option<Value> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(1500)))
        .build()
        .new_agent();
    agent.get(url).call().ok()?.into_body().read_json::<Value>().ok()
}

/// Nombres de modelos de una respuesta de Ollama (`/api/tags`) o compatible con OpenAI (`/v1/models`).
pub fn model_names(value: &Value) -> Vec<String> {
    let list = value["models"].as_array().or_else(|| value["data"].as_array());
    list.into_iter()
        .flatten()
        .filter_map(|model| model["name"].as_str().or_else(|| model["id"].as_str()).map(str::to_string))
        .collect()
}

pub fn detect() -> Detection {
    let ollama = get_json("http://127.0.0.1:11434/api/tags");
    let lmstudio = get_json("http://127.0.0.1:1234/v1/models");
    Detection {
        servers: vec![
            LocalServer { provider: "ollama", running: ollama.is_some(), models: ollama.as_ref().map(model_names).unwrap_or_default() },
            LocalServer { provider: "lmStudio", running: lmstudio.is_some(), models: lmstudio.as_ref().map(model_names).unwrap_or_default() },
        ],
        tools: TOOLS.iter().map(|name| Tool { name, path: which(name).map(|path| path.to_string_lossy().to_string()) }).collect(),
    }
}

/// Ejecutable de verdad para lanzar `name` en Windows: un `.cmd`/`.bat` (como
/// `npx.cmd`) no se puede lanzar directamente con `Command::new`.
pub fn launcher(path: &Path) -> (PathBuf, Vec<String>) {
    let is_script = path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
    if cfg!(windows) && is_script {
        (PathBuf::from("cmd.exe"), vec!["/d".into(), "/s".into(), "/c".into(), path.to_string_lossy().to_string()])
    } else {
        (path.to_path_buf(), Vec::new())
    }
}

/// Caché en memoria de la sesión para los modelos consultados a cada agente CLI.
static AGENT_MODELS_CACHE: OnceLock<Mutex<HashMap<String, Vec<AgentModelOption>>>> = OnceLock::new();

fn effort_rank(effort: &str) -> u8 {
    match effort {
        "off" => 0,
        "low" => 1,
        "medium" => 2,
        "high" => 3,
        "xhigh" => 4,
        "max" => 5,
        "ultra" => 6,
        _ => 7,
    }
}

/// Parsea la salida de `agy models` (`<id>\t<displayName>`) agrupando las variantes
/// por familia de modelo y sus niveles de dificultad (`low`, `medium`, `high`).
/// @spec REQ-47
pub fn parse_agy_models(stdout: &str) -> Vec<AgentModelOption> {
    let suffixes = ["-low", "-medium", "-high", "-xhigh", "-max", "-ultra"];
    let label_suffixes = [" (low)", " (medium)", " (high)", " (xhigh)", " (max)", " (ultra)"];
    let mut groups: Vec<AgentModelOption> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("Fetching")
            || trimmed.starts_with("Loading")
        {
            continue;
        }
        let Some((raw_id, raw_label)) = trimmed.split_once('\t').or_else(|| trimmed.split_once("  ")) else {
            continue;
        };
        let raw_id = raw_id.trim();
        let raw_label = raw_label.trim();
        if raw_id.is_empty() || raw_id.contains(' ') {
            continue;
        }
        let mut base_id = raw_id.to_string();
        let mut effort = "medium".to_string();
        for suf in suffixes {
            if let Some(stripped) = raw_id.strip_suffix(suf) {
                base_id = stripped.to_string();
                effort = suf.trim_start_matches('-').to_string();
                break;
            }
        }
        let mut base_label = raw_label.to_string();
        let lower_label = raw_label.to_lowercase();
        for lsuf in label_suffixes {
            if lower_label.ends_with(lsuf) {
                base_label = raw_label[..raw_label.len() - lsuf.len()].trim().to_string();
                break;
            }
        }
        if let Some(existing) = groups.iter_mut().find(|g| g.id == base_id) {
            if !existing.efforts.contains(&effort) {
                existing.efforts.push(effort);
            }
        } else {
            groups.push(AgentModelOption {
                id: base_id,
                label: base_label,
                efforts: vec![effort],
                default_effort: "medium".into(),
            });
        }
    }
    for group in &mut groups {
        group.efforts.sort_by_key(|e| effort_rank(e));
        group.default_effort = if group.efforts.iter().any(|e| e == "medium") {
            "medium".into()
        } else {
            group.efforts.first().cloned().unwrap_or_else(|| "medium".into())
        };
    }
    groups
}

/// Parsea la respuesta JSON de control `initialize` de Claude Code (`claude -p --input-format=stream-json`).
/// @spec REQ-47
pub fn parse_claude_models(stdout: &str) -> Vec<AgentModelOption> {
    let mut options: Vec<AgentModelOption> = Vec::new();
    let mut default_resolved: Option<String> = None;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        let Ok(val) = serde_json::from_str::<Value>(trimmed) else { continue };
        if val["type"].as_str() != Some("control_response") {
            continue;
        }
        let Some(models) = val["response"]["response"]["models"].as_array() else { continue };
        for m in models {
            if m["value"].as_str() == Some("default") {
                default_resolved = m["resolvedModel"].as_str().map(str::to_string);
                continue;
            }
            let Some(id) = m["resolvedModel"].as_str().or_else(|| m["value"].as_str()) else { continue };
            if id.is_empty() || options.iter().any(|o| o.id == id) {
                continue;
            }
            let raw_display = m["displayName"].as_str().unwrap_or(id).trim();
            let label = if raw_display.to_lowercase().starts_with("claude") {
                raw_display.to_string()
            } else {
                format!("Claude {raw_display}")
            };
            let mut efforts: Vec<String> = m["supportedEffortLevels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            if m["supportsThinkingOff"].as_bool() == Some(true) && !efforts.iter().any(|e| e == "off") {
                efforts.insert(0, "off".into());
            }
            if efforts.is_empty() {
                efforts.push("off".into());
            }
            efforts.sort_by_key(|e| effort_rank(e));
            let default_effort = if efforts.iter().any(|e| e == "medium") {
                "medium".into()
            } else {
                efforts.first().cloned().unwrap_or_else(|| "medium".into())
            };
            options.push(AgentModelOption {
                id: id.to_string(),
                label,
                efforts,
                default_effort,
            });
        }
    }
    if let Some(def_id) = default_resolved {
        if let Some(pos) = options.iter().position(|o| o.id == def_id) {
            let item = options.remove(pos);
            options.insert(0, item);
        }
    }
    options
}

/// Parsea el catálogo JSON de `codex debug models` o `~/.codex/models_cache.json`.
/// @spec REQ-47
pub fn parse_codex_models(json_text: &str) -> Vec<AgentModelOption> {
    let Ok(val) = serde_json::from_str::<Value>(json_text.trim()) else { return Vec::new() };
    let Some(models) = val["models"].as_array() else { return Vec::new() };
    let has_list_vis = models.iter().any(|m| m["visibility"].as_str() == Some("list"));
    let mut filtered: Vec<&Value> = models
        .iter()
        .filter(|m| !has_list_vis || m["visibility"].as_str() == Some("list"))
        .collect();
    filtered.sort_by_key(|m| m["priority"].as_i64().unwrap_or(999));
    let mut options = Vec::new();
    for m in filtered {
        let Some(slug) = m["slug"].as_str() else { continue };
        if slug.is_empty() || options.iter().any(|o: &AgentModelOption| o.id == slug) {
            continue;
        }
        let label = m["display_name"].as_str().unwrap_or(slug).trim().to_string();
        let mut efforts: Vec<String> = m["supported_reasoning_levels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|lvl| lvl["effort"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if efforts.is_empty() {
            efforts = vec!["low".into(), "medium".into(), "high".into()];
        }
        efforts.sort_by_key(|e| effort_rank(e));
        let default_effort = m["default_reasoning_level"]
            .as_str()
            .filter(|d| efforts.iter().any(|e| e == *d))
            .unwrap_or_else(|| if efforts.iter().any(|e| e == "medium") { "medium" } else { &efforts[0] })
            .to_string();
        options.push(AgentModelOption {
            id: slug.to_string(),
            label,
            efforts,
            default_effort,
        });
    }
    options
}

fn query_agy_models() -> Vec<AgentModelOption> {
    let Some(bin) = which("agy").or_else(|| which("antigravity")) else { return Vec::new() };
    let (cmd, args) = launcher(&bin);
    let mut cmd_builder = std::process::Command::new(cmd);
    cmd_builder.args(args);
    cmd_builder.arg("models");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd_builder.creation_flags(CREATE_NO_WINDOW);
    }
    let Ok(output) = cmd_builder.output() else { return Vec::new() };
    if !output.status.success() {
        return Vec::new();
    }
    parse_agy_models(&String::from_utf8_lossy(&output.stdout))
}

fn query_claude_models() -> Vec<AgentModelOption> {
    let Some(bin) = which("claude") else { return Vec::new() };
    let (cmd, args) = launcher(&bin);
    let mut cmd_builder = std::process::Command::new(cmd);
    cmd_builder.args(args);
    cmd_builder.args([
        "-p",
        "--input-format=stream-json",
        "--output-format=stream-json",
        "--verbose",
        "--no-session-persistence",
        "--disable-slash-commands",
    ]);
    cmd_builder.stdin(std::process::Stdio::piped());
    cmd_builder.stdout(std::process::Stdio::piped());
    cmd_builder.stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd_builder.creation_flags(CREATE_NO_WINDOW);
    }
    let Ok(mut child) = cmd_builder.spawn() else { return Vec::new() };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"{\"type\":\"control_request\",\"request_id\":\"1\",\"request\":{\"subtype\":\"initialize\"}}\n");
        let _ = stdin.flush();
    }
    let Ok(output) = child.wait_with_output() else { return Vec::new() };
    parse_claude_models(&String::from_utf8_lossy(&output.stdout))
}

fn query_codex_models() -> Vec<AgentModelOption> {
    if let Some(bin) = which("codex") {
        let (cmd, args) = launcher(&bin);
        let mut cmd_builder = std::process::Command::new(cmd);
        cmd_builder.args(args);
        cmd_builder.args(["debug", "models"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd_builder.creation_flags(CREATE_NO_WINDOW);
        }
        if let Ok(output) = cmd_builder.output() {
            if output.status.success() {
                let parsed = parse_codex_models(&String::from_utf8_lossy(&output.stdout));
                if !parsed.is_empty() {
                    return parsed;
                }
            }
        }
    }
    // Fallback al fichero de caché local de Codex (`~/.codex/models_cache.json`) si existe
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
    if let Some(h) = home {
        let cache_path = PathBuf::from(h).join(".codex").join("models_cache.json");
        if let Ok(text) = std::fs::read_to_string(cache_path) {
            let parsed = parse_codex_models(&text);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

/// Consulta los modelos y niveles de razonamiento disponibles para un agente CLI,
/// usando la caché de sesión salvo que `force_refresh` sea `true`.
/// @spec REQ-47
pub fn agent_model_options(agent_id: &str, force_refresh: bool) -> Vec<AgentModelOption> {
    let key = match agent_id {
        "agy" | "antigravity" => "antigravity",
        other => other,
    };
    let cache = AGENT_MODELS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if !force_refresh {
        if let Some(cached) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(key).cloned() {
            if !cached.is_empty() {
                return cached;
            }
        }
    }

    let options = match key {
        "antigravity" => {
            let mut list = query_agy_models();
            if list.is_empty() {
                list = vec![
                    AgentModelOption {
                        id: "gemini-4-argon".into(),
                        label: "Gemini 4 Argon".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "gemini-3.8-flash".into(),
                        label: "Gemini 3.8 Flash".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into()],
                        default_effort: "medium".into(),
                    },
                ];
            }
            list
        }
        "claude" => {
            let mut list = query_claude_models();
            if list.is_empty() {
                list = vec![
                    AgentModelOption {
                        id: "claude-sonnet-5-5".into(),
                        label: "Claude Sonnet 5.5".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "claude-opus-5-5".into(),
                        label: "Claude Opus 5.5".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "claude-fable-5-1".into(),
                        label: "Claude Fable 5.1".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "claude-haiku-5-5".into(),
                        label: "Claude Haiku 5.5".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                ];
            }
            list
        }
        "codex" => {
            let mut list = query_codex_models();
            if list.is_empty() {
                list = vec![
                    AgentModelOption {
                        id: "gpt-6-luna".into(),
                        label: "GPT-6-Luna".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "gpt-5.6-terra".into(),
                        label: "GPT-5.6-Terra".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                    AgentModelOption {
                        id: "gpt-5.6-luna".into(),
                        label: "GPT-5.6-Luna".into(),
                        efforts: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into(), "max".into()],
                        default_effort: "medium".into(),
                    },
                ];
            }
            list
        }
        "gemini" => vec![
            AgentModelOption {
                id: "gemini-4-argon".into(),
                label: "Gemini 4 Argon".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
            AgentModelOption {
                id: "gemini-3.8-flash".into(),
                label: "Gemini 3.8 Flash".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
        ],
        "copilot" => vec![
            AgentModelOption {
                id: "claude-sonnet-5-5".into(),
                label: "Claude Sonnet 5.5".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
            AgentModelOption {
                id: "claude-opus-5-5".into(),
                label: "Claude Opus 5.5".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
            AgentModelOption {
                id: "gemini-4-argon".into(),
                label: "Gemini 4 Argon".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
            AgentModelOption {
                id: "gemini-3.8-flash".into(),
                label: "Gemini 3.8 Flash".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
            AgentModelOption {
                id: "gpt-6-luna".into(),
                label: "GPT-6-Luna".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into()],
                default_effort: "medium".into(),
            },
        ],
        _ => vec![AgentModelOption {
            id: "default".into(),
            label: "Default".into(),
            efforts: vec!["low".into(), "medium".into(), "high".into()],
            default_effort: "medium".into(),
        }],
    };

    cache.lock().unwrap_or_else(|p| p.into_inner()).insert(key.to_string(), options.clone());
    options
}

/// Devuelve los identificadores de modelo disponibles para un agente CLI.
/// @spec REQ-47
pub fn agent_models(agent_id: &str) -> Vec<String> {
    agent_model_options(agent_id, false).into_iter().map(|m| m.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encuentra_un_programa_con_su_extension() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("claude.cmd");
        std::fs::write(&file, "").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        let found = find_in("claude", path.to_str().unwrap(), &[".exe".into(), ".cmd".into()]);
        assert_eq!(found, Some(file));
        assert_eq!(find_in("gemini", path.to_str().unwrap(), &[".exe".into(), ".cmd".into()]), None);
    }

    #[test]
    fn lee_los_modelos_de_ollama_y_de_openai() {
        assert_eq!(model_names(&json!({"models": [{"name": "qwen2.5:7b"}]})), vec!["qwen2.5:7b"]);
        assert_eq!(model_names(&json!({"data": [{"id": "local-model"}]})), vec!["local-model"]);
        assert!(model_names(&json!({})).is_empty());
    }

    #[test]
    fn en_windows_un_cmd_se_lanza_con_cmd_exe() {
        let (program, args) = launcher(Path::new("C:/nodejs/npx.cmd"));
        if cfg!(windows) {
            assert_eq!(program, PathBuf::from("cmd.exe"));
            assert_eq!(args.last().unwrap(), "C:/nodejs/npx.cmd");
        } else {
            assert!(args.is_empty());
        }
    }

    #[test]
    fn busca_antigravity_con_fallback_a_agy() {
        let dir = tempfile::tempdir().unwrap();
        let agy_file = dir.path().join("agy.exe");
        std::fs::write(&agy_file, "").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        let found = find_in("antigravity", path.to_str().unwrap(), &[".exe".into()]);
        assert_eq!(found, None);
        let found_agy = find_in("agy", path.to_str().unwrap(), &[".exe".into()]);
        assert_eq!(found_agy, Some(agy_file));
    }

    #[test]
    fn agent_models_devuelve_modelos_gemini_minimo_3_8_flash() {
        let models = agent_models("antigravity");
        assert!(models.iter().any(|m| m.contains("3.8-flash") || m.contains("3.8")));
        assert!(models.contains(&"gemini-3.8-flash".to_string()));
    }

    #[test]
    fn parsea_salida_real_de_agy_models_con_argon_y_esfuerzos() {
        let sample = "Fetching available models...\n\
            gemini-4-argon-medium\tGemini 4 Argon (Medium)\n\
            gemini-4-argon-high\tGemini 4 Argon (High)\n\
            gemini-4-argon-low\tGemini 4 Argon (Low)\n\
            gemini-3.8-flash-high\tGemini 3.8 Flash (High)\n\
            gemini-3.8-flash-medium\tGemini 3.8 Flash (Medium)\n\
            gemini-3.8-flash-low\tGemini 3.8 Flash (Low)\n\
            gpt-oss-120b-medium\tGPT-OSS 120B (Medium)\n";
        let parsed = parse_agy_models(sample);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].id, "gemini-4-argon");
        assert_eq!(parsed[0].label, "Gemini 4 Argon");
        assert_eq!(parsed[0].efforts, vec!["low", "medium", "high"]);
        assert_eq!(parsed[0].default_effort, "medium");
        assert_eq!(parsed[2].id, "gpt-oss-120b");
        assert_eq!(parsed[2].efforts, vec!["medium"]);
    }

    #[test]
    fn parsea_control_response_initialize_de_claude_code() {
        let sample = r#"{"type":"control_response","response":{"subtype":"success","request_id":"1","response":{"models":[{"value":"default","resolvedModel":"claude-sonnet-5-5","displayName":"Default (recommended)"},{"value":"opus","resolvedModel":"claude-opus-5-5","displayName":"Opus 5.5","supportsEffort":true,"supportedEffortLevels":["low","medium","high","xhigh","max"]},{"value":"sonnet","resolvedModel":"claude-sonnet-5-5","displayName":"Sonnet 5.5","supportsEffort":true,"supportedEffortLevels":["low","medium","high","xhigh","max"]}]}}}"#;
        let parsed = parse_claude_models(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "claude-sonnet-5-5");
        assert_eq!(parsed[0].label, "Claude Sonnet 5.5");
        assert_eq!(parsed[0].efforts, vec!["low", "medium", "high", "xhigh", "max"]);
        assert_eq!(parsed[1].id, "claude-opus-5-5");
        assert_eq!(parsed[1].label, "Claude Opus 5.5");
    }

    #[test]
    fn parsea_catalogo_json_de_codex_debug_models() {
        let sample = r#"{"models":[{"slug":"gpt-reserve","display_name":"GPT-Reserve","visibility":"hide","priority":4,"default_reasoning_level":"medium","supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"}]},{"slug":"gpt-6-luna","display_name":"GPT-6-Luna","visibility":"list","priority":5,"default_reasoning_level":"medium","supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"},{"effort":"high"}]},{"slug":"gpt-5.6-terra","display_name":"GPT-5.6-Terra","visibility":"list","priority":8,"default_reasoning_level":"medium","supported_reasoning_levels":[{"effort":"low"},{"effort":"medium"},{"effort":"high"},{"effort":"xhigh"}]}]}"#;
        let parsed = parse_codex_models(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, "gpt-6-luna");
        assert_eq!(parsed[0].label, "GPT-6-Luna");
        assert_eq!(parsed[1].id, "gpt-5.6-terra");
    }
}

