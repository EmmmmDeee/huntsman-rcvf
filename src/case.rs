//! One case run. Identity, co-location, and local search share one ledger and one session.
//! No network. No technique score. Missing inputs are residual uncertainty, not empty success.

use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::geoint::{colocated, Fix};
use crate::identity::{resolve, PersonRecord};
use crate::ledger::{append, binding_count, chain_intact, seal, Claim, LedgerEntry};
use crate::navigator::layer;
use crate::search::{search, Document};
use crate::session::{Candidate, ExecuteRecord, FalsifyRecord, Session, VerifyRecord};
use crate::stage::{EvidenceLevel, Status};
use crate::stix::bundle;

#[derive(Debug, Clone)]
pub struct CaseSpec {
    pub title: String,
    pub objective: String,
    pub constraints: String,
    pub query: String,
    pub radius_m: f64,
    pub window_secs: i64,
}

impl Default for CaseSpec {
    fn default() -> Self {
        Self {
            title: "case".into(),
            objective: "compose offline identity, geoint, and retrieval".into(),
            constraints: "no network; no paid source; no technique score".into(),
            query: String::new(),
            radius_m: 2_000.0,
            window_secs: 3_600,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseReport {
    pub records: usize,
    pub clusters: usize,
    pub links: usize,
    pub pairs: usize,
    pub hits: usize,
    pub techniques: usize,
    pub bindings: usize,
    pub tip: String,
    pub residual: String,
}

pub struct CaseOutput {
    pub session: Session,
    pub entries: Vec<LedgerEntry>,
    pub navigator: Value,
    pub stix: Value,
    pub report: CaseReport,
}

pub fn execute(
    spec: &CaseSpec,
    people: &[PersonRecord],
    fixes: &[Fix],
    docs: &[Document],
) -> Result<CaseOutput, Error> {
    if binding_count() != 0 {
        return Err(Error::Invalid("binding table must stay empty until a method implements a technique".into()));
    }
    let clusters = resolve(people);
    let links = clusters.iter().map(|c| c.links.len()).sum::<usize>();
    let pairs = colocated(fixes, spec.radius_m, spec.window_secs);
    let hits = if spec.query.trim().is_empty() {
        Vec::new()
    } else {
        search(docs, &spec.query)
    };

    let identity = claim(
        format!(
            "{} records resolved to {} clusters with {} links",
            people.len(),
            clusters.len(),
            links
        ),
        "operator people file",
        "src/identity.rs",
        "a shared name is not a link; plus-tags and national phone forms do not merge",
    );
    let geo = append(
        &identity.hash,
        &claim(
            format!(
                "{} pairs inside {:.0} m and {} s",
                pairs.len(),
                spec.radius_m,
                spec.window_secs
            ),
            "operator fixes file",
            "src/geoint.rs",
            "not T1591 and not a survey",
        )
        .claim,
    );
    let found = append(
        &geo.hash,
        &claim(
            format!("{} hits for query {:?}", hits.len(), spec.query),
            "operator corpus",
            "src/search.rs",
            "a challenge response is not a hit; a local file is not a live harvest",
        )
        .claim,
    );
    let closed = append(
        &found.hash,
        &claim(
            format!("bindings={}", binding_count()),
            "src/ledger.rs",
            "src/ledger.rs",
            "catalog presence is not a score",
        )
        .claim,
    );
    let entries = vec![identity, geo, found, closed];
    if !chain_intact(&entries) {
        return Err(Error::Invalid("case chain broken".into()));
    }
    let navigator = layer(&entries);
    let stix = bundle(&entries);
    let techniques = navigator["techniques"].as_array().map(Vec::len).unwrap_or(0);
    if techniques != 0 || !stix["objects"].as_array().is_none_or(Vec::is_empty) {
        return Err(Error::Invalid("case emitted a technique without a binding".into()));
    }

    let mut residual = Vec::new();
    if people.is_empty() {
        residual.push("no people");
    }
    if fixes.is_empty() {
        residual.push("no fixes");
    }
    if docs.is_empty() {
        residual.push("no corpus");
    }
    if spec.query.trim().is_empty() {
        residual.push("no query");
    }
    residual.push("no handset run");
    residual.push("no live harvest");
    let residual = residual.join("; ");

    let mut session = Session::new(spec.title.clone());
    session.apply_recover(
        &spec.objective,
        "session bound to ledger tip; navigator techniques 0",
        &spec.constraints,
        "chain intact and interop gate closed",
    );
    session.add_candidate(Candidate {
        statement: "one case run over operator files".into(),
        alternatives: vec!["separate commands".into(), "live providers".into()],
        reverse_observation: "namesake merged or technique emitted".into(),
    })?;
    session.add_falsify(FalsifyRecord {
        attack: "shared display name".into(),
        test: "resolve without email handle or phone".into(),
        result: format!("clusters={} records={}", clusters.len(), people.len()),
    })?;
    session.add_execute(ExecuteRecord {
        action: "case".into(),
        observed: format!("links={links} pairs={} hits={}", pairs.len(), hits.len()),
        component: "src/case.rs".into(),
    })?;
    session.add_verify(VerifyRecord {
        claim: "navigator stayed empty".into(),
        status: Status::Verified,
        evidence_level: EvidenceLevel::DirectObservation,
        does_not_show: "not a live collection and not an ATT&CK score".into(),
    })?;
    let tip = entries.last().map(|e| e.hash.clone()).unwrap_or_default();
    session.terminate(residual.clone(), false, &tip)?;

    Ok(CaseOutput {
        session,
        entries,
        navigator,
        stix,
        report: CaseReport {
            records: people.len(),
            clusters: clusters.len(),
            links,
            pairs: pairs.len(),
            hits: hits.len(),
            techniques,
            bindings: binding_count(),
            tip,
            residual,
        },
    })
}

pub fn load_dir(dir: &Path) -> Result<(CaseSpec, Vec<PersonRecord>, Vec<Fix>, Vec<Document>), Error> {
    let mut spec = CaseSpec::default();
    let meta = dir.join("case.json");
    if meta.is_file() {
        let body = fs::read(&meta).map_err(|e| Error::Store(e.to_string()))?;
        let value: Value = serde_json::from_slice(&body).map_err(|e| Error::Store(e.to_string()))?;
        if let Some(title) = value.get("title").and_then(Value::as_str) {
            spec.title = title.to_owned();
        }
        if let Some(objective) = value.get("objective").and_then(Value::as_str) {
            spec.objective = objective.to_owned();
        }
        if let Some(constraints) = value.get("constraints").and_then(Value::as_str) {
            spec.constraints = constraints.to_owned();
        }
        if let Some(query) = value.get("query").and_then(Value::as_str) {
            spec.query = query.to_owned();
        }
        if let Some(radius) = value.get("radius_m").and_then(Value::as_f64) {
            spec.radius_m = radius;
        }
        if let Some(window) = value.get("window_secs").and_then(Value::as_i64) {
            spec.window_secs = window;
        }
    }
    let people = optional_json(&dir.join("people.json"))?;
    let fixes = optional_json(&dir.join("fixes.json"))?;
    let corpus = dir.join("corpus");
    let docs = if corpus.is_dir() {
        crate::search::load_corpus(&corpus)?
    } else {
        Vec::new()
    };
    Ok((spec, people, fixes, docs))
}

pub fn write_output(dir: &Path, output: &CaseOutput) -> Result<(), Error> {
    fs::create_dir_all(dir).map_err(|e| Error::Store(e.to_string()))?;
    write_json(&dir.join("ledger.json"), &output.entries)?;
    write_json(&dir.join("navigator.json"), &output.navigator)?;
    write_json(&dir.join("stix.json"), &output.stix)?;
    write_json(&dir.join("report.json"), &json!({
        "records": output.report.records,
        "clusters": output.report.clusters,
        "links": output.report.links,
        "pairs": output.report.pairs,
        "hits": output.report.hits,
        "techniques": output.report.techniques,
        "bindings": output.report.bindings,
        "tip": output.report.tip,
        "residual": output.report.residual,
    }))?;
    Ok(())
}

fn claim(text: String, source: &str, component: &str, does_not_show: &str) -> LedgerEntry {
    seal(&Claim {
        claim: text,
        source: source.to_owned(),
        component: component.to_owned(),
        technique_id: None,
        status: Status::Verified,
        evidence_level: EvidenceLevel::DirectObservation,
        does_not_show: does_not_show.to_owned(),
    })
}

fn optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>, Error> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let body = fs::read(path).map_err(|e| Error::Store(e.to_string()))?;
    serde_json::from_slice(&body).map_err(|e| Error::Store(e.to_string()))
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), Error> {
    let body = serde_json::to_vec_pretty(value).map_err(|e| Error::Store(e.to_string()))?;
    fs::write(path, body).map_err(|e| Error::Store(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_merges_phone_splits_namesake_and_emits_no_technique() {
        let people = vec![
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
        let fixes = vec![
            Fix { id: "a".into(), lat: -27.47, lon: 153.02, at_unix: 1_000 },
            Fix { id: "b".into(), lat: -27.47, lon: 153.021, at_unix: 1_100 },
            Fix { id: "c".into(), lat: -33.87, lon: 151.21, at_unix: 1_050 },
        ];
        let docs = vec![
            Document { id: "port.txt".into(), body: "Brisbane port radar".into(), source: "local".into() },
            Document { id: "note.md".into(), body: "Sydney harbour note".into(), source: "local".into() },
        ];
        let mut spec = CaseSpec::default();
        spec.query = "brisbane port".into();
        let out = execute(&spec, &people, &fixes, &docs).unwrap();
        assert_eq!(out.report.clusters, 2);
        assert_eq!(out.report.links, 1);
        assert_eq!(out.report.pairs, 1);
        assert_eq!(out.report.hits, 1);
        assert_eq!(out.report.techniques, 0);
        assert!(out.session.bound_to(&out.report.tip));
        assert!(out.report.residual.contains("no live harvest"));
    }
}
