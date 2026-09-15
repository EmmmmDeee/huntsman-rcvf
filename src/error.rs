//! Library errors for session storage, config, and RCVF rule checks.

use thiserror::Error;

/// Recoverable huntsman failures. Never used to mean success.
#[derive(Debug, Error)]
pub enum Error {
    /// Named session is not in the store.
    #[error("session not found: {id}")]
    SessionNotFound { id: String },
    /// No `--session` and no `var/current.txt`.
    #[error("no current session; run huntsman new")]
    NoCurrentSession,
    /// Config file or environment rejected.
    #[error("config: {0}")]
    Config(String),
    /// Filesystem failure.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Session JSON rejected.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Session file larger than the configured cap.
    #[error("session file exceeds size cap ({max} bytes)")]
    SessionTooLarge { max: u64 },
    /// Session id contains characters other than `[A-Za-z0-9._-]`.
    #[error("invalid session id: {0}")]
    InvalidSessionId(String),
    /// `terminate` called without required records and without `--partial`.
    #[error("terminate refused: {0}")]
    TerminateRefused(String),
    /// A required text field is empty.
    #[error("missing required field: {0}")]
    MissingField(String),
    /// Neither `HUNTSMAN_ROOT` nor `HOME` is usable.
    #[error("home directory is unset")]
    HomeUnset,
}

impl Error {
    /// True when the operator can retry with different input.
    #[must_use]
    pub const fn is_operator_error(&self) -> bool {
        matches!(
            self,
            Self::SessionNotFound { .. }
                | Self::NoCurrentSession
                | Self::InvalidSessionId(_)
                | Self::TerminateRefused(_)
                | Self::MissingField(_)
                | Self::Config(_)
        )
    }
}
