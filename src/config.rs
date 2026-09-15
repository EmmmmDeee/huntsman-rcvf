//! Sole reader of environment variables and optional config.toml.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;

/// Default cap on a session document, in bytes.
pub const DEFAULT_MAX_SESSION_BYTES: u64 = 1_048_576;

/// Runtime paths and limits. Built only through [`Config::load`] or
/// [`Config::from_root`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    root: PathBuf,
    max_session_bytes: u64,
}

#[derive(Debug, Default)]
struct FileConfig {
    root: Option<PathBuf>,
    max_session_bytes: Option<u64>,
}

impl Config {
    /// Load from `HUNTSMAN_*`, optional TOML, then `$HOME/huntsman`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::HomeUnset`] when no root can be resolved,
    /// [`Error::Config`] when TOML is present but invalid or oversize.
    pub fn load() -> Result<Self, Error> {
        let config_path = env_path("HUNTSMAN_CONFIG").unwrap_or_else(|| {
            default_root_from_home()
                .map(|root| root.join("config.toml"))
                .unwrap_or_else(|| PathBuf::from("config.toml"))
        });

        let file = read_optional_toml(&config_path)?;

        let root = if let Some(root) = env_path("HUNTSMAN_ROOT") {
            root
        } else if let Some(root) = file.root {
            root
        } else {
            default_root_from_home().ok_or(Error::HomeUnset)?
        };

        let max_session_bytes = env_u64("HUNTSMAN_MAX_SESSION_BYTES")?
            .or(file.max_session_bytes)
            .unwrap_or(DEFAULT_MAX_SESSION_BYTES);

        if max_session_bytes == 0 {
            return Err(Error::Config(
                "max_session_bytes must be greater than zero".into(),
            ));
        }

        Ok(Self {
            root,
            max_session_bytes,
        })
    }

    /// Build a config rooted at `root` without reading the process environment.
    /// Used by tests.
    #[must_use]
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_session_bytes: DEFAULT_MAX_SESSION_BYTES,
        }
    }

    /// Install / data root (`$HOME/huntsman` by default).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `root/var`.
    #[must_use]
    pub fn var_dir(&self) -> PathBuf {
        self.root.join("var")
    }

    /// `root/var/sessions`.
    #[must_use]
    pub fn sessions_dir(&self) -> PathBuf {
        self.var_dir().join("sessions")
    }

    /// Pointer file holding the current session id.
    #[must_use]
    pub fn current_pointer(&self) -> PathBuf {
        self.var_dir().join("current.txt")
    }

    /// Hard cap on session file size.
    #[must_use]
    pub const fn max_session_bytes(&self) -> u64 {
        self.max_session_bytes
    }
}

fn default_root_from_home() -> Option<PathBuf> {
    env_path("HOME").map(|home| home.join("huntsman"))
}

fn env_path(key: &str) -> Option<PathBuf> {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => Some(PathBuf::from(value)),
        _ => None,
    }
}

fn env_u64(key: &str) -> Result<Option<u64>, Error> {
    match std::env::var(key) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| Error::Config(format!("{key} must be a positive integer"))),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(Error::Config(format!("{key}: {err}"))),
    }
}

fn read_optional_toml(path: &Path) -> Result<FileConfig, Error> {
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let meta = fs::metadata(path)?;
    if meta.len() > 64 * 1024 {
        return Err(Error::Config(format!(
            "{} exceeds 64KiB cap",
            path.display()
        )));
    }
    let text = fs::read_to_string(path)?;
    parse_file_config(&text, path)
}

/// Minimal TOML subset: `key = "string"` and `key = integer`, `#` comments.
fn parse_file_config(text: &str, path: &Path) -> Result<FileConfig, Error> {
    let mut out = FileConfig::default();
    for (lineno, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(Error::Config(format!(
                "{}:{}: expected key = value",
                path.display(),
                lineno + 1
            )));
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "root" => out.root = Some(PathBuf::from(unquote(value, path, lineno + 1)?)),
            "max_session_bytes" => {
                let n = value.parse::<u64>().map_err(|_| {
                    Error::Config(format!(
                        "{}:{}: max_session_bytes must be an integer",
                        path.display(),
                        lineno + 1
                    ))
                })?;
                out.max_session_bytes = Some(n);
            }
            other => {
                return Err(Error::Config(format!(
                    "{}:{}: unknown key {other}",
                    path.display(),
                    lineno + 1
                )));
            }
        }
    }
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (idx, ch) in line.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

fn unquote(value: &str, path: &Path, line: usize) -> Result<String, Error> {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return Ok(value[1..value.len() - 1].to_owned());
    }
    if value.starts_with('"') {
        return Err(Error::Config(format!(
            "{}:{line}: unterminated string",
            path.display()
        )));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_file_config;
    use std::path::Path;

    #[test]
    fn parses_subset() {
        let cfg = parse_file_config(
            "# comment\nroot = \"/tmp/h\"\nmax_session_bytes = 12\n",
            Path::new("t.toml"),
        )
        .expect("parse");
        assert_eq!(cfg.root.as_deref(), Some(Path::new("/tmp/h")));
        assert_eq!(cfg.max_session_bytes, Some(12));
    }

    #[test]
    fn rejects_unknown_key() {
        let err = parse_file_config("nope = 1\n", Path::new("t.toml")).unwrap_err();
        assert!(err.to_string().contains("unknown key"));
    }
}
