use std::fmt;

/// Custom error types for the registry
#[derive(Debug)]
pub enum RegistryError {
    Database(sqlx::Error),
    NotFound(String),
    InvalidInput(String),
    VerificationFailed(String),
    StellarRpc(String),
    Internal(String),
    S3(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Database(e) => write!(f, "Database error: {}", e),
            RegistryError::NotFound(msg) => write!(f, "Not found: {}", msg),
            RegistryError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            RegistryError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            RegistryError::StellarRpc(msg) => write!(f, "Stellar RPC error: {}", msg),
            RegistryError::Internal(msg) => write!(f, "Internal error: {}", msg),
            RegistryError::S3(msg) => write!(f, "S3 error: {}", msg),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<sqlx::Error> for RegistryError {
    fn from(err: sqlx::Error) -> Self {
        RegistryError::Database(err)
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> Self {
        RegistryError::Internal(format!("JSON error: {}", err))
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(err: std::io::Error) -> Self {
        RegistryError::Internal(format!("IO error: {}", err))
    }
}

impl From<anyhow::Error> for RegistryError {
    fn from(err: anyhow::Error) -> Self {
        RegistryError::Internal(format!("{}", err))
    }
}

impl From<s3::error::S3Error> for RegistryError {
    fn from(err: s3::error::S3Error) -> Self {
        RegistryError::S3(format!("{}", err))
    }
}

impl From<s3::creds::error::CredentialsError> for RegistryError {
    fn from(err: s3::creds::error::CredentialsError) -> Self {
        RegistryError::S3(format!("Credentials error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, RegistryError>;
