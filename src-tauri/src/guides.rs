// =============================================================================
// DBV Unreal AI — Guías de Unreal Engine sin red, desde una carpeta local
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-25, R-15
//
// `search_unreal_guides` busca en una copia local de las guías de Unreal 5.8 con el formato de MCPCLiente
// (`index_guides.json` y `guides/*.md`), que la persona elige en Ajustes → Conocimiento (D44). La app no trae ni
// descarga documentación de Epic: «Construir índice» desde la web espera a la decisión P-13. La búsqueda no usa la
// red: puntúa título, palabras clave y texto, y devuelve pocos fragmentos cortos con su URL (tope de REQ-51).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const FILE: &str = "knowledge.json";
const INDEX: &str = "index_guides.json";
/// Fragmentos que se devuelven como mucho, y caracteres de cada uno.
pub const MAX_RESULTS: usize = 5;
const EXCERPT_CHARS: usize = 1_100;
/// Palabras que no ayudan a buscar (inglés y español: la gente pregunta en su idioma).
const STOPWORDS: [&str; 44] = [
    "the", "and", "for", "with", "how", "what", "can", "you", "use", "are", "from", "this", "that", "into", "does", "unreal", "engine",
    "que", "como", "cómo", "para", "con", "una", "uno", "los", "las", "del", "por", "qué", "hago", "hacer", "puedo", "crear", "crea", "creo",
    "esta", "este", "eso", "hay", "son", "sus", "mis", "desde", "sobre",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Settings {
    folder: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndexFile {
    #[serde(default)]
    updated: String,
    guides: BTreeMap<String, Guide>,
}

#[derive(Debug, Deserialize)]
struct Guide {
    #[serde(default)]
    title: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    chunks: Vec<ChunkRef>,
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChunkRef {
    file: String,
    #[serde(default)]
    title: String,
}

/// Estado de la copia local, para Ajustes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Carpeta elegida por la persona (o `None`).
    pub folder: Option<String>,
    /// Carpeta donde está de verdad `index_guides.json`.
    pub index_folder: Option<String>,
    pub guides: usize,
    pub chunks: usize,
    pub updated: String,
    /// Qué pasa si no se puede usar (en inglés: también lo lee la IA).
    pub problem: Option<String>,
}

fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE)
}

