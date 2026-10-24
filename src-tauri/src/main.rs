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
