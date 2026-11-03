// =============================================================================
// DBV Unreal AI — Punto de entrada del binario (ventana Tauri y servidor --mcp)
// Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
// Licensed under the MIT License. See LICENSE for details.
// Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
// =============================================================================

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `--mcp`: en lugar de la ventana, servidor MCP por stdio para agentes (REQ-20). Un ejecutable de
// subsistema «windows» sigue heredando las tuberías de stdin/stdout del proceso que lo lanza.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(desktop)]
    if args.iter().any(|a| a == "--mcp") {
        std::process::exit(dbv_unreal_ai_lib::mcp_server::run(&args));
    }
    dbv_unreal_ai_lib::run()
}
