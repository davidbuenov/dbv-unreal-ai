// =============================================================================
// DBV Unreal AI — Compilar el código del proyecto tras un cambio de la IA
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================
//
// @spec REQ-22, R-31
//
// Compilación mixta (decidida por el usuario el 2026-11-01): si solo cambian `.cpp`, Live Coding con el editor
// abierto (`LiveCodingToolset.CompileLiveCoding`); si cambian cabeceras, `*.Build.cs` o `*.Target.cs`, Live Coding no
// basta (reflexión, módulos) y se compila con UBT con el editor cerrado. Python e `.ini` no se compilan. Los errores
// se devuelven a la IA recortados (REQ-51): el log de UBT tiene miles de líneas.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use super::mcp_client::McpSession;
use super::{discovery, UnrealError};

/// Toolset de Epic con `CompileLiveCoding`; «Preparar» lo activa en los proyectos con C++.
pub const LIVE_CODING_TOOLSET: &str = "LiveCodingToolset.LiveCodingToolset";
/// Una compilación de Live Coding puede tardar minutos en un proyecto grande.
const LIVE_CODING_TIMEOUT: Duration = Duration::from_secs(600);

/// Errores que se devuelven como mucho, y caracteres por error.
const MAX_ERRORS: usize = 20;
const MAX_ERROR_CHARS: usize = 400;

/// Cómo se compila un conjunto de ficheros cambiados.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CompilePlan {
    /// No hay C++ entre los cambios.
    Nothing,
    /// Solo `.cpp`: Live Coding con el editor abierto.
    LiveCoding,
    /// Cabeceras o reglas de módulo: hay que cerrar el editor y compilar con UBT. `files` son los que lo obligan.
    EditorClosed { files: Vec<String> },
}

/// Resultado de una compilación, ya listo para la IA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileOutcome {
    pub ok: bool,
    /// Una línea: `Success`, `NoChanges`, `Failure`… o el código de salida de UBT.
    pub summary: String,
    pub errors: Vec<String>,
}

fn extension(path: &str) -> String {
    path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default()
}

/// Decide cómo compilar a partir de las rutas cambiadas (relativas al proyecto). @spec REQ-22
pub fn compile_plan(paths: &[String]) -> CompilePlan {
    let cpp: Vec<&String> = paths.iter().filter(|p| matches!(extension(p).as_str(), "cpp" | "c" | "inl" | "h" | "hpp" | "cs")).collect();
    let structural: Vec<String> = cpp.iter().filter(|p| !matches!(extension(p).as_str(), "cpp" | "c" | "inl")).map(|p| p.to_string()).collect();
    let plan = if cpp.is_empty() {
        CompilePlan::Nothing
    } else if structural.is_empty() {
        CompilePlan::LiveCoding
    } else {
        CompilePlan::EditorClosed { files: structural }
    };
    plan
}

/// Recorta una línea de error para la IA.
fn clip(line: &str) -> String {
    let trimmed = line.trim();
    let clipped: String = trimmed.chars().take(MAX_ERROR_CHARS).collect();
    if clipped.len() < trimmed.len() { format!("{clipped}…") } else { clipped }
}

/// Errores del compilador y del enlazador en la salida de UBT o de Live Coding (no los avisos), sin repetir.
pub fn compiler_errors(output: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        let is_error = lower.contains("): error ") || lower.contains("): fatal error ") || lower.contains("error lnk") || lower.contains("error: ") && !lower.contains("0 error");
        let entry = clip(line);
        if is_error && errors.len() < MAX_ERRORS && !errors.contains(&entry) {
            errors.push(entry);
        }
    }
    errors
}

/// Interpreta lo que devuelve `CompileLiveCoding` («Result: Success» y, debajo, la salida del compilador).
pub fn live_coding_outcome(text: &str) -> CompileOutcome {
    let result = text.lines().find_map(|line| line.trim().strip_prefix("Result: ")).map(str::trim);
    let summary = match result {
        Some(result) => result.to_string(),
        // «Error: Live Coding is not enabled…» y parecidos: la primera línea ya lo explica.
        None => text.lines().next().unwrap_or("Live Coding did not answer").trim().to_string(),
    };
    let ok = matches!(result, Some("Success") | Some("NoChanges"));
    let mut errors = compiler_errors(text);
    if !ok && errors.is_empty() {
        errors.push(clip(&summary));
    }
    CompileOutcome { ok, summary, errors }
}

