//! Public-source parsers. A blocked or throttled body is not a hit.
//! Live fetch stays in the binary. This module does not open a socket.

use crate::classify::{classify_response, FetchOutcome};
use crate::error::Error;
use crate::geoint::haversine_m;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceHit {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub source: String,
}

/// Refuse a wall, a throttle, and a non-2xx body before parsing.
pub fn admit_body(status: u16, body: &str) -> Result<(), Error> {
    match classify_response(status, body) {
        FetchOutcome::Parsed => Ok(()),
        FetchOutcome::RateLimited { status, detail } => {
            Err(Error::Invalid(format!("rate limited {status}: {detail}")))
        }
        FetchOutcome::Blocked { status, detail } => {
            Err(Error::Invalid(format!("blocked {status}: {detail}")))
        }
        FetchOutcome::Failed { detail } => Err(Error::Invalid(detail)),
    }
}

pub fn parse_wikidata_search(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    let Some(rows) = value.get("search").and_then(Value::as_array) else {
        return Err(Error::Invalid("wikidata search missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        if !id.starts_with('Q') {
            continue;
        }
        hits.push(SourceHit {
            id: id.to_owned(),
            label: row.get("label").and_then(Value::as_str).unwrap_or("").to_owned(),
            detail: row.get("description").and_then(Value::as_str).unwrap_or("").to_owned(),
            lat: None,
            lon: None,
            source: "wikidata".into(),
        });
    }
    Ok(hits)
}

pub fn parse_nominatim(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    let Some(rows) = value.as_array() else {
        return Err(Error::Invalid("nominatim array missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let lat = row.get("lat").and_then(Value::as_str).and_then(|s| s.parse().ok());
        let lon = row.get("lon").and_then(Value::as_str).and_then(|s| s.parse().ok());
        let (Some(lat), Some(lon)) = (lat, lon) else {
            continue;
        };
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            continue;
        }
        let osm_type = row.get("osm_type").and_then(Value::as_str).unwrap_or("osm");
        let osm_id = row.get("osm_id").map(|v| v.to_string()).unwrap_or_default();
        hits.push(SourceHit {
            id: format!("{osm_type}:{osm_id}"),
            label: row.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
            detail: row.get("licence").and_then(Value::as_str).unwrap_or("OpenStreetMap").to_owned(),
            lat: Some(lat),
            lon: Some(lon),
            source: "nominatim".into(),
        });
    }
    Ok(hits)
}

/// Public ABN view. A recaptcha script on the same page is not a reason to drop the legal name.
pub fn parse_abn_html(body: &str) -> Result<Vec<SourceHit>, Error> {
    let Some(name) = between(body, "itemprop=\"legalName\">", "</span>") else {
        return Err(Error::Invalid("ABN page has no legal name".into()));
    };
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("ABN legal name empty".into()));
    }
    let title = between(body, "<title>", "</title>").unwrap_or("");
    let abn: String = title.chars().filter(|c| c.is_ascii_digit()).collect();
    if abn.len() != 11 {
        return Err(Error::Invalid("ABN title has no 11-digit identifier".into()));
    }
    let status = between(body, "ABN status:</th>", "</td>")
        .unwrap_or("")
        .replace("&nbsp;", " ");
    let status = status.split('<').next().unwrap_or("").trim().to_owned();
    Ok(vec![SourceHit {
        id: abn,
        label: name.to_owned(),
        detail: status,
        lat: None,
        lon: None,
        source: "abn".into(),
    }])
}

pub fn parse_datagov(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(Error::Invalid("data.gov.au search failed".into()));
    }
    let Some(rows) = value.pointer("/result/results").and_then(Value::as_array) else {
        return Err(Error::Invalid("data.gov.au results missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        hits.push(SourceHit {
            id: id.to_owned(),
            label: row.get("title").and_then(Value::as_str).unwrap_or("").to_owned(),
            detail: row.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
            lat: None,
            lon: None,
            source: "data.gov.au".into(),
        });
    }
    Ok(hits)
}

fn between<'a>(body: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = body.split_once(start)?.1;
    Some(rest.split_once(end)?.0)
}

/// Federal Register of Legislation titles.
pub fn parse_legislation(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    let Some(rows) = value.get("value").and_then(Value::as_array) else {
        return Err(Error::Invalid("legislation titles missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        hits.push(SourceHit {
            id: id.to_owned(),
            label: row.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
            detail: row.get("collection").and_then(Value::as_str).unwrap_or("").to_owned(),
            lat: None,
            lon: None,
            source: "legislation.gov.au".into(),
        });
    }
    Ok(hits)
}

