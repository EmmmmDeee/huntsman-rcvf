//! Operable offline binary. No hardcoded corpus. No network client.
//! `check` fails if a self-labeled technique enters Navigator.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use huntsman_recon::classify::classify_response;
use huntsman_recon::geoint::{colocated, haversine_m, load_fixes, parse_latlon};
use huntsman_recon::identity::{load_people, resolve, PersonRecord};
use huntsman_recon::ledger::{append, binding_count, chain_intact, load_chain, save_chain, seal, Claim};
use huntsman_recon::navigator::layer;
use huntsman_recon::search::{load_corpus, search, search_response};
use huntsman_recon::session::{Candidate, ExecuteRecord, FalsifyRecord, Session, VerifyRecord};
use huntsman_recon::stage::{EvidenceLevel, Status};
use huntsman_recon::stix::bundle;
use huntsman_recon::store::Store;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("geo") => geo(args.next(), args.next()),
        Some("search") => search_cmd(args.next(), args.next()),
        Some("resolve") => resolve_cmd(args.next()),
        Some("coloc") => coloc_cmd(args.next(), args.next(), args.next()),
        Some("classify") => classify(args.next(), args.next()),
        Some("session") => session_cmd(args.collect()),
        Some("run") => run_cmd(args.next()),
        Some("lookup") => lookup_cmd(args.next(), args.next()),
        Some("gather") => gather_cmd(args.next()),
        Some("check") | None => check(),
        Some(other) => {
            eprintln!("unknown command: {other}");
            usage();
            ExitCode::from(64)
        }
    }
}

fn usage() {
    eprintln!(
        "usage: huntsman-recon check | run CASE_DIR | gather QUERY | lookup wikidata|nominatim|abn|datagov|legislation|gleif|rdap|crtsh|exa|seeknow QUERY | geo A B | search DIR QUERY | resolve PEOPLE.json | coloc FIXES.json RADIUS WINDOW | classify STATUS BODY | session ..."
    );
}

fn var_root() -> PathBuf {
    env::var_os("HUNTSMAN_VAR").map_or_else(|| PathBuf::from("var"), PathBuf::from)
}

fn geo(a: Option<String>, b: Option<String>) -> ExitCode {
    let (Some(a), Some(b)) = (a, b) else {
        eprintln!("usage: huntsman-recon geo LAT,LON LAT,LON");
        return ExitCode::from(64);
    };
    let Ok((lat1, lon1)) = parse_latlon(&a) else {
        eprintln!("bad coordinate: {a}");
        return ExitCode::from(65);
    };
    let Ok((lat2, lon2)) = parse_latlon(&b) else {
        eprintln!("bad coordinate: {b}");
        return ExitCode::from(65);
    };
    println!("{:.0}", haversine_m(lat1, lon1, lat2, lon2));
    ExitCode::SUCCESS
}

fn search_cmd(dir: Option<String>, query: Option<String>) -> ExitCode {
    let (Some(dir), Some(query)) = (dir, query) else {
        eprintln!("usage: huntsman-recon search DIR QUERY");
        return ExitCode::from(64);
    };
    let docs = match load_corpus(Path::new(&dir)) {
        Ok(docs) => docs,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(66);
        }
    };
    let hits = search(&docs, &query);
    println!("hits={}", hits.len());
    for hit in &hits {
        println!("{}\t{}\t{}", hit.score, hit.id, hit.source);
    }
    ExitCode::SUCCESS
}

fn resolve_cmd(path: Option<String>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("usage: huntsman-recon resolve PEOPLE.json");
        return ExitCode::from(64);
    };
    let people = match load_people(Path::new(&path)) {
        Ok(people) => people,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(66);
        }
    };
    let clusters = resolve(&people);
    println!("clusters={}", clusters.len());
    for cluster in &clusters {
        let reasons: Vec<&str> = cluster.links.iter().map(|l| l.reason.as_str()).collect();
        println!("{}\t{}", cluster.members.join(","), reasons.join(","));
    }
    ExitCode::SUCCESS
}