/// Compila con Live Coding en el editor conectado y espera el resultado. @spec REQ-22
pub fn live_coding(session: &McpSession) -> Result<CompileOutcome, UnrealError> {
    let called = session.call_toolset_tool(LIVE_CODING_TOOLSET, "CompileLiveCoding", json!({}), LIVE_CODING_TIMEOUT)?;
    let text = called.value.as_ref().and_then(Value::as_str).unwrap_or(&called.text).to_string();
    // El MCP de Epic contesta «Toolset '…' not found» como resultado con `isError`, no como error de protocolo.
    let result = if called.is_error && text.to_lowercase().contains("not found") {
        Err(UnrealError::NotFound(format!(
            "the Live Coding toolset is not enabled in this project: use «Prepare project» in DBV Unreal AI (it enables LiveCodingToolset) and restart the editor ({text})"
        )))
    } else {
        Ok(live_coding_outcome(&text))
    };
    result
}

/// Target de editor del proyecto: el `*Editor.Target.cs` de `Source/`, o `<Proyecto>Editor` si no se encuentra.
pub fn editor_target(project_path: &Path) -> String {
    let name = project_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let source = project_path.parent().map(|dir| dir.join("Source")).unwrap_or_default();
    let found = std::fs::read_dir(&source).ok().and_then(|entries| {
        entries
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().and_then(|f| f.strip_suffix(".Target.cs")).map(str::to_string))
            .find(|target| target.ends_with("Editor"))
    });
    found.unwrap_or_else(|| format!("{name}Editor"))
}

/// Programa y argumentos de UBT para compilar el editor del proyecto en este sistema.
pub fn build_command(engine_root: &Path, project_path: &Path) -> (PathBuf, Vec<String>) {
    let batch = engine_root.join("Engine").join("Build").join("BatchFiles");
    let (program, platform) = if cfg!(windows) {
        (batch.join("Build.bat"), "Win64")
    } else if cfg!(target_os = "macos") {
        (batch.join("Mac").join("Build.sh"), "Mac")
    } else {
        (batch.join("Linux").join("Build.sh"), "Linux")
    };
    let args = vec![
        editor_target(project_path),
        platform.to_string(),
        "Development".to_string(),
        format!("-Project={}", project_path.display()),
        "-WaitMutex".to_string(),
        "-NoHotReloadFromIDE".to_string(),
    ];
    (program, args)
}

/// Motor del proyecto (por su `EngineAssociation`).
pub fn engine_root_for(project_path: &Path) -> Result<PathBuf, UnrealError> {
    let project = discovery::read_uproject(project_path)?;
    let engines = discovery::find_engines();
    let engine = discovery::engine_for(&engines, &project.engine_association)
        .ok_or_else(|| UnrealError::NotFound(format!("Unreal Engine {} is not installed", project.engine_association)))?;
    Ok(PathBuf::from(&engine.root))
}

/// Compila el editor del proyecto con UBT. Se niega si su editor está abierto: UBT no puede sustituir sus DLL y la
/// compilación fallaría a medias. Bloquea hasta que termina (llamar desde un hilo aparte). @spec REQ-22
pub fn build_project(engine_root: &Path, project_path: &Path) -> Result<CompileOutcome, UnrealError> {
    if discovery::editor_open_for(project_path) {
        return Err(UnrealError::Format("the Unreal editor of this project is open: save and close it before compiling (Live Coding cannot apply header or Build.cs changes)".into()));
    }
    let (program, args) = build_command(engine_root, project_path);
    if !program.is_file() {
        return Err(UnrealError::NotFound(format!("UnrealBuildTool not found: {}", program.display())));
    }
    let output = Command::new(&program).args(&args).output().map_err(|error| UnrealError::Io(format!("{}: {error}", program.display())))?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let ok = output.status.success();
    let mut errors = compiler_errors(&text);
    if !ok && errors.is_empty() {
        // Sin errores reconocibles: las últimas líneas suelen decir qué pasó (target inexistente, SDK…).
        errors = text.lines().filter(|l| !l.trim().is_empty()).rev().take(5).map(clip).collect::<Vec<_>>().into_iter().rev().collect();
    }
    let summary = if ok { "Success".to_string() } else { format!("Failure (UnrealBuildTool exit code {})", output.status.code().unwrap_or(-1)) };
    Ok(CompileOutcome { ok, summary, errors })
}

