use crate::errors::AppError;
use crate::models::ColumnInfo;
use crate::models::DatabaseHealth;
use crate::models::Note;
use crate::models::TableInfo;
use rusqlite::Connection;
use std::path::PathBuf;
use rusqlite::Statement;
pub struct DatabaseService {
    connection: Connection,
}

impl DatabaseService {
    pub fn new(path: PathBuf) -> Result<Self, AppError> {
        let connection = Connection::open(path).map_err(|e| AppError::Message(e.to_string()))?;
        Ok(Self { connection })
    }

    #[cfg(test)]
    fn from_connection(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn ping_sqlite(&self) -> Result<String, AppError> {
        let result: i32 = self
            .connection
            .query_row("SELECT 1", [], |row| row.get(0))
            .map_err(sqlite_err)?;
        Ok(result.to_string())
    }

    pub fn db_health_check(&self) -> Result<DatabaseHealth, AppError> {
        let health = self
            .ping_sqlite()
            .map_err(|e| AppError::Message(e.to_string()))?;
        if health != "1" {
            return Ok(DatabaseHealth::new(
                false,
                false,
                Some("SQLite DB failed to ping".to_string()),
            ));
        }
        Ok(DatabaseHealth::new(
            true,
            true,
            Some("SQLite DB successfully pinged".to_string()),
        ))
    }

    pub fn inspect_table_schema(&self, table_name: &str) -> Result<Vec<ColumnInfo>, AppError> {
        let table = self.find_table_by_name(table_name)?;
        if table.is_none() {
            return Err(AppError::Message(format!(
                "Table '{table_name}' not found"
            )));
        }

        // PRAGMA table names are identifiers, not bindable values — validate first, then quote.
        let escaped_name = table_name.replace('"', "\"\"");
        let sql = format!("PRAGMA table_info(\"{escaped_name}\")");

        let mut statement = self
            .connection
            .prepare(&sql)
            .map_err(sqlite_err)?;

        let columns = statement
            .query_map([], ColumnInfo::from_row)
            .map_err(sqlite_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_err)?;
        Ok(columns)
    }

    pub fn list_tables(&self) -> Result<Vec<TableInfo>, AppError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT name, type FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
            )
            .map_err(sqlite_err)?;

        get_tables(&mut statement)
    }

    pub fn get_user_created_tables(&self) -> Result<Vec<TableInfo>, AppError> {
        let mut statement: Statement = self
            .connection
            .prepare(
                "SELECT name, type FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
            )
            .map_err(sqlite_err)?;

        get_tables(&mut statement)
    }

    pub fn find_table_by_name(&self, table_name: &str) -> Result<Option<TableInfo>, AppError> {
        let table = self.connection.query_row(
            "SELECT name, type FROM sqlite_master
                 WHERE type = 'table' AND name = ? AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            [table_name], // Bind table_name as a value parameter — never interpolate user input into SQL.
            TableInfo::from_row,
        );

        match table {
            Ok(table) => Ok(Some(table)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(sqlite_err(e)),
        }
    }
}

pub fn get_tables(statement: &mut Statement) -> Result<Vec<TableInfo>, AppError> {
    let tables = statement
            .query_map([], TableInfo::from_row)
            .map_err(sqlite_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_err)?;
    Ok(tables)
}

fn sqlite_err(e: rusqlite::Error) -> AppError {
    AppError::Message(e.to_string())
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
            Note::from_row,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;

    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_service() -> DatabaseService {
        let connection = Connection::open_in_memory().unwrap();
        DatabaseService::from_connection(connection)
    }

    fn in_memory_service_with_tables() -> DatabaseService {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        DatabaseService::from_connection(connection)
    }

    #[test]
    fn test_create_notes_table() {
        let note = create_notes_table("Test Note", "Test Content").unwrap();
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "Test Content");
    }

    #[test]
    fn test_database_service() {
        let database_service = in_memory_service();
        let health = database_service.db_health_check().unwrap();
        assert!(health.sqlite_available);
        assert!(health.sample_query_passed);
    }

    #[test]
    fn test_ping_sqlite() {
        let database_service = in_memory_service();
        let result = database_service.ping_sqlite().unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_list_tables() {
        let database_service = in_memory_service_with_tables();
        let tables = database_service.list_tables().unwrap();
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].name, "notes");
        assert_eq!(tables[0].table_type, "table");
        assert_eq!(tables[1].name, "users");
        assert_eq!(tables[1].table_type, "table");
    }

    #[test]
    fn test_find_table_by_name() {
        let database_service = in_memory_service_with_tables();
        let table = database_service.find_table_by_name("notes").unwrap();
        assert!(table.is_some());
        assert_eq!(table.unwrap().name, "notes");

        let missing = database_service.find_table_by_name("ghost").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_inspect_table_schema() {
        let database_service = in_memory_service_with_tables();
        let columns = database_service.inspect_table_schema("notes").unwrap();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].data_type, "INTEGER");
        assert!(columns[0].primary_key);
        assert_eq!(columns[1].name, "title");
        assert_eq!(columns[1].data_type, "TEXT");
        assert!(columns[1].not_null);

        let missing = database_service.inspect_table_schema("ghost");
        assert!(missing.is_err());
    }

    #[test]
    fn test_inspect_table_schema_from_file() {
        let candidates = [
            "../test_data.sqlite",
            "test_data.sqlite",
            "funny_test_data.sqlite",
        ];
        let mut opened = false;
        for path in candidates {
            if let Ok(database_service) = DatabaseService::new(PathBuf::from(path)) {
                if let Ok(tables) = database_service.list_tables() {
                    if tables.is_empty() {
                        continue;
                    }
                    opened = true;
                    let schema = database_service
                        .inspect_table_schema(&tables[0].name)
                        .unwrap_or_else(|e| panic!("schema failed for {}: {e}", tables[0].name));
                    assert!(!schema.is_empty(), "expected columns for {}", tables[0].name);
                    break;
                }
            }
        }
        assert!(opened, "could not open a sample database from known paths");
    }
}
