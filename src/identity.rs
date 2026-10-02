//! People-centric identity on operator-supplied records.
//! A shared display name is not a link. Email and handle are.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonRecord {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub handles: Vec<String>,
    /// Digits only after canonicalization. National and international forms do not merge.
    #[serde(default)]
    pub phones: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub left: String,
    pub right: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cluster {
    pub members: Vec<String>,
    pub links: Vec<Link>,
}

#[must_use]
pub fn canonical_name(raw: &str) -> String {
    raw.split_whitespace()
        .map(|w| w.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn canonical_email(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_ascii_lowercase();
    let (local, domain) = trimmed.split_once('@')?;
    if local.is_empty() || domain.is_empty() || domain.contains('@') || !domain.contains('.') {
        return None;
    }
    if !local.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-')) {
        return None;
    }
    Some(format!("{local}@{domain}"))
}

#[must_use]
pub fn canonical_handle(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('@').to_ascii_lowercase();
    if trimmed.is_empty() || trimmed.len() > 40 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(trimmed)
}

/// Separators stripped. 8 to 15 digits. No country inference. A leading zero is kept.
#[must_use]
pub fn canonical_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if (8..=15).contains(&digits.len()) {
        Some(digits)
    } else {
        None
    }
}

/// Union by shared canonical email, handle, or phone. Names never merge records.
/// Plus-tags and dotted local-parts stay distinct.
#[must_use]
pub fn resolve(records: &[PersonRecord]) -> Vec<Cluster> {
    let n = records.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut links = Vec::new();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    fn unite(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if shared_email(&records[i], &records[j]) {
                unite(&mut parent, i, j);
                links.push(Link {
                    left: records[i].id.clone(),
                    right: records[j].id.clone(),
                    reason: "shared_email".to_owned(),
                });
            } else if shared_handle(&records[i], &records[j]) {
                unite(&mut parent, i, j);
                links.push(Link {
                    left: records[i].id.clone(),
                    right: records[j].id.clone(),
                    reason: "shared_handle".to_owned(),
                });
            } else if shared_phone(&records[i], &records[j]) {
                unite(&mut parent, i, j);
                links.push(Link {
                    left: records[i].id.clone(),
                    right: records[j].id.clone(),
                    reason: "shared_phone".to_owned(),
                });
            }
        }
    }
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        buckets[find(&mut parent, i)].push(i);
    }
    buckets
        .into_iter()
        .filter(|b| !b.is_empty())
        .map(|idxs| {
            let members: Vec<String> = idxs.iter().map(|i| records[*i].id.clone()).collect();
            let cluster_links = links
                .iter()
                .filter(|l| members.contains(&l.left) && members.contains(&l.right))
                .cloned()
                .collect();
            Cluster {
                members,
                links: cluster_links,
            }
        })
        .collect()
}

fn shared_email(a: &PersonRecord, b: &PersonRecord) -> bool {
    let left: Vec<String> = a.emails.iter().filter_map(|e| canonical_email(e)).collect();
    b.emails.iter().filter_map(|e| canonical_email(e)).any(|e| left.contains(&e))
}

fn shared_handle(a: &PersonRecord, b: &PersonRecord) -> bool {
    let left: Vec<String> = a.handles.iter().filter_map(|h| canonical_handle(h)).collect();
    b.handles.iter().filter_map(|h| canonical_handle(h)).any(|h| left.contains(&h))
}

fn shared_phone(a: &PersonRecord, b: &PersonRecord) -> bool {
    let left: Vec<String> = a.phones.iter().filter_map(|p| canonical_phone(p)).collect();
    b.phones.iter().filter_map(|p| canonical_phone(p)).any(|p| left.contains(&p))
}

pub fn load_people(path: &std::path::Path) -> Result<Vec<PersonRecord>, crate::error::Error> {
    let body = std::fs::read(path).map_err(|e| crate::error::Error::Store(e.to_string()))?;
    if body.len() > 1_048_576 {
        return Err(crate::error::Error::Store("people file exceeds 1 MiB".into()));
    }
    serde_json::from_slice(&body).map_err(|e| crate::error::Error::Store(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_without_link_stays_split() {
        let records = vec![
            PersonRecord {
                id: "a".into(),
                name: "Jane Doe".into(),
                emails: vec!["a@example.com".into()],
                handles: vec![],
                phones: vec![],
            },
            PersonRecord {
                id: "b".into(),
                name: "jane doe".into(),
                emails: vec!["b@example.com".into()],
                handles: vec![],
                phones: vec![],
            },
        ];
        let clusters = resolve(&records);
        assert_eq!(clusters.len(), 2);
        assert!(clusters.iter().all(|c| c.links.is_empty()));
    }

    #[test]
    fn shared_email_merges_case_and_at_handle() {
        let records = vec![
            PersonRecord {
                id: "a".into(),
                name: "Ada".into(),
                emails: vec!["Ada@Example.com".into()],
                handles: vec![],
                phones: vec![],
            },
            PersonRecord {
                id: "b".into(),
                name: "Other".into(),
                emails: vec!["ada@example.com".into()],
                handles: vec!["@Ada".into()],
                phones: vec![],
            },
        ];
        let clusters = resolve(&records);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].links[0].reason, "shared_email");
        assert_eq!(canonical_handle("@Ada"), Some("ada".into()));
    }

    #[test]
    fn phone_merges_formatting_only_and_namesake_stays_split() {
        let records = vec![
            PersonRecord {
                id: "a".into(),
                name: "Same".into(),
                emails: vec![],
                handles: vec![],
                phones: vec!["+61 412 345 678".into()],
            },
            PersonRecord {
                id: "b".into(),
                name: "Same".into(),
                emails: vec![],
                handles: vec![],
                phones: vec!["61412345678".into()],
            },
            PersonRecord {
                id: "c".into(),
                name: "Same".into(),
                emails: vec!["c@ex.com".into()],
                handles: vec![],
                phones: vec!["0412345678".into()],
            },
        ];
        let clusters = resolve(&records);
        assert_eq!(clusters.len(), 2);
        let merged = clusters.iter().find(|c| c.members.len() == 2).unwrap();
        assert_eq!(merged.links[0].reason, "shared_phone");
        assert!(clusters.iter().any(|c| c.members == vec!["c".to_owned()]));
        assert_eq!(canonical_phone("123"), None);
        assert_eq!(canonical_phone("user+tag@ex.com"), None);
    }

    #[test]
    fn plus_tag_and_dotted_local_do_not_merge() {
        let records = vec![
            PersonRecord {
                id: "a".into(),
                name: "Ada".into(),
                emails: vec!["ada@gmail.com".into()],
                handles: vec![],
                phones: vec![],
            },
            PersonRecord {
                id: "b".into(),
                name: "Ada".into(),
                emails: vec!["ada+news@gmail.com".into()],
                handles: vec![],
                phones: vec![],
            },
            PersonRecord {
                id: "c".into(),
                name: "Ada".into(),
                emails: vec!["a.da@gmail.com".into()],
                handles: vec![],
                phones: vec![],
            },
        ];
        assert_eq!(resolve(&records).len(), 3);
    }
}
