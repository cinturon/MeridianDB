use crate::errors::AppError;
use crate::models::Note;
use rusqlite::Connection;
use std::path::PathBuf;
use crate::models::DatabaseHealth;

pub struct DatabaseService {
    connection: Connection,
}

impl DatabaseService {
    pub fn new(path: PathBuf) -> Result<Self, AppError> {
        let connection = Connection::open(path).map_err(|e| AppError::Message(e.to_string()))?;
        Ok(Self { connection })
    }

    pub fn ping_sqlite(&self) -> Result<String, AppError> {
        let result: i32 = self.connection.query_row("SELECT 1", [], |row| row.get(0))
            .map_err(|e| AppError::Message(e.to_string()))?;
        Ok(result.to_string())
    }

    pub fn db_health_check(&self) -> Result<DatabaseHealth, AppError> {
        let health = self.ping_sqlite().map_err(|e| AppError::Message(e.to_string()))?;
        if health != "1" {
            return Ok(DatabaseHealth::new(false, false, Some("SQLite DB failed to ping".to_string())));
        }
        Ok(DatabaseHealth::new(true, true, Some("SQLite DB successfully pinged".to_string())))
    }
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
            |row| Note::from_row(row),
        )
        .map_err(|e| AppError::Message(e.to_string()))?;

    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_notes_table() {
        let note = create_notes_table("Test Note", "Test Content").unwrap();
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "Test Content");
    }

    #[test]
    fn test_database_service() {
        let database_service = DatabaseService::new(PathBuf::from("funny_test_data.sqlite")).unwrap();
        let health = database_service.db_health_check().unwrap();
        assert!(health.sqlite_available);
        assert!(health.sample_query_passed);
    }

    #[test]
    fn test_ping_sqlite() {
        let database_service = DatabaseService::new(PathBuf::from("funny_test_data.sqlite")).unwrap();
        let result = database_service.ping_sqlite().unwrap();
        assert_eq!(result, "1");
    }
}
