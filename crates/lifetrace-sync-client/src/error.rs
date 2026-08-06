use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureClass {
    AuthRequired,
    PermissionDenied,
    RateLimited { retry_after_seconds: Option<u64> },
    PayloadTooLarge,
    UpgradeRequired,
    Permanent,
    Offline,
    Transient,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{code}: {message}")]
pub struct SyncError {
    pub code: String,
    pub message: String,
    pub class: FailureClass,
}

impl SyncError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, class: FailureClass) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            class,
        }
    }

    pub fn transient(message: impl Into<String>) -> Self {
        Self::new("SYNC_TRANSIENT", message, FailureClass::Transient)
    }

    pub fn offline(message: impl Into<String>) -> Self {
        Self::new("SYNC_OFFLINE", message, FailureClass::Offline)
    }
}
