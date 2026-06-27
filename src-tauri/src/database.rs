use crate::errors::AppError;
use crate::models::CellEditRequest;
use crate::models::CellEditResult;
use crate::models::ChangeHistoryEntry;
use crate::models::ColumnInfo;
use crate::models::DatabaseHealth;
use crate::models::Note;
use crate::models::QueryResult;
use crate::models::TableInfo;
use crate::models::TablePreview;
use rusqlite::Connection;
use rusqlite::Statement;
use std::path::PathBuf;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
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
            return Err(AppError::Message(format!("Table '{table_name}' not found")));
        }

        // PRAGMA table names are identifiers, not bindable values — validate first, then quote.
        let escaped_name = table_name.replace('"', "\"\"");
        let sql = format!("PRAGMA table_info(\"{escaped_name}\")");

        let mut statement = self.connection.prepare(&sql).map_err(sqlite_err)?;

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

    pub fn table_preview(&self, table_name: &str) -> Result<TablePreview, AppError> {
        let table = self.find_table_by_name(table_name)?;
        if table.is_none() {
            return Err(AppError::Message(format!("Table '{table_name}' not found")));
        }

        let limit = TablePreview::default().limit;
        let columns = self.inspect_table_schema(table_name)?;
        let column_names: Vec<String> = columns.iter().map(|column| column.name.clone()).collect();
        let rows = self.get_table_rows(table_name, limit)?;

        Ok(TablePreview::new(column_names, rows, limit))
    }

    pub fn get_table_rows(
        &self,
        table_name: &str,
        limit: i64,
    ) -> Result<Vec<Vec<String>>, AppError> {
        let escaped_name = table_name.replace('"', "\"\"");
        let sql = format!("SELECT * FROM \"{escaped_name}\" LIMIT ?");
        let mut statement = self.connection.prepare(&sql).map_err(sqlite_err)?;
        let rows = statement
            .query_map([limit], |row| {
                let mut cells = Vec::new();
                for i in 0..row.as_ref().column_count() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    cells.push(value_to_string(value));
                }
                Ok(cells)
            })
            .map_err(sqlite_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_err)?;
        Ok(rows)
    }

    pub fn query(&self, sql: &str) -> Result<QueryResult, AppError> {
        let trimmed = sql.trim();
        if trimmed.is_empty() {
            return Err(AppError::Message(
                "Query failed: Enter a SELECT query before running.".to_string(),
            ));
        }
        if !trimmed.to_ascii_lowercase().starts_with("select") {
            return Err(AppError::Message(
                "Query failed: Only read-only SELECT queries are supported.".to_string(),
            ));
        }

        let start_time = Instant::now();
        let mut statement = self.connection.prepare(trimmed).map_err(sqlite_err)?;
        let columns = statement
            .column_names()
            .iter()
            .map(|name| name.to_string())
            .collect();

        let rows = statement
            .query_map([], |row| {
                let mut cells = Vec::new();
                for i in 0..row.as_ref().column_count() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    cells.push(value_to_string(value));
                }
                Ok(cells)
            })
            .map_err(sqlite_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_err)?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(QueryResult::new(columns, rows, Some(duration_ms)))
    }

    pub fn primary_key_column(&self, table_name: &str) -> Result<Option<String>, AppError> {
        let table = self.find_table_by_name(table_name)?;
        if table.is_none() {
            return Err(AppError::Message(format!("Table '{table_name}' not found")));
        }

        let columns = self.inspect_table_schema(table_name)?;

        let primary_key_column: Vec<&ColumnInfo> =
            columns.iter().filter(|column| column.primary_key).collect();

        match primary_key_column.len() {
            0 => Ok(None),
            1 => Ok(Some(primary_key_column[0].name.clone())),
            _ => Ok(None),
        }
    }

    pub fn validate_cell_edit_request(&self, request: &CellEditRequest) -> Result<(), AppError> {
        if request.table_name.trim().is_empty() {
            return Err(AppError::Message("Table name is required".to_string()));
        }
        if request.primary_key_column.trim().is_empty() {
            return Err(AppError::Message(
                "Primary key column is required".to_string(),
            ));
        }
        if request.primary_key_value.trim().is_empty() {
            return Err(AppError::Message(
                "Primary key value is required".to_string(),
            ));
        }
        if request.target_column.trim().is_empty() {
            return Err(AppError::Message("Column name is required".to_string()));
        }

        let table = self.find_table_by_name(&request.table_name)?;
        if table.is_none() {
            return Err(AppError::Message(format!(
                "Table '{}' not found",
                request.table_name
            )));
        }

        let detected_pk = self.primary_key_column(&request.table_name)?;
        let Some(detected_pk) = detected_pk else {
            return Err(AppError::Message(format!(
                "Table '{}' has no supported primary key for editing",
                request.table_name
            )));
        };

        if request.primary_key_column != detected_pk {
            return Err(AppError::Message(
                "Primary key column does not match table schema".to_string(),
            ));
        }

        if request.target_column == detected_pk {
            return Err(AppError::Message(format!(
                "Cannot edit primary key column '{}'",
                detected_pk
            )));
        }

        let columns = self.inspect_table_schema(&request.table_name)?;
        if !columns
            .iter()
            .any(|column| column.name == request.target_column)
        {
            return Err(AppError::Message(format!(
                "Column '{}' not found in table '{}'",
                request.target_column, request.table_name
            )));
        }

        Ok(())
    }

    pub fn update_cell(&self, request: &CellEditRequest) -> Result<CellEditResult, AppError> {
        self.validate_cell_edit_request(request)?;

        let escaped_table_name = request.table_name.replace('"', "\"\"");
        let escaped_primary_key_column = request.primary_key_column.replace('"', "\"\"");
        let escaped_target_column = request.target_column.replace('"', "\"\"");

        let sql = format!(
            "UPDATE \"{escaped_table_name}\" SET \"{escaped_target_column}\" = ? WHERE \"{escaped_primary_key_column}\" = ?"
        );

        ensure_change_history_table(&self.connection)?;

        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(sqlite_err)?;

        let rows_updated = transaction
            .execute(
                &sql,
                rusqlite::params![
                    request.new_value.as_deref(),
                    request.primary_key_value.as_str(),
                ],
            )
            .map_err(sqlite_err)?;

        if rows_updated > 0 {
            let entry = ChangeHistoryEntry::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
                request.clone(),
            );
            insert_change_history(&transaction, &entry)?;
        }

        transaction.commit().map_err(sqlite_err)?;

        Ok(CellEditResult::new(rows_updated as i64))
    }

    pub fn get_change_history(&self, table_name: Option<String>) -> Result<Vec<ChangeHistoryEntry>, AppError> {
        ensure_change_history_table(&self.connection)?;

        let where_clause = if table_name.is_some() {
            format!("WHERE table_name = ?")
        } else {
            "".to_string()
        };

        let sql = format!(
            "SELECT timestamp, table_name, primary_key_column, primary_key_value, target_column, new_value, original_value
             FROM meridian_change_history
             {where_clause}
             ORDER BY id DESC
             LIMIT 25",
        );

        let mut statement = self.connection.prepare(&sql).map_err(sqlite_err)?;
        
        let rows = match table_name {
            Some(name_value) => {
                statement.query_map([name_value], ChangeHistoryEntry::from_row)
            }
            None => {
                statement.query_map([], ChangeHistoryEntry::from_row)
            }
        };

        let entries = rows.map_err(sqlite_err)?.collect::<Result<Vec<_>, _>>().map_err(sqlite_err)?;

        Ok(entries)
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

fn value_to_string(value: rusqlite::types::Value) -> String {
    match value {
        rusqlite::types::Value::Null => "NULL".to_string(),
        rusqlite::types::Value::Integer(i) => i.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        rusqlite::types::Value::Text(s) => s,
        rusqlite::types::Value::Blob(b) => format!("<blob {} bytes>", b.len()),
    }
}

const CHANGE_HISTORY_INSERT: &str = "INSERT INTO meridian_change_history (
    timestamp,
    table_name,
    primary_key_column,
    primary_key_value,
    target_column,
    new_value,
    original_value
) VALUES (?, ?, ?, ?, ?, ?, ?)";

