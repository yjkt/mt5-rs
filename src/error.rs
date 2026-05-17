use thiserror::Error;

#[derive(Error, Debug)]
pub enum Mt5Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("MT5 not initialized")]
    NotInitialized,

    #[error("Command failed: cmd={cmd}, error={error}")]
    CommandFailed { cmd: u32, error: String },

    #[error("Not supported: {0}")]
    NotSupported(String),
}

pub type Result<T> = std::result::Result<T, Mt5Error>;
