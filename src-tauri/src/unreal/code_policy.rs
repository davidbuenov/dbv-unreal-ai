// =============================================================================
// DBV Unreal AI — Qué ficheros del proyecto puede cambiar la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-22, R-30
//
// Una sola definición para la conversación de la ventana (`propose_changes`, al proponer y al escribir) y para el
// `--mcp` de los agentes. Solo texto del proyecto: `Source/**` (C++ y `*.Build.cs`), `Content/Python/**` y
// `Config/*.ini`. Nunca `.uasset`/`.umap`, nada fuera del proyecto ni en `Plugins/` (los de la app la mantiene ella;
// los del usuario quedan fuera por decisión suya, 2026-11-01). Los mensajes van en inglés: los lee el modelo.

/// Plugins que instala y actualiza la app en el proyecto.
pub const APP_PLUGINS: [&str; 2] = ["DBVUnrealAI", "DBVUnrealAICpp"];

const SOURCE_EXTENSIONS: [&str; 6] = ["h", "hpp", "cpp", "c", "inl", "cs"];

const ALLOWED: &str = "only Source/** (C++ and *.Build.cs), Content/Python/** (.py) and Config/*.ini of the project can be changed";

/// Ruta relativa con `/`, sin `./` delante.
fn normalized(relative: &str) -> String {
    let unified = relative.trim().replace('\\', "/");
    unified.trim_start_matches("./").to_string()
}

/// Motivo por el que la IA no puede cambiar `relative` (relativa a la carpeta del `.uproject`), o `None` si puede.
pub fn code_path_problem(relative: &str) -> Option<String> {
    let path = normalized(relative);
    let parts: Vec<&str> = path.split('/').collect();
    let lower: Vec<String> = parts.iter().map(|p| p.to_ascii_lowercase()).collect();
    let extension = path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).filter(|ext| !ext.contains('/')).unwrap_or_default();
    let outside = path.is_empty() || path.starts_with('/') || path.contains(':') || parts.iter().any(|p| p.is_empty() || *p == "." || *p == "..");
    if outside {
        return Some(format!("{relative}: the path must be relative to the project folder, without '..'"));
    }
    if parts.iter().any(|p| p.starts_with('.')) {
        return Some(format!("{path}: hidden files and folders cannot be changed"));
    }
    if matches!(extension.as_str(), "uasset" | "umap") {
        return Some(format!("{path}: .uasset and .umap are binary assets; change them with the editor tools, never as text"));
    }
    let allowed = match lower[0].as_str() {
        "source" => parts.len() > 1 && SOURCE_EXTENSIONS.contains(&extension.as_str()),
        "content" => parts.len() > 2 && lower[1] == "python" && extension == "py",
        "config" => parts.len() == 2 && extension == "ini",
        _ => false,
    };
    let app_plugin = lower[0] == "plugins" && parts.len() > 1 && APP_PLUGINS.iter().any(|p| p.eq_ignore_ascii_case(parts[1]));
    let result = if app_plugin {
        Some(format!("{path}: the {} plugin is installed and updated by DBV Unreal AI; never change it", parts[1]))
    } else if allowed {
        None
    } else {
        Some(format!("{path}: {ALLOWED}"))
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @spec REQ-22
    #[test]
    fn deja_el_codigo_el_python_y_la_configuracion_del_proyecto() {
        for path in [
            "Source/Demo/DemoActor.h",
            "Source/Demo/Private/DemoActor.cpp",
            "Source/Demo/Demo.Build.cs",
            "Source/Demo.Target.cs",
            r"Source\Demo\DemoActor.cpp",
            "./Content/Python/tools/spawn.py",
            "Config/DefaultEngine.ini",
            "source/demo/x.CPP",
        ] {
            assert_eq!(code_path_problem(path), None, "{path}");
        }
    }

    /// @spec REQ-22, R-30
    #[test]
    fn rechaza_assets_binarios_lo_de_fuera_y_los_plugins() {
        let cases = [
            ("Content/Maps/Level.umap", "binary"),
            ("Content/Blueprints/BP_Demo.uasset", "binary"),
            ("../Otro/Source/A.cpp", "relative"),
            ("D:/Unreal/Demo/Source/A.cpp", "relative"),
            ("/etc/passwd", "relative"),
            ("Source/../Config/DefaultEngine.ini", "relative"),
            ("Plugins/DBVUnrealAI/Content/Python/dbv_unreal_ai/editor.py", "installed and updated by DBV Unreal AI"),
            ("plugins/dbvunrealaicpp/Source/X.cpp", "installed and updated by DBV Unreal AI"),
            ("Plugins/MiPlugin/Source/MiPlugin/A.cpp", "only Source/**"),
            ("Config/Windows/WindowsEngine.ini", "only Source/**"),
            ("Content/Python/readme.md", "only Source/**"),
            ("Demo.uproject", "only Source/**"),
            ("Source/.vs/x.h", "hidden"),
            ("Source", "only Source/**"),
            ("", "relative"),
        ];
        for (path, expected) in cases {
            let problem = code_path_problem(path).unwrap_or_default();
            assert!(problem.contains(expected), "{path}: «{problem}» debería decir «{expected}»");
        }
    }
}