fn insert_change_history(
    conn: &Connection,
    entry: &ChangeHistoryEntry,
) -> Result<(), AppError> {
    let new_value = entry
        .cell_edit_request
        .new_value
        .clone()
        .unwrap_or_else(|| "NULL".to_string());
    let original_value = entry
        .cell_edit_request
        .original_value
        .clone()
        .unwrap_or_else(|| "NULL".to_string());

    conn.execute(
        CHANGE_HISTORY_INSERT,
        rusqlite::params![
            entry.timestamp,
            entry.cell_edit_request.table_name,
            entry.cell_edit_request.primary_key_column,
            entry.cell_edit_request.primary_key_value,
            entry.cell_edit_request.target_column,
            new_value,
            original_value,
        ],
    )
    .map_err(sqlite_err)?;

    Ok(())
}

pub fn ensure_change_history_table(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS meridian_change_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            table_name TEXT NOT NULL,
            primary_key_column TEXT NOT NULL,
            primary_key_value TEXT NOT NULL,
            target_column TEXT NOT NULL,
            new_value TEXT NOT NULL,
            original_value TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;

    Ok(())
}

pub fn record_change_history(
    conn: &Connection,
    entry: &ChangeHistoryEntry,
) -> Result<(), AppError> {
    insert_change_history(conn, entry)
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
        connection
            .execute(
                "CREATE TABLE items (name TEXT NOT NULL, qty INTEGER NOT NULL)",
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
        assert_eq!(tables.len(), 3);
        assert_eq!(tables[0].name, "items");
        assert_eq!(tables[0].table_type, "table");
        assert_eq!(tables[1].name, "notes");
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
                    assert!(
                        !schema.is_empty(),
                        "expected columns for {}",
                        tables[0].name
                    );
                    break;
                }
            }
        }
        assert!(opened, "could not open a sample database from known paths");
    }

    #[test]
    fn test_table_preview() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE items (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    qty INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO items (name, qty) VALUES ('apple', 3)", [])
            .unwrap();
        connection
            .execute("INSERT INTO items (name, qty) VALUES ('banana', 5)", [])
            .unwrap();

        let database_service = DatabaseService::from_connection(connection);
        let preview = database_service.table_preview("items").unwrap();

        assert_eq!(preview.columns, vec!["id", "name", "qty"]);
        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.rows[0][1], "apple");
        assert_eq!(preview.rows[0][2], "3");
        assert_eq!(preview.rows[1][1], "banana");
        assert_eq!(preview.limit, 25);

        let missing = database_service.table_preview("ghost");
        assert!(missing.is_err());
    }

    #[test]
    fn test_query_returns_columns_and_rows() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO items (name) VALUES ('alpha')", [])
            .unwrap();
        connection
            .execute("INSERT INTO items (name) VALUES ('beta')", [])
            .unwrap();

        let database_service = DatabaseService::from_connection(connection);
        let result = database_service
            .query("SELECT id, name FROM items ORDER BY id")
            .unwrap();

        assert_eq!(result.columns, vec!["id", "name"]);
        assert_eq!(result.row_count, 2);
        assert_eq!(result.rows[0][1], "alpha");
        assert!(result.duration_ms.is_some());
    }

    fn query_error_message(result: Result<QueryResult, AppError>) -> String {
        match result {
            Ok(_) => panic!("expected query to fail"),
            Err(AppError::Message(message)) => message,
            Err(other) => panic!("unexpected error variant: {other}"),
        }
    }

    #[test]
    fn test_query_rejects_empty_sql() {
        let database_service = in_memory_service();
        let message = query_error_message(database_service.query(""));
        assert_eq!(
            message,
            "Query failed: Enter a SELECT query before running."
        );
    }

    #[test]
    fn test_query_rejects_whitespace_only_sql() {
        let database_service = in_memory_service();
        let message = query_error_message(database_service.query("   \n\t  "));
        assert_eq!(
            message,
            "Query failed: Enter a SELECT query before running."
        );
    }

    #[test]
    fn test_query_rejects_non_select_sql() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        let database_service = DatabaseService::from_connection(connection);

        let delete_message = query_error_message(database_service.query("DELETE FROM items"));
        assert_eq!(
            delete_message,
            "Query failed: Only read-only SELECT queries are supported."
        );

        let insert_message =
            query_error_message(database_service.query("INSERT INTO items (name) VALUES ('x')"));
        assert_eq!(
            insert_message,
            "Query failed: Only read-only SELECT queries are supported."
        );
    }

    #[test]
    fn test_query_surfaces_sqlite_syntax_error() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        let database_service = DatabaseService::from_connection(connection);

        let message = query_error_message(database_service.query("SELECT FROM items"));
        assert!(
            message.contains("syntax error"),
            "expected SQLite syntax detail, got: {message}"
        );
        assert!(
            message.contains("FROM"),
            "expected parser location hint, got: {message}"
        );
    }

    #[test]
    fn test_query_populates_duration_ms() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();

        connection
            .execute("INSERT INTO items (name) VALUES ('alpha')", [])
            .unwrap();

        let database_service = DatabaseService::from_connection(connection);
        let result = database_service
            .query("SELECT id, name FROM items ORDER BY id")
            .unwrap();
        assert!(result.duration_ms.is_some());
        assert!(result.duration_ms.unwrap() < 1000);
    }

    #[test]
    fn test_primary_key_column() {
        let database_service = in_memory_service_with_tables();
        let primary_key_column = database_service.primary_key_column("notes").unwrap();
        assert!(primary_key_column.is_some());
        assert_eq!(primary_key_column.unwrap(), "id");
    }

    #[test]
    fn test_primary_key_column_multiple() {
        let database_service = in_memory_service_with_tables();
        let primary_key_column = database_service.primary_key_column("items").unwrap();
        assert!(primary_key_column.is_none());
    }

    fn notes_edit_request(
        primary_key_column: &str,
        primary_key_value: &str,
        target_column: &str,
    ) -> CellEditRequest {
        notes_edit_request_with_value(
            primary_key_column,
            primary_key_value,
            target_column,
            Some("New Title".to_string()),
        )
    }

    fn notes_edit_request_with_value(
        primary_key_column: &str,
        primary_key_value: &str,
        target_column: &str,
        new_value: Option<String>,
    ) -> CellEditRequest {
        CellEditRequest::new(
            "notes".to_string(),
            primary_key_column.to_string(),
            primary_key_value.to_string(),
            target_column.to_string(),
            new_value,
            Some("Original Title".to_string()),
        )
    }

    #[test]
    fn test_validate_cell_edit_request_accepts_valid_edit() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("id", "1", "title");
        assert!(database_service
            .validate_cell_edit_request(&request)
            .is_ok());
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_unknown_table() {
        let database_service = in_memory_service_with_tables();
        let request = CellEditRequest::new(
            "ghost".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            None,
            None,
        );
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_unknown_column() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("id", "1", "ghost");
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_primary_key_target() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("id", "1", "id");
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert_eq!(err.to_string(), "Cannot edit primary key column 'id'");
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_table_without_primary_key() {
        let database_service = in_memory_service_with_tables();
        let request = CellEditRequest::new(
            "items".to_string(),
            "id".to_string(),
            "1".to_string(),
            "name".to_string(),
            None,
            None,
        );
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert!(err.to_string().contains("no supported primary key"));
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_mismatched_primary_key_column() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("wrong_pk", "1", "title");
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Primary key column does not match table schema"
        );
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_empty_table_name() {
        let database_service = in_memory_service_with_tables();
        let request = CellEditRequest::new(
            "   ".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            None,
            None,
        );
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert_eq!(err.to_string(), "Table name is required");
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_empty_target_column() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("id", "1", "   ");
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert_eq!(err.to_string(), "Column name is required");
    }

    #[test]
    fn test_validate_cell_edit_request_rejects_empty_primary_key_value() {
        let database_service = in_memory_service_with_tables();
        let request = notes_edit_request("id", "   ", "title");
        let err = database_service
            .validate_cell_edit_request(&request)
            .unwrap_err();
        assert_eq!(err.to_string(), "Primary key value is required");
    }

    fn in_memory_service_with_notes_row() -> DatabaseService {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE notes (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO notes (id, title) VALUES (1, 'Original Title')",
                [],
            )
            .unwrap();
        DatabaseService::from_connection(connection)
    }

    #[test]
    fn test_update_cell_updates_one_row_and_readback() {
        let database_service = in_memory_service_with_notes_row();
        let request = CellEditRequest::new(
            "notes".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            Some("Updated Title".to_string()),
            Some("Original Title".to_string()),
        );

        let result = database_service.update_cell(&request).unwrap();
        assert_eq!(result.rows_updated, 1);

        let title: String = database_service
            .connection
            .query_row("SELECT title FROM notes WHERE id = ?", ["1"], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Updated Title");
    }

    #[test]
    fn test_update_cell_rejects_invalid_request_without_updating() {
        let database_service = in_memory_service_with_notes_row();
        let request = notes_edit_request("id", "1", "ghost");

        assert!(database_service.update_cell(&request).is_err());

        let title: String = database_service
            .connection
            .query_row("SELECT title FROM notes WHERE id = ?", ["1"], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Original Title");
    }

    #[test]
    fn test_update_cell_returns_zero_rows_for_missing_primary_key() {
        let database_service = in_memory_service_with_notes_row();
        let request = CellEditRequest::new(
            "notes".to_string(),
            "id".to_string(),
            "999".to_string(),
            "title".to_string(),
            Some("Updated Title".to_string()),
            None,
        );

        let result = database_service.update_cell(&request).unwrap();
        assert_eq!(result.rows_updated, 0);

        let title: String = database_service
            .connection
            .query_row("SELECT title FROM notes WHERE id = ?", ["1"], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Original Title");
    }

    #[test]
    fn test_update_cell_rejects_primary_key_edit() {
        let database_service = in_memory_service_with_notes_row();
        let request = notes_edit_request("id", "1", "id");

        assert!(database_service.update_cell(&request).is_err());
    }

    #[test]
    fn test_update_cell_rolls_back_on_not_null_violation() {
        let database_service = in_memory_service_with_notes_row();
        let request = notes_edit_request_with_value("id", "1", "title", None);

        assert!(database_service.update_cell(&request).is_err());

        let title: String = database_service
            .connection
            .query_row("SELECT title FROM notes WHERE id = ?", ["1"], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Original Title");
    }

    #[test]
    fn test_update_cell_sequential_commits_persist_last_value() {
        let database_service = in_memory_service_with_notes_row();
        let request1 =
            notes_edit_request_with_value("id", "1", "title", Some("First Update".to_string()));
        let request2 =
            notes_edit_request_with_value("id", "1", "title", Some("Second Update".to_string()));

        let result1 = database_service.update_cell(&request1).unwrap();
        let result2 = database_service.update_cell(&request2).unwrap();
        assert_eq!(result1.rows_updated, 1);
        assert_eq!(result2.rows_updated, 1);

        let title: String = database_service
            .connection
            .query_row("SELECT title FROM notes WHERE id = ?", ["1"], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Second Update");
    }

    #[test]
    fn test_ensure_change_history_table() {
        let connection = Connection::open_in_memory().unwrap();
        assert!(ensure_change_history_table(&connection).is_ok());
    }

    #[test]
    fn test_ensure_change_history_table_already_exists() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute(
            "CREATE TABLE meridian_change_history (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL, table_name TEXT NOT NULL, primary_key_column TEXT NOT NULL, primary_key_value TEXT NOT NULL, target_column TEXT NOT NULL, new_value TEXT NOT NULL, original_value TEXT NOT NULL)",
            [],
        ).unwrap();
        assert!(ensure_change_history_table(&connection).is_ok());
    }

    #[test]
    fn test_update_cell_records_change_history_row() {
        let database_service = in_memory_service_with_notes_row();
        let request = CellEditRequest::new(
            "notes".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            Some("Updated Title".to_string()),
            Some("Original Title".to_string()),
        );

        database_service.update_cell(&request).unwrap();

        let (table_name, pk_col, pk_val, target_col, new_val, orig_val): (
            String,
            String,
            String,
            String,
            String,
            String,
        ) = database_service
            .connection
            .query_row(
                "SELECT table_name, primary_key_column, primary_key_value, target_column, new_value, original_value
                 FROM meridian_change_history
                 LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(table_name, "notes");
        assert_eq!(pk_col, "id");
        assert_eq!(pk_val, "1");
        assert_eq!(target_col, "title");
        assert_eq!(new_val, "Updated Title");
        assert_eq!(orig_val, "Original Title");
    }

    #[test]
    fn test_update_cell_failure_does_not_record_change_history() {
        let database_service = in_memory_service_with_notes_row();
        let request = notes_edit_request_with_value("id", "1", "title", None);

        assert!(database_service.update_cell(&request).is_err());

        let count: i64 = database_service
            .connection
            .query_row(
                "SELECT COUNT(*) FROM meridian_change_history",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_change_history() {
        let database_service = in_memory_service_with_notes_row();
        let request = CellEditRequest::new(
            "notes".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            Some("Updated Title".to_string()),
            Some("Original Title".to_string()),
        );
        database_service.update_cell(&request).unwrap();

        let change_history = database_service.get_change_history(None).unwrap();
        assert_eq!(change_history.len(), 1);
        assert_eq!(change_history[0].cell_edit_request.table_name, "notes");
        assert_eq!(change_history[0].cell_edit_request.primary_key_column, "id");
        assert_eq!(change_history[0].cell_edit_request.primary_key_value, "1");
        assert_eq!(change_history[0].cell_edit_request.target_column, "title");
        assert_eq!(change_history[0].cell_edit_request.new_value, Some("Updated Title".to_string()));
        assert_eq!(change_history[0].cell_edit_request.original_value, Some("Original Title".to_string()));
    }

    #[test]
    fn test_get_change_history_returns_empty_list() {
        let database_service = in_memory_service_with_notes_row();

        let change_history = database_service.get_change_history(None).unwrap();

        assert!(change_history.is_empty());
    }

    #[test]
    fn test_get_change_history_orders_newest_first() {
        let database_service = in_memory_service_with_notes_row();
        let requests = [
            notes_edit_request_with_value("id", "1", "title", Some("First Update".to_string())),
            notes_edit_request_with_value("id", "1", "title", Some("Second Update".to_string())),
            notes_edit_request_with_value("id", "1", "title", Some("Third Update".to_string())),
        ];

        for request in &requests {
            database_service.update_cell(request).unwrap();
        }

        let change_history = database_service.get_change_history(None).unwrap();

        assert_eq!(change_history.len(), 3);
        assert_eq!(
            change_history[0].cell_edit_request.new_value,
            Some("Third Update".to_string())
        );
        assert_eq!(
            change_history[1].cell_edit_request.new_value,
            Some("Second Update".to_string())
        );
        assert_eq!(
            change_history[2].cell_edit_request.new_value,
            Some("First Update".to_string())
        );
    }

    #[test]
    fn test_get_change_history_filters_by_table_name() {
        let database_service = in_memory_service_with_notes_row();
        let request = CellEditRequest::new(
            "notes".to_string(),
            "id".to_string(),
            "1".to_string(),
            "title".to_string(),
            Some("Updated Title".to_string()),
            Some("Original Title".to_string()),
        );
        database_service.update_cell(&request).unwrap();

        let all_history = database_service.get_change_history(None).unwrap();
        let notes_history = database_service
            .get_change_history(Some("notes".to_string()))
            .unwrap();
        let other_history = database_service
            .get_change_history(Some("items".to_string()))
            .unwrap();

        assert_eq!(all_history.len(), 1);
        assert_eq!(notes_history.len(), 1);
        assert_eq!(notes_history[0].cell_edit_request.table_name, "notes");
        assert!(other_history.is_empty());
    }
}
