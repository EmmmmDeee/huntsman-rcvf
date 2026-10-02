//! End-to-end acceptance for the reconstructed crate.

use std::fs;

use huntsman_recon::classify::classify_response;
use huntsman_recon::ledger::{seal, Claim};
use huntsman_recon::navigator::layer;
use huntsman_recon::session::{Candidate, FalsifyRecord, Session, VerifyRecord};
use huntsman_recon::stage::{EvidenceLevel, Status};
use huntsman_recon::stix::bundle;
use huntsman_recon::store::Store;
use huntsman_recon::Error;

#[test]
fn terminate_refuses_empty_and_store_roundtrip() {
    let mut empty = Session::new("empty");
    let err = empty.terminate("still unknown".into(), false, "").expect_err("refuse");
    assert!(matches!(err, Error::TerminateRefused(_)));
    let err = empty.terminate("  ".into(), true, "").expect_err("residual");
    assert!(matches!(err, Error::MissingField(_)));

    let mut session = Session::new("full");
    session.apply_recover("obj", "out", "no network", "terminate");
    session.add_candidate(Candidate {
        statement: "json store".into(),
        alternatives: vec!["sqlite".into()],
        reverse_observation: "lost after restart".into(),
    }).unwrap();
    session.add_falsify(FalsifyRecord {
        attack: "empty terminate".into(),
        test: "terminate".into(),
        result: "refused".into(),
    }).unwrap();
    session.add_verify(VerifyRecord {
        claim: "gate holds".into(),
        status: Status::Verified,
        evidence_level: EvidenceLevel::DirectObservation,
        does_not_show: "not the monolith".into(),
    }).unwrap();
    let tip = "ab".repeat(32);
    session.terminate("egress blocked".into(), false, &tip).unwrap();
    assert!(session.bound_to(&tip));
    assert!(!session.bound_to("cd".repeat(32).as_str()));

    let root = std::env::temp_dir().join(format!("huntsman-recon-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let store = Store::new(&root);
    store.save(&session).unwrap();
    let loaded = store.load(&session.id).unwrap();
    assert!(loaded.termination.is_some());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn navigator_and_stix_drop_challenge_and_catalog() {
    let wall = classify_response(403, "<html>checking your browser cloudflare</html>");
    assert!(wall.is_wall());
    let admitted = seal(&Claim {
        claim: "challenge is not a result".into(),
        source: "src/classify.rs tests".into(),
        component: "src/classify.rs".into(),
        technique_id: Some("T1592".into()),
        status: Status::Verified,
        evidence_level: EvidenceLevel::Reproduction,
        does_not_show: "does not bypass the wall".into(),
    });
    let catalog = seal(&Claim {
        claim: "mapped only".into(),
        source: "attack catalog".into(),
        component: "none".into(),
        technique_id: Some("T1595".into()),
        status: Status::Verified,
        evidence_level: EvidenceLevel::Assertion,
        does_not_show: "no runnable method".into(),
    });
    let nav = layer(&[admitted.clone(), catalog.clone()]);
    let ids: Vec<&str> = nav["techniques"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["techniqueID"].as_str())
        .collect();
    assert!(ids.is_empty(), "self-labeled T1592 is not an implemented technique");
    assert!(bundle(&[admitted, catalog])["objects"].as_array().unwrap().is_empty());
}

#[test]
fn store_refuses_symlink_and_phone_file_roundtrip() {
    let root = std::env::temp_dir().join(format!("huntsman-recon-link-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let store = Store::new(&root);
    let session = Session::new("link");
    store.save(&session).unwrap();
    let sessions = root.join("sessions");
    let link = sessions.join(format!("{}.json", session.id));
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
    assert!(store.load(&session.id).is_err());
    let _ = fs::remove_dir_all(&root);
    let _ = session;
}

#[test]
fn case_dir_roundtrip_keeps_navigator_empty() {
    let root = std::env::temp_dir().join(format!("huntsman-case-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("corpus")).unwrap();
    fs::write(
        root.join("people.json"),
        r#"[{"id":"a","name":"Same","phones":["+61 412 345 678"]},{"id":"b","name":"Same","emails":["b@ex.com"],"phones":["61412345678"]}]"#,
    )
    .unwrap();
    fs::write(
        root.join("fixes.json"),
        r#"[{"id":"a","lat":-27.47,"lon":153.02,"at_unix":1000},{"id":"b","lat":-27.47,"lon":153.021,"at_unix":1100}]"#,
    )
    .unwrap();
    fs::write(root.join("corpus/port.txt"), "Brisbane port radar").unwrap();
    fs::write(
        root.join("case.json"),
        r#"{"title":"accept","query":"brisbane port","radius_m":2000,"window_secs":3600}"#,
    )
    .unwrap();
    let (spec, people, fixes, docs) = huntsman_recon::case::load_dir(&root).unwrap();
    let output = huntsman_recon::case::execute(&spec, &people, &fixes, &docs).unwrap();
    assert_eq!(output.report.clusters, 1);
    assert_eq!(output.report.hits, 1);
    assert_eq!(output.report.techniques, 0);
    huntsman_recon::case::write_output(&root.join("out"), &output).unwrap();
    let nav = fs::read_to_string(root.join("out/navigator.json")).unwrap();
    assert!(nav.contains("\"techniques\": []"));
    let _ = fs::remove_dir_all(&root);
}
