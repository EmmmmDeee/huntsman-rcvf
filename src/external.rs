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
    }
}
