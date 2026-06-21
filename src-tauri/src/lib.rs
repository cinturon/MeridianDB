mod models;
use crate::models::AppInfo;
mod errors;
use crate::errors::AppError;

#[cfg(test)]
mod sqlite_smoke {
    use rusqlite::Connection;

    #[test]
    fn can_open_in_memory_connection() {
        let _ = Connection::open_in_memory().expect("in-memory SQLite should open");
    }
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "MeridianDB".to_string(),
        version: "0.1.0".to_string(),
        description: "A desktop SQLite database explorer".to_string(),
    }
}


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn my_command() -> String {
    "Hello, world!".to_string()
}

#[tauri::command]
fn check_database_support() -> Result<(), AppError> {
    Err(AppError::NotImplemented("DB Not Implemented...yet".to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, my_command, get_app_info, check_database_support])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