fn load_settings(config_dir: &Path) -> Settings {
    std::fs::read_to_string(settings_path(config_dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Dónde está el índice dentro de la carpeta elegida: ella misma, `knowledge/` o la ruta de la skill de MCPCLiente.
pub fn index_folder(folder: &Path) -> Option<PathBuf> {
    [folder.to_path_buf(), folder.join("knowledge"), folder.join("skills").join("dbv-unreal-python-api").join("knowledge")]
        .into_iter()
        .find(|candidate| candidate.join(INDEX).is_file())
}

fn read_index(dir: &Path) -> Result<IndexFile, String> {
    let text = std::fs::read_to_string(dir.join(INDEX)).map_err(|e| format!("{}: {e}", dir.join(INDEX).display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{INDEX} is not a guides index: {e}"))
}

const NO_INDEX: &str = "No local copy of the Unreal Engine guides is set up. The user can choose one in DBV Unreal AI → Settings → Knowledge: a folder with index_guides.json and guides/*.md (the format of the dbv-unreal-python-api skill of MCPCLiente). Downloading the guides from the app is not available yet. Meanwhile, use unreal_describe_python_api for exact Python names.";

/// Estado de la copia local configurada.
pub fn status(config_dir: &Path) -> Status {
    let folder = load_settings(config_dir).folder;
    let index = folder.as_deref().map(Path::new).and_then(index_folder);
    let mut result = Status { folder: folder.clone(), index_folder: index.as_ref().map(|p| p.to_string_lossy().to_string()), guides: 0, chunks: 0, updated: String::new(), problem: None };
    match (&folder, &index) {
        (None, _) => result.problem = Some(NO_INDEX.to_string()),
        (Some(f), None) => result.problem = Some(format!("{f} has no {INDEX} (nor knowledge/{INDEX})")),
        (Some(_), Some(dir)) => match read_index(dir) {
            Ok(file) => {
                result.guides = file.guides.len();
                result.chunks = file.guides.values().map(|g| g.chunks.len()).sum();
                result.updated = file.updated;
            }
            Err(error) => result.problem = Some(error),
        },
    }
    result
}

/// Guarda la carpeta elegida (`None` la quita) y devuelve el estado. Una carpeta sin índice no se guarda.
pub fn set_folder(config_dir: &Path, folder: Option<&str>) -> Result<Status, String> {
    let folder = folder.map(str::trim).filter(|f| !f.is_empty());
    if let Some(f) = folder {
        index_folder(Path::new(f)).ok_or_else(|| format!("{f} has no {INDEX} (nor knowledge/{INDEX})"))?;
    }
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&Settings { folder: folder.map(str::to_string) }).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(config_dir), text).map_err(|e| e.to_string())?;
    Ok(status(config_dir))
}

/// Un término de la consulta con sus formas: la documentación escribe «Nav Mesh Bounds Volume» separado, así que
/// `NavMeshBoundsVolume` se busca también así (visto con las guías reales).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub variants: Vec<String>,
}

/// `NavMeshBoundsVolume` → `nav mesh bounds volume`; `None` si no es CamelCase.
fn split_camel(word: &str) -> Option<String> {
    let mut out = String::new();
    let mut previous_lower = false;
    let mut breaks = 0;
    for c in word.chars() {
        if c.is_uppercase() && previous_lower {
            out.push(' ');
            breaks += 1;
        }
        previous_lower = c.is_lowercase() || c.is_ascii_digit();
        out.extend(c.to_lowercase());
    }
    (breaks > 0).then_some(out)
}

/// Términos de la consulta que sirven para buscar, sin palabras de relleno ni repetidos.
pub fn terms(query: &str) -> Vec<Term> {
    let mut found: Vec<Term> = Vec::new();
    for word in query.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        let lower = word.to_lowercase();
        let fresh = !found.iter().any(|t| t.variants[0] == lower);
        if lower.chars().count() >= 3 && !STOPWORDS.contains(&lower.as_str()) && fresh {
            found.push(Term { variants: std::iter::once(lower).chain(split_camel(word)).collect() });
        }
    }
    found
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// La URL de la sección del fragmento (línea «Sección Directa»), o la de la guía.
fn section_url(text: &str, guide_url: &str) -> String {
    let found = text.lines().take(8).find(|l| l.contains("Sección Directa") || l.contains("Direct Section")).and_then(|l| {
        let start = l.find("](")? + 2;
        let end = l[start..].find(')')? + start;
        Some(l[start..end].to_string())
    });
    found.unwrap_or_else(|| guide_url.to_string())
}

/// Trozo del texto alrededor de la primera línea con un término (sin la cabecera de enlaces del fragmento).
fn excerpt(text: &str, terms: &[Term]) -> String {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim_start().starts_with('>') && l.trim() != "---").collect();
    let hit = lines.iter().position(|l| {
        let lower = l.to_lowercase();
        !l.trim_start().starts_with('#') && terms.iter().flat_map(|t| &t.variants).any(|v| lower.contains(v.as_str()))
    });
    let start = hit.unwrap_or(0).saturating_sub(2);
    let mut out = String::new();
    for line in lines[start.min(lines.len())..].iter().filter(|l| !l.trim().is_empty()) {
        if out.chars().count() + line.chars().count() > EXCERPT_CHARS {
            break;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    if out.is_empty() {
        out = text.chars().take(EXCERPT_CHARS).collect();
    }
    out.trim_end().to_string()
}

/// Un resultado de la búsqueda.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub title: String,
    pub url: String,
    pub excerpt: String,
    pub score: f64,
}

/// Un fragmento ya leído del disco.
struct Doc {
    title: String,
    url: String,
    text: String,
    /// En minúsculas, para puntuar.
    guide_title: String,
    chunk_title: String,
    meta: String,
}

/// ¿Es una ruta relativa que no sale de la carpeta del índice?
fn inside_folder(file: &str) -> bool {
    let path = Path::new(file);
    !file.is_empty() && path.is_relative() && !file.contains(':') && path.components().all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// Los fragmentos de la última carpeta usada: se leen una vez (33 MB con la copia de MCPCLiente) y se vuelven a leer
/// si cambia la carpeta o su `index_guides.json`.
type DocCache = Option<(PathBuf, Option<SystemTime>, Arc<Vec<Doc>>)>;
static DOCS: Mutex<DocCache> = Mutex::new(None);

fn docs(dir: &Path) -> Result<Arc<Vec<Doc>>, String> {
    let stamp = std::fs::metadata(dir.join(INDEX)).and_then(|m| m.modified()).ok();
    let mut cache = DOCS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let cached = cache.as_ref().filter(|(path, when, _)| path == dir && *when == stamp).map(|(_, _, docs)| Arc::clone(docs));
    let result = match cached {
        Some(docs) => Ok(docs),
        None => {
            let index = read_index(dir)?;
            let mut loaded = Vec::new();
            for guide in index.guides.values() {
                let meta = format!("{} {}", guide.keywords.join(" "), guide.category).to_lowercase();
                // Un índice manipulado no puede hacer leer (y dar a la IA) ficheros de fuera de su carpeta.
                for chunk in guide.chunks.iter().filter(|c| inside_folder(&c.file)) {
                    let Ok(text) = std::fs::read_to_string(dir.join(&chunk.file)) else { continue };
                    let title = if chunk.title.is_empty() || chunk.title == "General" { guide.title.clone() } else { format!("{} — {}", guide.title, chunk.title) };
                    loaded.push(Doc { title, url: section_url(&text, &guide.url), text, guide_title: guide.title.to_lowercase(), chunk_title: chunk.title.to_lowercase(), meta: meta.clone() });
                }
            }
            let shared = Arc::new(loaded);
            *cache = Some((dir.to_path_buf(), stamp, Arc::clone(&shared)));
            Ok(shared)
        }
    };
    result
}

const K1: f64 = 1.2;
const B: f64 = 0.75;

/// Busca en la copia local que hay en `dir` (la carpeta de `index_guides.json`). Sin red. Puntuación BM25 sobre el
/// texto (las palabras raras pesan más y una página larga no gana por repetir) más el título y las palabras clave.
/// @spec REQ-25
pub fn search_in(dir: &Path, query: &str, limit: usize) -> Result<Vec<Hit>, String> {
    let wanted = terms(query);
    if wanted.is_empty() {
        return Err("the query has no searchable words: use the names of the systems, classes or tasks (for example \"navmesh bounds volume\")".into());
    }
    let docs = docs(dir)?;
    let lowers: Vec<String> = docs.iter().map(|d| d.text.to_lowercase()).collect();
    let total = docs.len().max(1) as f64;
    let average = lowers.iter().map(String::len).sum::<usize>() as f64 / total;
    let frequency = |lower: &str, term: &Term| term.variants.iter().map(|v| count(lower, v)).sum::<usize>() as f64;
    let in_text = |text: &str, term: &Term| term.variants.iter().any(|v| text.contains(v.as_str()));
    let idf: Vec<f64> = wanted
        .iter()
        .map(|term| {
            let df = docs.iter().zip(&lowers).filter(|(d, lower)| in_text(lower, term) || in_text(&d.guide_title, term)).count() as f64;
            // Al cuadrado: la palabra específica («NavMeshBoundsVolume») manda sobre la genérica («python»), que en las
            // guías reales llenaba los resultados con páginas de Python sin nada de navegación.
            (1.0 + (total - df + 0.5) / (df + 0.5)).ln().powi(2)
        })
        .collect();
    let mut hits: Vec<(usize, f64)> = Vec::new();
    for (i, (doc, lower)) in docs.iter().zip(&lowers).enumerate() {
        let length = lower.len() as f64 / average.max(1.0);
        let mut score = 0.0;
        let mut matched = 0;
        for (term, weight) in wanted.iter().zip(&idf) {
            let tf = frequency(lower, term);
            // Por debajo del máximo de BM25 (2,2): el título ayuda a desempatar, no a ganar sin hablar del tema.
            let titles = f64::from(u8::from(in_text(&doc.guide_title, term))) + 0.75 * f64::from(u8::from(in_text(&doc.chunk_title, term))) + 0.5 * f64::from(u8::from(in_text(&doc.meta, term)));
            score += weight * (tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * length)) + titles);
            matched += usize::from(tf > 0.0 || titles > 0.0);
        }
        // Que aparezcan todos los términos pesa más que repetir uno.
        let score = if wanted.len() > 1 && matched == wanted.len() { score * 1.5 } else { score };
        if score > 0.0 {
            hits.push((i, score));
        }
    }
    hits.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| docs[a.0].title.cmp(&docs[b.0].title)));
    let found = hits
        .into_iter()
        .take(limit.clamp(1, MAX_RESULTS))
        .map(|(i, score)| Hit { title: docs[i].title.clone(), url: docs[i].url.clone(), excerpt: excerpt(&docs[i].text, &wanted), score: (score * 10.0).round() / 10.0 })
        .collect();
    Ok(found)
}

