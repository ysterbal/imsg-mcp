use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ImsgError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Message not found")]
    MessageNotFound,

    #[error("Contact access denied")]
    ContactAccessDenied,

    #[error("Not supported on this platform")]
    PlatformNotSupported,

    #[error("Invalid snowflake ID: {0}")]
    InvalidSnowflake(String),
}

pub type Result<T> = std::result::Result<T, ImsgError>;
