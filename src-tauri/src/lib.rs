mod models;
use crate::models::AppInfo;
mod errors;
use crate::errors::AppError;
mod database;
use crate::database::create_notes_table;
use crate::database::DatabaseService;
use crate::models::DatabaseHealth;
use crate::models::Note;
use crate::models::TableInfo;
use std::path::PathBuf;


#[tauri::command]
fn get_user_created_tables(path: &str) -> Result<Vec<TableInfo>, AppError> {
    let database_service =
        DatabaseService::new(PathBuf::from(path)).map_err(|e| AppError::Message(e.to_string()))?;
    database_service
        .get_user_created_tables()
        .map_err(|e| AppError::Message(e.to_string()))
}

#[tauri::command]
fn list_tables(path: &str) -> Result<Vec<TableInfo>, AppError> {
    let database_service =
        DatabaseService::new(PathBuf::from(path)).map_err(|e| AppError::Message(e.to_string()))?;
    database_service
        .list_tables()
        .map_err(|e| AppError::Message(e.to_string()))
}

#[tauri::command]
fn open_database(path: &str) -> Result<DatabaseHealth, AppError> {
    let database_service =
        DatabaseService::new(PathBuf::from(path)).map_err(|e| AppError::Message(e.to_string()))?;

    let health = database_service
        .db_health_check()
        .map_err(|e| AppError::Message(e.to_string()))?;

    Ok(health)
}

#[tauri::command]
fn create_note(title: &str, content: &str) -> Result<Note, AppError> {
    let note = create_notes_table(title, content).map_err(|e| AppError::Message(e.to_string()))?;
    Ok(note)
}

#[tauri::command]
fn get_database_health() -> Result<DatabaseHealth, AppError> {
    let database_service = DatabaseService::new(PathBuf::from("funny_test_data.sqlite"))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let health = database_service
        .db_health_check()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(health)
}

#[tauri::command]
fn ping_sqlite_command() -> Result<String, AppError> {
    let database_service = DatabaseService::new(PathBuf::from("funny_test_data.sqlite"))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let result = database_service
        .ping_sqlite()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(result)
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
    Err(AppError::NotImplemented(
        "DB Not Implemented...yet".to_string(),
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            my_command,
            get_app_info,
            check_database_support,
            ping_sqlite_command,
            create_note,
            get_database_health,
            open_database,
            list_tables,
            get_user_created_tables
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod sqlite_smoke {
    use rusqlite::Connection;

    #[test]
    fn can_open_in_memory_connection() {
        let _ = Connection::open_in_memory().expect("in-memory SQLite should open");
    }
}
