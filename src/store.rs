//! JSON session store. No size cap. No symlinks. Id must be a safe stem.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::session::{valid_session_id, Session};

pub struct Store {
    root: PathBuf,
}

impl Store {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn save(&self, session: &Session) -> Result<PathBuf, Error> {
        if !valid_session_id(&session.id) {
            return Err(Error::Invalid(format!("unsafe session id: {}", session.id)));
        }
        let dir = self.root.join("sessions");
        fs::create_dir_all(&dir).map_err(|e| Error::Store(e.to_string()))?;
        let path = dir.join(format!("{}.json", session.id));
        if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            return Err(Error::Invalid("refusing symlink session path".into()));
        }
        let body = serde_json::to_vec_pretty(session).map_err(|e| Error::Store(e.to_string()))?;
        fs::write(&path, &body).map_err(|e| Error::Store(e.to_string()))?;
        fs::write(self.root.join("current.txt"), session.id.as_bytes())
            .map_err(|e| Error::Store(e.to_string()))?;
        Ok(path)
    }

    pub fn load(&self, id: &str) -> Result<Session, Error> {
        if !valid_session_id(id) {
            return Err(Error::Invalid(format!("unsafe session id: {id}")));
        }
        let path = self.root.join("sessions").join(format!("{id}.json"));
        if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            return Err(Error::Invalid("refusing symlink session path".into()));
        }
        read_bounded(&path)
    }
}

fn read_bounded(path: &Path) -> Result<Session, Error> {
    let body = fs::read(path).map_err(|e| Error::Store(e.to_string()))?;
    serde_json::from_slice(&body).map_err(|e| Error::Store(e.to_string()))
}
