// deskBuddy — Desktop Pet Zero (Tauri 2 + SvelteKit)
// Fase 0: scaffold mínimo. Plugins e comandos entram nas próximas fases.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running deskbuddy");
}
