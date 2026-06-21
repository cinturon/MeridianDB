use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}