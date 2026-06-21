use crate::errors::AppError;
use crate::models::Note;
use rusqlite::Connection;

pub fn ping_sqlite() -> Result<String, AppError> {
    let conn = Connection::open_in_memory().map_err(|e| AppError::Message(e.to_string()))?;
    let result: i32 = conn
        .query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|e| AppError::Message(e.to_string()))?;

    if result != 1 {
        return Err(AppError::Message("SQLite ping failed".to_string()));
    }

    Ok(result.to_string())
}

pub fn create_notes_table(title: &str, content: &str) -> Result<Note, AppError> {
    let conn = Connection::open_in_memory().map_err(|e| AppError::Message(e.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;

    let inserted = conn
        .execute(
            "INSERT INTO notes (title, content) VALUES (?, ?)",
            [title, content],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;

    if inserted != 1 {
        return Err(AppError::Message("Failed to insert note".to_string()));
    }

    
    let note = conn
        .query_row(
            "SELECT id, title, content FROM notes WHERE title = ?",
            [title],
            |row| {
                Note::from_row(row)
            },
        )
        .map_err(|e| AppError::Message(e.to_string()))?;

    Ok(note)
}