use rusqlite::Connection;
use crate::errors::AppError;

pub fn ping_sqlite()-> Result<String, AppError>{
    let conn = Connection::open_in_memory().map_err(|e|AppError::Message(e.to_string()))?;
        let result: i32 = conn.query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|e|AppError::Message(e.to_string()))?;
        if result != 1 {
        return Err(AppError::Message("SQLite ping failed".to_string()));
    }
    Ok(result.to_string())
}