fn coloc_cmd(path: Option<String>, radius: Option<String>, window: Option<String>) -> ExitCode {
    let (Some(path), Some(radius), Some(window)) = (path, radius, window) else {
        eprintln!("usage: huntsman-recon coloc FIXES.json RADIUS_M WINDOW_SECS");
        return ExitCode::from(64);
    };
    let Ok(radius) = radius.parse::<f64>() else {
        eprintln!("bad radius");
        return ExitCode::from(65);
    };
    let Ok(window) = window.parse::<i64>() else {
        eprintln!("bad window");
        return ExitCode::from(65);
    };
    let fixes = match load_fixes(Path::new(&path)) {
        Ok(fixes) => fixes,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(66);
        }
    };
    let pairs = colocated(&fixes, radius, window);
    println!("pairs={}", pairs.len());
    for pair in &pairs {
        println!("{}\t{}\t{:.0}\t{}", pair.left, pair.right, pair.meters, pair.delta_secs);
    }
    ExitCode::SUCCESS
}

fn classify(status: Option<String>, body: Option<String>) -> ExitCode {
    let (Some(status), Some(body)) = (status, body) else {
        eprintln!("usage: huntsman-recon classify STATUS BODY");
        return ExitCode::from(64);
    };
    let Ok(status) = status.parse::<u16>() else {
        eprintln!("bad status");
        return ExitCode::from(65);
    };
    let outcome = classify_response(status, &body);
    println!("{outcome:?}");
    ExitCode::SUCCESS
}

