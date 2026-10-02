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
    }
}
