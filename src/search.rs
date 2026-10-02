//! Local search. Operator-supplied documents only.
//! A blocked or throttled fetch is not a hit. No paid source. No live client.

use crate::classify::{classify_response, FetchOutcome};
use crate::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: String,
    pub body: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub id: String,
    pub score: u32,
    pub source: String,
}

#[must_use]
pub fn tokenize(raw: &str) -> Vec<String> {
    raw.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

#[must_use]
pub fn search(docs: &[Document], query: &str) -> Vec<Hit> {
    let terms = tokenize(query);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for doc in docs {
        let tokens = tokenize(&doc.body);
        let mut score = 0u32;
        let mut matched = 0u32;
        for term in &terms {
            let count = tokens.iter().filter(|t| *t == term).count() as u32;
            if count > 0 {
                matched += 1;
                score += count;
            }
        }
        if matched == terms.len() as u32 {
            hits.push(Hit {
                id: doc.id.clone(),
                score,
                source: doc.source.clone(),
            });
        }
    }
    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    hits
}

/// Regular files only. Symlinks are skipped. Extensions: txt, md, json.
/// No document cap. A symlink is still not a document.
pub fn load_corpus(dir: &Path) -> Result<Vec<Document>, Error> {
    let entries = fs::read_dir(dir).map_err(|e| Error::Store(e.to_string()))?;
    let mut docs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| Error::Store(e.to_string()))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| Error::Store(e.to_string()))?;
        if meta.file_type().is_symlink() || !meta.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let ext = name.rsplit('.').next().unwrap_or("");
        if !matches!(ext, "txt" | "md" | "json") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|e| Error::Store(e.to_string()))?;
        docs.push(Document {
            id: name,
            body,
            source: path.display().to_string(),
        });
    }
    docs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(docs)
}

/// A fetch becomes hits only when the classifier says the body is the payload.
#[must_use]
pub fn search_response(status: u16, body: &str, query: &str, source: &str) -> Vec<Hit> {
    if !matches!(classify_response(status, body), FetchOutcome::Parsed) {
        return Vec::new();
    }
    search(
        &[Document { id: source.to_owned(), body: body.to_owned(), source: source.to_owned() }],
        query,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs() -> Vec<Document> {
        vec![
            Document { id: "a".into(), body: "Brisbane radar sighting at the port".into(), source: "local".into() },
            Document { id: "b".into(), body: "Sydney harbour note".into(), source: "local".into() },
            Document { id: "c".into(), body: "Brisbane port schedule port".into(), source: "local".into() },
        ]
    }

    #[test]
    fn all_terms_required_and_blocked_page_is_not_a_hit() {
        let hits = search(&docs(), "brisbane port");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "c");
        assert!(search(&docs(), "").is_empty());
        assert!(search_response(200, "<html>just a moment cloudflare</html>", "brisbane", "remote").is_empty());
        assert!(search_response(429, "brisbane port", "brisbane", "remote").is_empty());
        let parsed = search_response(200, "brisbane port open", "brisbane port", "remote");
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn corpus_requires_every_term_and_skips_symlink() {
        let root = std::env::temp_dir().join(format!("huntsman-corpus-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("port.txt"), "Brisbane port radar").unwrap();
        fs::write(root.join("note.md"), "Sydney harbour note").unwrap();
        fs::write(root.join("skip.bin"), "brisbane port").unwrap();
        let _ = std::os::unix::fs::symlink("/etc/passwd", root.join("secret.txt"));
        let docs = load_corpus(&root).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(search(&docs, "brisbane port").len(), 1);
        assert!(search(&docs, "brisbane harbour").is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