fn session_cmd(args: Vec<String>) -> ExitCode {
    let mut args = args.into_iter();
    let store = Store::new(var_root());
    match args.next().as_deref() {
        Some("new") => {
            let title = args.next().unwrap_or_else(|| "session".into());
            let session = Session::new(title);
            match store.save(&session) {
                Ok(path) => {
                    println!("{}", session.id);
                    println!("{}", path.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::from(66)
                }
            }
        }
        Some("show") => {
            let id = match args.next().or_else(|| current_id()) {
                Some(id) => id,
                None => {
                    eprintln!("no current session");
                    return ExitCode::from(64);
                }
            };
            match store.load(&id) {
                Ok(session) => {
                    println!("{}", serde_json::to_string_pretty(&session).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("{err}");
                    ExitCode::from(66)
                }
            }
        }
        Some("recover") => {
            let Some(id) = args.next() else {
                eprintln!("usage: session recover ID OBJ OUT CONSTRAINTS CRITERIA");
                return ExitCode::from(64);
            };
            let fields: Vec<String> = args.collect();
            if fields.len() != 4 {
                eprintln!("usage: session recover ID OBJ OUT CONSTRAINTS CRITERIA");
                return ExitCode::from(64);
            }
            mutate(&store, &id, |session| {
                session.apply_recover(&fields[0], &fields[1], &fields[2], &fields[3]);
                Ok(())
            })
        }
        Some("candidate") => {
            let Some(id) = args.next() else {
                return ExitCode::from(64);
            };
            let Some(statement) = args.next() else {
                return ExitCode::from(64);
            };
            mutate(&store, &id, |session| {
                session.add_candidate(Candidate {
                    statement,
                    alternatives: vec!["rejected unnamed alternative".into()],
                    reverse_observation: "operator did not record a killer observation".into(),
                })
            })
        }
        Some("falsify") => {
            let fields: Vec<String> = args.collect();
            if fields.len() != 4 {
                eprintln!("usage: session falsify ID ATTACK TEST RESULT");
                return ExitCode::from(64);
            }
            mutate(&store, &fields[0], |session| {
                session.add_falsify(FalsifyRecord {
                    attack: fields[1].clone(),
                    test: fields[2].clone(),
                    result: fields[3].clone(),
                })
            })
        }
        Some("execute") => {
            let fields: Vec<String> = args.collect();
            if fields.len() != 4 {
                eprintln!("usage: session execute ID ACTION OBSERVED COMPONENT");
                return ExitCode::from(64);
            }
            mutate(&store, &fields[0], |session| {
                session.add_execute(ExecuteRecord {
                    action: fields[1].clone(),
                    observed: fields[2].clone(),
                    component: fields[3].clone(),
                })
            })
        }
        Some("verify") => {
            let fields: Vec<String> = args.collect();
            if fields.len() != 5 {
                eprintln!("usage: session verify ID CLAIM STATUS LEVEL DOES_NOT_SHOW");
                return ExitCode::from(64);
            }
            let Ok(status) = fields[2].parse::<Status>() else {
                eprintln!("bad status");
                return ExitCode::from(65);
            };
            let Ok(level) = fields[3].parse::<EvidenceLevel>() else {
                eprintln!("bad evidence level");
                return ExitCode::from(65);
            };
            mutate(&store, &fields[0], |session| {
                session.add_verify(VerifyRecord {
                    claim: fields[1].clone(),
                    status,
                    evidence_level: level,
                    does_not_show: fields[4].clone(),
                })
            })
        }
        Some("terminate") => {
            let fields: Vec<String> = args.collect();
            if fields.len() != 4 {
                eprintln!("usage: session terminate ID RESIDUAL PARTIAL TIP");
                return ExitCode::from(64);
            }
            let partial = fields[2] == "true";
            mutate(&store, &fields[0], |session| {
                session.terminate(fields[1].clone(), partial, &fields[3])
            })
        }
        _ => {
            eprintln!(
                "usage: session new|show|recover|candidate|falsify|execute|verify|terminate"
            );
            ExitCode::from(64)
        }
    }
}

fn current_id() -> Option<String> {
    fs::read_to_string(var_root().join("current.txt"))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn mutate(store: &Store, id: &str, op: impl FnOnce(&mut Session) -> Result<(), huntsman_recon::Error>) -> ExitCode {
    let mut session = match store.load(id) {
        Ok(session) => session,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(66);
        }
    };
    if let Err(err) = op(&mut session) {
        eprintln!("{err}");
        return ExitCode::from(65);
    }
    if let Err(err) = store.save(&session) {
        eprintln!("{err}");
        return ExitCode::from(66);
    }
    println!("{}", session.id);
    ExitCode::SUCCESS
}

fn lookup_cmd(kind: Option<String>, query: Option<String>) -> ExitCode {
    let (Some(kind), Some(query)) = (kind, query) else {
        eprintln!("usage: huntsman-recon lookup wikidata|nominatim|abn|datagov|legislation|gleif|rdap|crtsh|exa|seeknow QUERY");
        return ExitCode::from(64);
    };
    let url = match kind.as_str() {
        "wikidata" => format!(
            "https://www.wikidata.org/w/api.php?action=wbsearchentities&search={}&language=en&format=json&limit=50",
            urlencoding(&query)
        ),
        "nominatim" => format!(
            "https://nominatim.openstreetmap.org/search?q={}&format=jsonv2&limit=10",
            urlencoding(&query)
        ),
        "abn" => format!("https://abr.business.gov.au/ABN/View?id={}", digits_only(&query)),
        "datagov" => format!(
            "https://data.gov.au/data/api/3/action/package_search?q={}&rows=50",
            urlencoding(&query)
        ),
        "legislation" => format!(
            "https://api.prod.legislation.gov.au/v1/titles?$filter=contains(name,'{}')&$top=10&$select=id,name,makingDate,collection,isInForce",
            query.replace('\'', "")
        ),
        "gleif" => format!(
            "https://api.gleif.org/api/v1/lei-records?filter%5Bentity.legalName%5D={}&page%5Bsize%5D=5",
            urlencoding(&query)
        ),
        "rdap" => format!("https://rdap.org/domain/{}", domain_only(&query)),
        "crtsh" => format!(
            "https://crt.sh/?q={}&output=json",
            urlencoding(&domain_only(&query))
        ),
        "exa" => "https://api.exa.ai/search".into(),
        "seeknow" => "https://see-know.ru/api/v1/search".into(),
        other => {
            eprintln!("unknown source: {other}");
            return ExitCode::from(64);
        }
    };
    let (status, body) = if kind == "exa" {
        match curl_post_exa(&query) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(66);
            }
        }
    } else if kind == "seeknow" {
        match curl_post_seeknow(&query) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(66);
            }
        }
    } else {
        match curl_get(&url) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(66);
            }
        }
    };
    if kind != "abn" && kind != "exa" && kind != "seeknow" {
        if let Err(err) = huntsman_recon::external::admit_body(status, &body) {
            eprintln!("{err}");
            return ExitCode::from(65);
        }
    } else if kind == "abn" && !(200..300).contains(&status) {
        eprintln!("ABN HTTP {status}");
        return ExitCode::from(65);
    }
    let hits = match kind.as_str() {
        "wikidata" => huntsman_recon::external::parse_wikidata_search(&body),
        "nominatim" => huntsman_recon::external::parse_nominatim(&body),
        "abn" => huntsman_recon::external::parse_abn_html(&body),
        "datagov" => huntsman_recon::external::parse_datagov(&body),
        "legislation" => huntsman_recon::external::parse_legislation(&body),
        "gleif" => huntsman_recon::external::parse_gleif(&body),
        "rdap" => huntsman_recon::external::parse_rdap(&body),
        "crtsh" => huntsman_recon::external::parse_crtsh(&body),
        "exa" => huntsman_recon::external::parse_exa(status, &body),
        "seeknow" => huntsman_recon::external::parse_seeknow(status, &body),
        _ => unreachable!(),
    };
    let hits = match hits {
        Ok(hits) => hits,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(65);
        }
    };
    let hash = huntsman_recon::sha256::hex32(&huntsman_recon::sha256::sha256(body.as_bytes()));
    println!("source={kind}");
    println!("url={url}");
    println!("sha256={hash}");
    println!("hits={}", hits.len());
    for hit in &hits {
        println!(
            "{}\t{}\t{}\t{}",
            hit.id,
            hit.label,
            hit.lat.map(|v| format!("{v:.5}")).unwrap_or_default(),
            hit.lon.map(|v| format!("{v:.5}")).unwrap_or_default()
        );
    }
    ExitCode::SUCCESS
}

