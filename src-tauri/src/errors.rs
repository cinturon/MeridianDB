use serde::{Serialize, Deserialize};
use std::fmt::{self, Display};

#[derive(Debug, Serialize, Deserialize)]
pub enum AppError {
    Message(String),
    NotImplemented(String)
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Message(message) => write!(f, "{}", message),
            AppError::NotImplemented(message) => write!(f, "{}", message),
        }
    }
}