# Architecture — huntsman-recon 0.4.0

Defined independently of the monolith, then scored against it.

## Objective

One offline case run. Resolve identity, compute co-location, retrieve operator documents, seal a hash chain, and terminate an RCVF session bound to that tip. STIX and Navigator stay empty until this crate implements a technique.

## Boundaries

No network client. No paid source. No live geolocation. Challenge page and HTTP 429 are not hits. A display name is not identity. National and international phone forms do not merge. Plus-tags and dotted local-parts do not merge. Haversine is not T1591. Classification is not T1592. Catalog presence is not a score. ABR, ASIC, HIBP, VirusTotal, Shodan, and SeekNow stay out: they need a live client or a key. Termux aarch64 is out of this session.

## Contracts

- `run CASE_DIR` reads optional `case.json`, `people.json`, `fixes.json`, and `corpus/`. Writes `out/ledger.json`, `out/navigator.json`, `out/stix.json`, `out/report.json`, and a session.
- Missing inputs are named in the residual. They are not silent success.
- `search`, `resolve`, `coloc`, `geo`, `classify`, and `session` remain the single-step paths.
- `check` fails closed if a wall is a result or a technique is admitted.

## Invariants

Binding table is empty. Interop admission requires Verified, evidence at or above direct observation, a non-empty component, a valid technique id, and `method_implements`. Store refuses a symlink session path. A case run that emits a technique is an error.

## Disposition

| Element | Decision | Why |
| --- | --- | --- |
| 0.3.0 gates and operator commands | PRESERVE | Already killed name-merge, wall-as-hit, catalog score |
| Separate commands as the only system | REPLACE | Modules were not one run |
| Live provider modules | REMOVE | No key, no client, not a source |
| Empty binding table | PRESERVE | No function here performs an ATT&CK technique |
| AU use notes | PRESERVE as evidence | Not a capability |