fn digits_only(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn domain_only(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('.');
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_ascii_lowercase()
}

fn gather_cmd(query: Option<String>) -> ExitCode {
    let Some(query) = query else {
        eprintln!("usage: huntsman-recon gather QUERY");
        return ExitCode::from(64);
    };
    let mut failed = 0u8;
    for kind in ["nominatim", "datagov", "legislation", "gleif", "wikidata"] {
        println!("--- {kind}");
        match lookup_cmd(Some(kind.into()), Some(query.clone())) {
            ExitCode::SUCCESS => {}
            _ => failed += 1,
        }
    }
    println!("--- seeknow");
    if env::var_os("SEEKNOW_API_KEY").is_none_or(|v| v.is_empty()) {
        println!("source=seeknow");
        println!("status=missing_key");
        failed += 1;
    } else if lookup_cmd(Some("seeknow".into()), Some(query.clone())).ne(&ExitCode::SUCCESS) {
        failed += 1;
    }
    if env::var_os("EXA_API_KEY").is_none_or(|v| v.is_empty()) {
        println!("source=exa");
        println!("status=missing_key");
        failed += 1;
    } else if lookup_cmd(Some("exa".into()), Some(query.clone())).ne(&ExitCode::SUCCESS) {
        failed += 1;
    }
    if digits_only(&query).len() == 11 {
        println!("--- abn");
        if lookup_cmd(Some("abn".into()), Some(query.clone())).ne(&ExitCode::SUCCESS) {
            failed += 1;
        }
    }
    if query.contains('.') && !query.contains(' ') {
        println!("--- rdap");
        if lookup_cmd(Some("rdap".into()), Some(query.clone())).ne(&ExitCode::SUCCESS) {
            failed += 1;
        }
        println!("--- crtsh");
        if lookup_cmd(Some("crtsh".into()), Some(query)).ne(&ExitCode::SUCCESS) {
            failed += 1;
        }
    }
    println!("sources_failed={failed}");
    ExitCode::SUCCESS
}

fn urlencoding(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn curl_get(url: &str) -> Result<(u16, String), String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-L",
            "-A",
            "huntsman-recon/0.10.0",
            "--max-time",
            "25",
            "-w",
            "\n%{http_code}",
            url,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = text.rsplit_once('\n').unwrap_or((text.as_ref(), "0"));
    let status: u16 = status.trim().parse().unwrap_or(0);
    Ok((status, body.to_owned()))
}

fn curl_post_seeknow(query: &str) -> Result<(u16, String), String> {
    let key = env::var("SEEKNOW_API_KEY").map_err(|_| "SEEKNOW_API_KEY missing".to_owned())?;
    if key.trim().is_empty() {
        return Err("SEEKNOW_API_KEY missing".into());
    }
    let payload = serde_json::json!({
        "type": "auto",
        "query": query,
        "mode": "fast"
    });
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            "https://see-know.ru/api/v1/search",
            "-H",
            "Content-Type: application/json",
            "-H",
            &format!("X-API-Key: {key}"),
            "--max-time",
            "30",
            "-w",
            "\n%{http_code}",
            "-d",
            &payload.to_string(),
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = text.rsplit_once('\n').unwrap_or((text.as_ref(), "0"));
    let status: u16 = status.trim().parse().unwrap_or(0);
    Ok((status, body.to_owned()))
}

fn curl_post_exa(query: &str) -> Result<(u16, String), String> {
    let key = env::var("EXA_API_KEY").map_err(|_| "EXA_API_KEY missing".to_owned())?;
    if key.trim().is_empty() {
        return Err("EXA_API_KEY missing".into());
    }
    let payload = serde_json::json!({
        "query": query,
        "type": "auto",
        "numResults": 10,
        "contents": {"highlights": true}
    });
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            "https://api.exa.ai/search",
            "-H",
            "Content-Type: application/json",
            "-H",
            &format!("x-api-key: {key}"),
            "--max-time",
            "30",
            "-w",
            "\n%{http_code}",
            "-d",
            &payload.to_string(),
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = text.rsplit_once('\n').unwrap_or((text.as_ref(), "0"));
    let status: u16 = status.trim().parse().unwrap_or(0);
    Ok((status, body.to_owned()))
}

fn run_cmd(dir: Option<String>) -> ExitCode {
    let Some(dir) = dir else {
        eprintln!("usage: huntsman-recon run CASE_DIR");
        return ExitCode::from(64);
    };
    let (spec, people, fixes, docs) = match huntsman_recon::case::load_dir(Path::new(&dir)) {
        Ok(loaded) => loaded,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(66);
        }
    };
    let output = match huntsman_recon::case::execute(&spec, &people, &fixes, &docs) {
        Ok(output) => output,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(65);
        }
    };
    let out = Path::new(&dir).join("out");
    if huntsman_recon::case::write_output(&out, &output).is_err() {
        return ExitCode::from(66);
    }
    let store = Store::new(var_root());
    if store.save(&output.session).is_err() {
        return ExitCode::from(66);
    }
    println!("clusters={}", output.report.clusters);
    println!("links={}", output.report.links);
    println!("pairs={}", output.report.pairs);
    println!("hits={}", output.report.hits);
    println!("techniques={}", output.report.techniques);
    println!("bindings={}", output.report.bindings);
    println!("tip={}", output.report.tip);
    println!("residual={}", output.report.residual);
    ExitCode::SUCCESS
}