/// Texto para la IA: los fragmentos con su URL, o qué hacer si no hay copia local.
pub fn search_text(config_dir: &Path, query: &str, limit: usize) -> String {
    let folder = load_settings(config_dir).folder;
    let index = folder.as_deref().map(Path::new).and_then(index_folder);
    let text = match index {
        None => NO_INDEX.to_string(),
        Some(dir) => match search_in(&dir, query, limit) {
            Ok(hits) if hits.is_empty() => format!("No fragment of the local Unreal guides matches \"{query}\". Try other words (English names of systems and classes work best)."),
            Ok(hits) => hits.iter().map(|h| format!("## {}\n{}\n\n{}", h.title, h.url, h.excerpt)).collect::<Vec<_>>().join("\n\n---\n\n"),
            Err(error) => format!("error: {error}"),
        },
    };
    text
}

/// La herramienta, igual para el asistente integrado y el MCP propio (D10).
pub fn spec() -> Value {
    json!({
        "name": "search_unreal_guides",
        "description": "Search the Unreal Engine 5.8 guides (concepts and how-to pages of dev.epicgames.com) in the local copy the user set up, without the network. Returns up to 5 short fragments with their source URL. Use it for \"how do I…\" questions and before using a system you do not know well; for exact Python names use unreal_describe_python_api. Search with English names of systems and classes. Read-only.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RESULTS }
            },
            "required": ["query"]
        }
    })
}

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Estado de la copia local de las guías, para Ajustes → Conocimiento. @spec REQ-25
#[tauri::command]
pub fn guides_status(app: tauri::AppHandle) -> Result<Status, String> {
    Ok(status(&config_dir(&app)?))
}

