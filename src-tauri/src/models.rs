use serde::{Serialize, Deserialize};
use std::fmt::{self, Display};
use rusqlite::Row;


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
    pub fn new(id: i32, title: String, content: String) -> Self {
        Self { id, title, content }
    }

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
        write!(f, "Note {{ id: {}, title: {}, content: {} }}", self.id, self.title, self.content)
    }
}