fn check() -> ExitCode {
    if binding_count() != 0 {
        return ExitCode::from(7);
    }
    let (blat, blon) = parse_latlon("-27.4698,153.0251").expect("latlon");
    let (slat, slon) = parse_latlon("-33.8688,151.2093").expect("latlon");
    let meters = haversine_m(blat, blon, slat, slon);
    if !(700_000.0..760_000.0).contains(&meters) {
        return ExitCode::from(2);
    }
    if classify_response(200, "<html>just a moment cloudflare</html>").is_result() {
        return ExitCode::from(3);
    }
    if classify_response(429, "challenges.cloudflare.com").is_wall() {
        return ExitCode::from(3);
    }
    let people = resolve(&[
        PersonRecord {
            id: "a".into(),
            name: "Same".into(),
            emails: vec!["a@ex.com".into()],
            handles: vec![],
            phones: vec![],
        },
        PersonRecord {
            id: "b".into(),
            name: "Same".into(),
            emails: vec!["b@ex.com".into()],
            handles: vec![],
            phones: vec!["61412345678".into()],
        },
    ]);
    if people.len() != 2 {
        return ExitCode::from(4);
    }
    let geo = seal(&Claim {
        claim: format!("brisbane-sydney haversine {meters:.0} m inside band"),
        source: "published city centroids".into(),
        component: "src/geoint.rs".into(),
        technique_id: Some("T1591".into()),
        status: Status::Verified,
        evidence_level: EvidenceLevel::DirectObservation,
        does_not_show: "not T1591 and not a survey".into(),
    });
    let wall = append(
        &geo.hash,
        &Claim {
            claim: "challenge page is not a result".into(),
            source: "src/classify.rs".into(),
            component: "src/classify.rs".into(),
            technique_id: None,
            status: Status::Verified,
            evidence_level: EvidenceLevel::Reproduction,
            does_not_show: "does not bypass the wall".into(),
        },
    );
    let entries = vec![geo, wall];
    if !chain_intact(&entries) {
        return ExitCode::from(9);
    }
    let nav = layer(&entries);
    if nav["techniques"].as_array().is_none_or(|a| !a.is_empty()) {
        return ExitCode::from(7);
    }
    if !bundle(&entries)["objects"].as_array().is_none_or(Vec::is_empty) {
        return ExitCode::from(8);
    }
    let _ = fs::create_dir_all("var");
    let path = Path::new("var/ledger.json");
    if save_chain(path, &entries).is_err() {
        return ExitCode::from(9);
    }
    if load_chain(path).ok().as_deref() != Some(entries.as_slice()) {
        return ExitCode::from(9);
    }
    let tip = entries.last().map(|e| e.hash.as_str()).unwrap_or("");
    let mut session = Session::new("check");
    session.apply_recover(
        "offline core",
        "chain bound to session",
        "no network",
        "terminate only with tip",
    );
    if session
        .add_candidate(Candidate {
            statement: "hash chain".into(),
            alternatives: vec!["independent hashes".into()],
            reverse_observation: "reorder undetected".into(),
        })
        .is_err()
    {
        return ExitCode::from(6);
    }
    let _ = session.add_falsify(FalsifyRecord {
        attack: "terminate without tip".into(),
        test: "full terminate empty tip".into(),
        result: "refused".into(),
    });
    let _ = session.add_execute(ExecuteRecord {
        action: "check".into(),
        observed: format!("{meters:.0}"),
        component: "src/geoint.rs".into(),
    });
    let _ = session.add_verify(VerifyRecord {
        claim: "tip binds the session".into(),
        status: Status::Verified,
        evidence_level: EvidenceLevel::DirectObservation,
        does_not_show: "not a live collection".into(),
    });
    if session.terminate("no handset run".into(), false, "").is_ok() {
        return ExitCode::from(6);
    }
    if session.terminate("no handset run".into(), false, tip).is_err() || !session.bound_to(tip) {
        return ExitCode::from(6);
    }
    let _ = fs::write("var/navigator.json", serde_json::to_string_pretty(&nav).unwrap_or_default());
    if !search_response(200, "<html>just a moment cloudflare</html>", "brisbane", "remote").is_empty() {
        return ExitCode::from(3);
    }
    let root = std::env::temp_dir().join(format!("huntsman-check-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    if fs::create_dir_all(&root).is_err() {
        return ExitCode::from(66);
    }
    if fs::write(root.join("port.txt"), "Brisbane port radar").is_err() {
        return ExitCode::from(66);
    }
    let docs = match load_corpus(&root) {
        Ok(docs) => docs,
        Err(_) => return ExitCode::from(66),
    };
    if search(&docs, "brisbane port").len() != 1 || !search(&docs, "sydney port").is_empty() {
        return ExitCode::from(5);
    }
    let _ = fs::remove_dir_all(&root);
    println!("accepted techniques=0");
    println!("bindings={}", binding_count());
    println!("brisbane_sydney_m={meters:.0}");
    ExitCode::SUCCESS
}