/// Elige (o quita, con `null`) la carpeta de las guías. @spec REQ-25
#[tauri::command]
pub fn guides_set_folder(app: tauri::AppHandle, folder: Option<String>) -> Result<Status, String> {
    set_folder(&config_dir(&app)?, folder.as_deref())
}

/// `search_unreal_guides` para el asistente integrado: el mismo texto que recibe un agente. @spec REQ-25
#[tauri::command]
pub async fn guides_search(app: tauri::AppHandle, query: String, limit: Option<usize>) -> Result<String, String> {
    let dir = config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || search_text(&dir, &query, limit.unwrap_or(3))).await.map_err(|e| e.to_string())
}

/// La definición de la herramienta, la misma que publica el MCP propio. @spec REQ-25
#[tauri::command]
pub fn guides_tool_spec() -> Value {
    spec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una copia local mínima con el formato de MCPCLiente.
    fn knowledge(dir: &Path) -> PathBuf {
        let root = dir.join("MCPCLiente").join("skills").join("dbv-unreal-python-api").join("knowledge");
        std::fs::create_dir_all(root.join("guides")).unwrap();
        std::fs::write(root.join("guides").join("nav_1.md"), "# Basic Navigation — Navigation Mesh\n\n> **Guía Original**: [Basic Navigation](https://dev.epicgames.com/x/basic-navigation)\n> **Sección Directa**: [Navigation Mesh](https://dev.epicgames.com/x/basic-navigation#navmesh)\n\n---\n\n# Basic Navigation\n\nIntro line.\nAdd a NavMeshBoundsVolume to the level and scale it to cover the walkable floor.\nPress P to see the navigation mesh.\n").unwrap();
        std::fs::write(root.join("guides").join("light_1.md"), "# Lighting\n\nThe directional light simulates the sun.\n").unwrap();
        let index = json!({ "updated": "2026-07-02", "total_guides": 2, "guides": {
            "basic-navigation": { "title": "Basic Navigation", "category": "ai", "url": "https://dev.epicgames.com/x/basic-navigation", "keywords": ["navigation", "navmesh"], "chunks": [{ "file": "guides/nav_1.md", "title": "Navigation Mesh", "size": 300 }] },
            "lighting": { "title": "Lighting", "category": "lighting", "url": "https://dev.epicgames.com/x/lighting", "keywords": ["light"], "chunks": [{ "file": "guides/light_1.md", "title": "General", "size": 50 }] }
        } });
        std::fs::write(root.join(INDEX), index.to_string()).unwrap();
        root
    }

    /// @spec REQ-25
    #[test]
    fn sin_copia_local_explica_como_tenerla() {
        let config = tempfile::tempdir().unwrap();
        let text = search_text(config.path(), "navmesh", 3);
        assert!(text.contains("Settings → Knowledge") && text.contains("index_guides.json"), "{text}");
        assert!(status(config.path()).problem.is_some());
    }

    /// Se acepta la raíz de MCPCLiente y se encuentra el índice dentro; una carpeta sin índice no se guarda.
    /// @spec REQ-25
    #[test]
    fn elige_la_carpeta_y_cuenta_guias_y_fragmentos() {
        let dir = tempfile::tempdir().unwrap();
        let root = knowledge(dir.path());
        let config = dir.path().join("config");
        let state = set_folder(&config, Some(&dir.path().join("MCPCLiente").to_string_lossy())).unwrap();
        assert_eq!((state.guides, state.chunks, state.updated.as_str()), (2, 2, "2026-07-02"));
        assert_eq!(state.index_folder.as_deref(), Some(root.to_string_lossy().as_ref()));
        assert!(set_folder(&config, Some(&dir.path().to_string_lossy())).unwrap_err().contains("index_guides.json"));
        assert_eq!(set_folder(&config, None).unwrap().folder, None);
    }

    /// «cómo se crea un NavMeshBoundsVolume por Python» → el fragmento de navegación con la URL de su sección.
    /// @spec REQ-25, REQ-51
    #[test]
    fn busca_sin_red_y_devuelve_el_fragmento_con_su_url() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        knowledge(dir.path());
        set_folder(&config, Some(&dir.path().join("MCPCLiente").to_string_lossy())).unwrap();
        let text = search_text(&config, "cómo se crea un NavMeshBoundsVolume por Python", 3);
        assert!(text.starts_with("## Basic Navigation — Navigation Mesh\nhttps://dev.epicgames.com/x/basic-navigation#navmesh"), "{text}");
        assert!(text.contains("Add a NavMeshBoundsVolume") && !text.contains("Guía Original"), "{text}");
        assert!(!text.contains("Lighting"), "solo lo que coincide");
        assert!(search_text(&config, "zzzz qqqq", 3).starts_with("No fragment"));
        assert!(search_text(&config, "cómo para", 3).contains("no searchable words"));
    }

    /// Con la copia real de MCPCLiente (5.294 fragmentos): `DBV_GUIDES=<carpeta> cargo test guias_reales -- --ignored --nocapture`.
    /// @spec REQ-25
    #[test]
    #[ignore = "necesita la copia local de las guías (DBV_GUIDES)"]
    fn guias_reales() {
        let dir = index_folder(Path::new(&std::env::var("DBV_GUIDES").expect("DBV_GUIDES"))).expect("index_guides.json");
        for query in ["cómo se crea un NavMeshBoundsVolume por Python", "skylight real time capture", "enhanced input action mapping", "spawn actor from python script", "metahuman animation retarget", "world partition streaming"] {
            let started = std::time::Instant::now();
            let hits = search_in(&dir, query, 3).unwrap();
            println!("{query} ({} ms):", started.elapsed().as_millis());
            for hit in &hits {
                println!("  {} [{}] {}", hit.title, hit.score, hit.url);
            }
            assert!(!hits.is_empty(), "{query}");
        }
    }

    /// @spec REQ-25, R-15
    #[test]
    fn un_indice_no_lee_fuera_de_su_carpeta() {
        assert!(inside_folder("guides/nav_1.md"));
        for bad in ["../secret.md", "guides/../../x.md", "/etc/passwd", "C:/Users/x/.ssh/id", ""] {
            assert!(!inside_folder(bad), "{bad}");
        }
    }

    /// @spec REQ-25
    #[test]
    fn palabras_de_busqueda_sin_relleno() {
        let found = terms("¿Cómo se crea un NavMeshBoundsVolume por Python?");
        assert_eq!(found.iter().map(|t| t.variants.clone()).collect::<Vec<_>>(), vec![vec!["navmeshboundsvolume".to_string(), "nav mesh bounds volume".to_string()], vec!["python".to_string()]]);
        assert_eq!(terms("directional light light").len(), 2);
    }
}
