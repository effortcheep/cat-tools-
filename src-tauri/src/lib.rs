// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod print;
mod port_checker;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            print::get_printers,
            print::get_default_printer,
            print::print_pdf,
            print::save_temp_pdf,
            print::delete_temp_file,
            port_checker::get_ports,
            port_checker::kill_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
