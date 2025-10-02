use thiserror::Error;

pub type Result<T> = std::result::Result<T, AetherError>;

#[derive(Error, Debug)]
pub enum AetherError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Dialog error: {0}")]
    Dialog(#[from] dialoguer::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Deployment error: {0}")]
    Deployment(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("AWS/S3 error: {0}")]
    Aws(#[from] anyhow::Error),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid project: {0}")]
    InvalidProject(String),

    #[error("{0}")]
    Other(String),
}

impl AetherError {
    pub fn config<S: Into<String>>(msg: S) -> Self {
        AetherError::Config(msg.into())
    }

    pub fn build<S: Into<String>>(msg: S) -> Self {
        AetherError::Build(msg.into())
    }

    pub fn deployment<S: Into<String>>(msg: S) -> Self {
        AetherError::Deployment(msg.into())
    }

    pub fn auth<S: Into<String>>(msg: S) -> Self {
        AetherError::Auth(msg.into())
    }

    pub fn invalid_project<S: Into<String>>(msg: S) -> Self {
        AetherError::InvalidProject(msg.into())
    }
}
