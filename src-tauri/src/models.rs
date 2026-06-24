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
        let primary_key_flag: i32  = row.get(5)?;

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
    pub fn new(
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        duration_ms: Option<u64>,
    ) -> Self {
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