/// GLEIF LEI records. Legal name may be a string or an object with `name`.
pub fn parse_gleif(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    let Some(rows) = value.get("data").and_then(Value::as_array) else {
        return Err(Error::Invalid("gleif data missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let legal = row.pointer("/attributes/entity/legalName");
        let label = match legal {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Object(map)) => map.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
            _ => String::new(),
        };
        let status = row.pointer("/attributes/entity/status").and_then(Value::as_str).unwrap_or("");
        hits.push(SourceHit {
            id: id.to_owned(),
            label,
            detail: status.to_owned(),
            lat: None,
            lon: None,
            source: "gleif".into(),
        });
    }
    Ok(hits)
}

/// RDAP domain object. A notice-only or non-domain body is not a hit.
pub fn parse_rdap(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    if value.get("errorCode").is_some() {
        return Err(Error::Invalid("rdap error".into()));
    }
    let class = value.get("objectClassName").and_then(Value::as_str).unwrap_or("");
    if class != "domain" {
        return Err(Error::Invalid("rdap domain missing".into()));
    }
    let Some(name) = value.get("ldhName").and_then(Value::as_str) else {
        return Err(Error::Invalid("rdap name missing".into()));
    };
    let registrar = rdap_registrar(&value);
    let status = value
        .get("status")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    let ns = value
        .get("nameservers")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("ldhName").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    Ok(vec![SourceHit {
        id: name.to_owned(),
        label: registrar,
        detail: format!("{status} ns={ns}"),
        lat: None,
        lon: None,
        source: "rdap".into(),
    }])
}

/// crt.sh certificate rows. An HTML wall is not a hit.
pub fn parse_crtsh(body: &str) -> Result<Vec<SourceHit>, Error> {
    admit_body(200, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    let Some(rows) = value.as_array() else {
        return Err(Error::Invalid("crtsh array missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let id = row
            .get("id")
            .map(std::string::ToString::to_string)
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let name = row
            .get("common_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let names = row.get("name_value").and_then(Value::as_str).unwrap_or("");
        let issuer = row.get("issuer_name").and_then(Value::as_str).unwrap_or("");
        hits.push(SourceHit {
            id,
            label: name,
            detail: format!("{issuer} names={names}"),
            lat: None,
            lon: None,
            source: "crtsh".into(),
        });
    }
    Ok(hits)
}

fn rdap_registrar(value: &Value) -> String {
    let Some(entities) = value.get("entities").and_then(Value::as_array) else {
        return String::new();
    };
    for entity in entities {
        let roles = entity.get("roles").and_then(Value::as_array);
        let is_registrar = roles.is_some_and(|rows| {
            rows.iter().any(|role| role.as_str() == Some("registrar"))
        });
        if !is_registrar {
            continue;
        }
        if let Some(name) = vcard_fn(entity) {
            return name;
        }
        if let Some(handle) = entity.get("handle").and_then(Value::as_str) {
            return handle.to_owned();
        }
    }
    String::new()
}

fn vcard_fn(entity: &Value) -> Option<String> {
    let rows = entity.get("vcardArray")?.as_array()?.get(1)?.as_array()?;
    for row in rows {
        let cells = row.as_array()?;
        if cells.first().and_then(Value::as_str) == Some("fn") {
            return cells.get(3).and_then(Value::as_str).map(str::to_owned);
        }
    }
    None
}

/// Exa search JSON. A 402 or an error tag is not a hit.
pub fn parse_exa(status: u16, body: &str) -> Result<Vec<SourceHit>, Error> {
    if status == 402 || body.contains("X402_PAYMENT_REQUIRED") {
        return Err(Error::Invalid("exa payment required".into()));
    }
    admit_body(status, body)?;
    let value: Value = serde_json::from_str(body).map_err(|e| Error::Invalid(e.to_string()))?;
    if value.get("error").is_some() {
        return Err(Error::Invalid("exa error".into()));
    }
    let Some(rows) = value.get("results").and_then(Value::as_array) else {
        return Err(Error::Invalid("exa results missing".into()));
    };
    let mut hits = Vec::new();
    for row in rows {
        let Some(url) = row.get("url").and_then(Value::as_str) else {
            continue;
        };
        let highlight = row
            .get("highlights")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("");
        hits.push(SourceHit {
            id: row.get("id").and_then(Value::as_str).unwrap_or(url).to_owned(),
            label: row.get("title").and_then(Value::as_str).unwrap_or(url).to_owned(),
            detail: highlight.to_owned(),
            lat: None,
            lon: None,
            source: "exa".into(),
        });
    }
    Ok(hits)
}

/// SeekNow search JSON or stream. A missing session or an error is not a hit.
pub fn parse_seeknow(status: u16, body: &str) -> Result<Vec<SourceHit>, Error> {
    if status == 401 || status == 402 || status == 403 {
        return Err(Error::Invalid("seeknow session rejected".into()));
    }
    admit_body(status, body)?;
    if body.contains("QUERY_BLACKLISTED") {
        return Err(Error::Invalid("seeknow query rejected".into()));
    }
    let mut hits = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("error").is_some() {
            return Err(Error::Invalid("seeknow error".into()));
        }
        collect_seeknow(&value, &mut hits);
    }
    if hits.is_empty() {
        if let Ok(value) = serde_json::from_str::<Value>(body) {
            if value.get("error").is_some() {
                return Err(Error::Invalid("seeknow error".into()));
            }
            collect_seeknow(&value, &mut hits);
        }
    }
    Ok(hits)
}

fn collect_seeknow(value: &Value, hits: &mut Vec<SourceHit>) {
    if let Some(rows) = value.get("results").and_then(Value::as_array) {
        for row in rows {
            push_seeknow(row, hits);
        }
        return;
    }
    push_seeknow(value, hits);
}

fn push_seeknow(row: &Value, hits: &mut Vec<SourceHit>) {
    let id = row
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| row.get("source").and_then(Value::as_str))
        .unwrap_or("");
    if id.is_empty() {
        return;
    }
    let label = row
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| row.get("source").and_then(Value::as_str))
        .unwrap_or("")
        .to_owned();
    let source = row.get("source").and_then(Value::as_str).unwrap_or("");
    hits.push(SourceHit {
        id: id.to_owned(),
        label,
        detail: source.to_owned(),
        lat: None,
        lon: None,
        source: "seeknow".into(),
    });
}

/// Two public fixes. Returns meters. Does not infer identity.
#[must_use]
pub fn separation_m(left: &SourceHit, right: &SourceHit) -> Option<f64> {
    Some(haversine_m(left.lat?, left.lon?, right.lat?, right.lon?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_agree_on_brisbane_and_a_wall_is_not_a_hit() {
        let wd = parse_wikidata_search(include_str!("../fixtures/wikidata-brisbane.json")).unwrap();
        assert_eq!(wd[0].id, "Q34932");
        assert_eq!(wd[0].label, "Brisbane");
        let nom = parse_nominatim(include_str!("../fixtures/nominatim-brisbane.json")).unwrap();
        assert_eq!(nom[0].label, "Brisbane");
        let meters = haversine_m(nom[0].lat.unwrap(), nom[0].lon.unwrap(), -27.467777777778, 153.02777777778);
        assert!(meters < 2_000.0, "{meters}");
        assert!(parse_wikidata_search("<html>just a moment cloudflare</html>").is_err());
        assert!(parse_nominatim("").is_err());
        let abn = parse_abn_html(include_str!("../fixtures/abn-bp.html")).unwrap();
        assert_eq!(abn[0].label, "B P AUSTRALIA PTY LTD");
        assert_eq!(abn[0].id, "53004085616");
        let sets = parse_datagov(include_str!("../fixtures/datagov-brisbane.json")).unwrap();
        assert_eq!(sets[0].label, "Events — Brisbane parks");
        assert!(parse_exa(402, include_str!("../fixtures/exa-402.json")).is_err());
        let schema = r#"{"results":[{"id":"schema","title":"Schema example","url":"https://exa.ai/docs","highlights":["not a live hit"]}]}"#;
        let parsed = parse_exa(200, schema).unwrap();
        assert_eq!(parsed[0].source, "exa");
        assert_eq!(parsed[0].label, "Schema example");
        let laws = parse_legislation(include_str!("../fixtures/legislation-brisbane.json")).unwrap();
        assert!(laws[0].label.to_ascii_lowercase().contains("brisbane"));
        assert_eq!(laws[0].id, "C2025G00511");
        let lei = parse_gleif(include_str!("../fixtures/gleif-brisbane.json")).unwrap();
        assert_eq!(lei[0].id, "969500E98BGOX5KEG994");
        assert_eq!(lei[0].label, "BRISBANE MEDIA");
        let rdap = parse_rdap(include_str!("../fixtures/rdap-brisbane.json")).unwrap();
        assert_eq!(rdap[0].id, "brisbane.qld.gov.au");
        assert_eq!(rdap[0].label, "Department of Finance - QLD");
        assert!(rdap[0].detail.contains("ns-1010.awsdns-62.net"));
        assert!(parse_rdap(r#"{"errorCode":404,"title":"Not Found"}"#).is_err());
        let certs = parse_crtsh(include_str!("../fixtures/crtsh-brisbane.json")).unwrap();
        assert_eq!(certs[0].id, "24890111");
        assert_eq!(certs[0].label, "brisbane.qld.gov.au");
        assert!(certs[0].detail.contains("www.brisbane.qld.gov.au"));
        assert!(parse_crtsh("<html>just a moment</html>").is_err());
        let denied = r#"{"error":"invalid_api_key","message":"Missing API key"}"#;
        assert!(parse_seeknow(401, denied).is_err());
        let schema = r#"{"results":[{"id":"schema","type":"domain","source":"example"}]}"#;
        let parsed = parse_seeknow(200, schema).unwrap();
        assert_eq!(parsed[0].source, "seeknow");
        assert_eq!(parsed[0].id, "schema");
    }
}