/// Herramienta del `--mcp` para los agentes que editan el código por su cuenta (en la ventana compila la app).
pub const MCP_TOOL: &str = "compile_project_code";
pub const MCP_DESCRIPTION: &str = "Compile the project C++ after you changed it, and get the compiler errors to fix. Pass the files you changed, relative to the project folder. Only .cpp files: Live Coding in the open editor. Headers, *.Build.cs or *.Target.cs: Live Coding cannot apply them, so the Unreal editor must be closed; if it is open the tool says so: ask the user to save and close it, call again, and tell the user to reopen it from DBV Unreal AI afterwards. Python and .ini do not need compiling. After two failed fixes in a row, stop and explain the errors to the user.";

/// Texto para un agente con el resultado.
pub fn outcome_text(outcome: &CompileOutcome) -> String {
    let text = if outcome.ok {
        format!("Compiled ({}).", outcome.summary)
    } else {
        format!("It did not compile ({}). Compiler errors:
{}", outcome.summary, outcome.errors.iter().map(|e| format!("- {e}")).collect::<Vec<_>>().join("
"))
    };
    text
}

/// El `.uproject` de la carpeta del proyecto.
pub fn find_uproject(project_dir: &Path) -> Result<PathBuf, UnrealError> {
    let found = std::fs::read_dir(project_dir).ok().and_then(|entries| {
        entries.flatten().map(|entry| entry.path()).find(|path| path.extension().is_some_and(|e| e.eq_ignore_ascii_case("uproject")))
    });
    found.ok_or_else(|| UnrealError::NotFound(format!("no .uproject in {}", project_dir.display())))
}

/// `compile_project_code` del `--mcp`: decide cómo compilar y lo hace. `live` compila con Live Coding en el editor.
pub fn compile_for_agent(project_dir: &Path, files: &[String], live: impl FnOnce() -> Result<CompileOutcome, UnrealError>) -> Result<String, UnrealError> {
    let result = match compile_plan(files) {
        CompilePlan::Nothing => Ok("Nothing to compile: no C++ among the files (Python and .ini changes do not need compiling).".to_string()),
        CompilePlan::LiveCoding => live().map(|outcome| outcome_text(&outcome)),
        CompilePlan::EditorClosed { files } => {
            let uproject = find_uproject(project_dir)?;
            build_project(&engine_root_for(&uproject)?, &uproject)
                .map(|outcome| outcome_text(&outcome))
                .map_err(|error| UnrealError::Format(format!("{error} [files that need it: {}]", files.join(", "))))
        }
    };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|p| p.to_string()).collect()
    }

    /// @spec REQ-22
    #[test]
    fn solo_cpp_va_con_live_coding_y_cabeceras_o_build_cs_con_el_editor_cerrado() {
        assert_eq!(compile_plan(&paths(&["Content/Python/a.py", "Config/DefaultGame.ini"])), CompilePlan::Nothing);
        assert_eq!(compile_plan(&paths(&["Source/Demo/Private/DemoActor.cpp", "Config/DefaultGame.ini"])), CompilePlan::LiveCoding);
        assert_eq!(
            compile_plan(&paths(&["Source/Demo/Private/DemoActor.cpp", "Source/Demo/Public/DemoActor.h", "Source/Demo/Demo.Build.cs"])),
            CompilePlan::EditorClosed { files: paths(&["Source/Demo/Public/DemoActor.h", "Source/Demo/Demo.Build.cs"]) }
        );
    }

    /// @spec REQ-22
    #[test]
    fn interpreta_live_coding() {
        let ok = live_coding_outcome("Result: Success\n\nCompile Output:\nLogLiveCoding: Display: Patch creation completed");
        assert!(ok.ok && ok.errors.is_empty());
        assert!(live_coding_outcome("Result: NoChanges").ok);

        let failed = live_coding_outcome("Result: Failure\n\nCompiler Diagnostics:\nD:\\P\\Source\\Demo\\DemoActor.cpp(12): error C2065: 'Speed': undeclared identifier\nD:\\P\\Source\\Demo\\DemoActor.cpp(14): warning C4101: unused");
        assert!(!failed.ok);
        assert_eq!(failed.summary, "Failure");
        assert_eq!(failed.errors, vec![r"D:\P\Source\Demo\DemoActor.cpp(12): error C2065: 'Speed': undeclared identifier".to_string()]);

        let disabled = live_coding_outcome("Error: Live Coding is not enabled for this session. Enable it in Editor Preferences.");
        assert!(!disabled.ok);
        assert!(disabled.errors[0].contains("not enabled"));
    }

    /// @spec REQ-22, REQ-51
    #[test]
    fn los_errores_salen_sin_repetir_y_recortados() {
        let line = format!("A.cpp(1): error C1: {}", "x".repeat(1000));
        let log = format!("{line}\n{line}\nB.obj : error LNK2019: unresolved external symbol\nTotal execution time: 3 s\n0 error(s)\n");
        let errors = compiler_errors(&log);
        assert_eq!(errors.len(), 2);
        assert!(errors[0].chars().count() <= MAX_ERROR_CHARS + 1);
        assert!(errors[1].contains("LNK2019"));
        let many: String = (0..50).map(|i| format!("A.cpp({i}): error C2: e{i}\n")).collect();
        assert_eq!(compiler_errors(&many).len(), MAX_ERRORS);
    }

    /// @spec REQ-22
    #[test]
    fn la_herramienta_del_agente_compila_segun_los_ficheros() {
        let dir = tempfile::tempdir().unwrap();
        let nothing = compile_for_agent(dir.path(), &paths(&["Content/Python/a.py"]), || unreachable!()).unwrap();
        assert!(nothing.starts_with("Nothing to compile"));
        let failed = CompileOutcome { ok: false, summary: "Failure".into(), errors: vec!["A.cpp(1): error C1: x".into()] };
        let live = compile_for_agent(dir.path(), &paths(&["Source/Demo/A.cpp"]), || Ok(failed.clone())).unwrap();
        assert_eq!(live, "It did not compile (Failure). Compiler errors:
- A.cpp(1): error C1: x");
        let closed = compile_for_agent(dir.path(), &paths(&["Source/Demo/A.h"]), || unreachable!()).unwrap_err();
        assert!(closed.to_string().contains(".uproject"), "{closed}");
    }

    /// @spec REQ-22
    #[test]
    fn orden_de_ubt_con_el_target_de_editor_del_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("Demo").join("Demo.uproject");
        std::fs::create_dir_all(project.parent().unwrap().join("Source")).unwrap();
        assert_eq!(editor_target(&project), "DemoEditor");
        std::fs::write(project.parent().unwrap().join("Source").join("Demo.Target.cs"), "").unwrap();
        std::fs::write(project.parent().unwrap().join("Source").join("DemoToolsEditor.Target.cs"), "").unwrap();
        assert_eq!(editor_target(&project), "DemoToolsEditor");

        let (program, args) = build_command(Path::new("E:/UE_5.8"), &project);
        assert!(program.ends_with(if cfg!(windows) { "Build.bat" } else { "Build.sh" }));
        assert_eq!(args[0], "DemoToolsEditor");
        assert_eq!(args[2], "Development");
        assert!(args[3].starts_with("-Project=") && args[3].ends_with("Demo.uproject"));
    }
}
