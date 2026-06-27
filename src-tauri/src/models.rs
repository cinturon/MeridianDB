use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct Note {
    pub id: i32,
    pub title: String,
    pub content: String,
}

impl Note {
    //Maps a notes query row into a Note struct by column index.
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
        })
    }
}

impl Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Note {{ id: {}, title: {}, content: {} }}",
            self.id, self.title, self.content
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub sqlite_available: bool,
    pub sample_query_passed: bool,
    pub message: Option<String>,
}

impl DatabaseHealth {
    pub fn new(sqlite_available: bool, sample_query_passed: bool, message: Option<String>) -> Self {
        Self {
            sqlite_available,
            sample_query_passed,
            message,
        }
    }
}

impl Display for DatabaseHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DatabaseHealth {{ sqlite_available: {}, sample_query_passed: {}, message: {:?} }}",
            self.sqlite_available, self.sample_query_passed, self.message
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub table_type: String,
}

impl TableInfo {
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            name: row.get(0)?,
            table_type: row.get(1)?,
        })
    }
}

impl Display for TableInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TableInfo {{ name: {}, table_type: {} }}",
            self.name, self.table_type
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    pub data_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
}

impl ColumnInfo {
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let not_null_flag: i32 = row.get(3)?;
        let primary_key_flag: i32 = row.get(5)?;

        Ok(Self {
            cid: row.get(0)?,
            name: row.get(1)?,
            data_type: row.get(2)?,
            not_null: not_null_flag != 0,
            default_value: row.get(4)?,
            primary_key: primary_key_flag != 0,
        })
    }
}

impl Display for ColumnInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ColumnInfo {{ cid: {}, name: {}, data_type: {}, not_null: {}, default_value: {:?}, primary_key: {} }}", self.cid, self.name, self.data_type, self.not_null, self.default_value, self.primary_key)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TablePreview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub limit: i64,
}

impl TablePreview {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>, limit: i64) -> Self {
        Self {
            columns,
            rows,
            limit,
        }
    }
}

impl Default for TablePreview {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            limit: default_limit(),
        }
    }
}

fn default_limit() -> i64 {
    25
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count: usize,
    pub duration_ms: Option<u64>,
}

impl QueryResult {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>, duration_ms: Option<u64>) -> Self {
        let row_count = rows.len();
        Self {
            columns,
            rows,
            row_count,
            duration_ms,
        }
    }
}

impl Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueryResult {{ columns: {:?}, row_count: {}, duration_ms: {:?} }}",
            self.columns, self.row_count, self.duration_ms
        )
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CellEditRequest {
    pub table_name: String,
    pub primary_key_column: String,
    pub primary_key_value: String,
    pub target_column: String,
    pub new_value: Option<String>,
    pub original_value: Option<String>,
}

impl CellEditRequest {
    pub fn new(
        table_name: String,
        primary_key_column: String,
        primary_key_value: String,
        target_column: String,
        new_value: Option<String>,
        original_value: Option<String>,
    ) -> Self {
        Self {
            table_name,
            primary_key_column,
            primary_key_value,
            target_column,
            new_value,
            original_value,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CellEditResult {
    pub rows_updated: i64,
}

impl CellEditResult {
    pub fn new(rows_updated: i64) -> Self {
        Self { rows_updated }
    }
}

impl Display for CellEditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CellEditResult {{ rows_updated: {} }}",
            self.rows_updated
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ChangeHistoryEntry {
    pub timestamp: String,
    pub cell_edit_request: CellEditRequest,
}

impl ChangeHistoryEntry {
    pub fn new(timestamp: String, cell_edit_request: CellEditRequest) -> Self {
        Self {
            timestamp,
            cell_edit_request,
        }
    }
}

impl ChangeHistoryEntry {
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let new_value: String = row.get(5)?;
        let original_value: String = row.get(6)?;

        let cell_edit_request = CellEditRequest::new(
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            optional_stored_value(new_value),
            optional_stored_value(original_value),
        );

        Ok(Self {
            timestamp: row.get(0)?,
            cell_edit_request,
        })
    }
}

fn optional_stored_value(value: String) -> Option<String> {
    if value == "NULL" {
        None
    } else {
        Some(value)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct UndoPreview {
    pub history_entry_id: i64,
    pub table_name: String,
    pub primary_key_column: String,
    pub primary_key_value: String,
    pub target_column: String,
    pub current_value: String,
    pub restored_value: String,
    pub is_safe_to_undo: bool,
    pub warning_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::UndoPreview;

    #[test]
    fn test_undo_preview_safe_construction() {
        let preview = UndoPreview {
            history_entry_id: 1,
            table_name: "employees".into(),
            primary_key_column: "id".into(),
            primary_key_value: "3".into(),
            target_column: "name".into(),
            current_value: "Ada Lovelace".into(),
            restored_value: "Ada".into(),
            is_safe_to_undo: true,
            warning_message: None,
        };

        assert!(preview.is_safe_to_undo);
        assert_eq!(preview.restored_value, "Ada");
        assert!(preview.warning_message.is_none());
    }

    #[test]
    fn test_undo_preview_unsafe_construction_with_warning() {
        let preview = UndoPreview {
            history_entry_id: 2,
            table_name: "employees".into(),
            primary_key_column: "id".into(),
            primary_key_value: "3".into(),
            target_column: "name".into(),
            current_value: "Someone Else".into(),
            restored_value: "Ada".into(),
            is_safe_to_undo: false,
            warning_message: Some(
                "Cell value changed since this history entry was recorded.".into(),
            ),
        };

        assert!(!preview.is_safe_to_undo);
        assert_eq!(preview.restored_value, "Ada");
        assert!(preview.warning_message.is_some());
    }
}
