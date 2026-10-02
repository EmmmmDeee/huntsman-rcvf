# Architecture — huntsman-recon 0.3.0

Defined independently of the monolith, then scored against it.

## Objective

Unprivileged offline Rust core. Record an RCVF session. Refuse an unsupported claim. Resolve identity only on a shared canonical email, handle, or phone. Compute sphere distance and co-location. Retrieve over operator-supplied documents with every term required. Seal claims in a hashed ledger. Emit STIX or a Navigator layer only from an in-crate technique binding.

## Boundaries

No network client. No paid source. No live geolocation. Challenge page and HTTP 429 are not hits. A display name is not identity. National and international phone forms do not merge. Plus-tags and dotted local-parts do not merge. Haversine is not T1591. Classification is not T1592. Catalog presence is not a score. Termux aarch64 is out of this session.

## Contracts

- `search DIR QUERY` reads regular `.txt`, `.md`, `.json` files. Symlinks skipped. All tokens required.
- `resolve PEOPLE.json` clusters on email, handle, or phone.
- `coloc FIXES.json RADIUS WINDOW` reports pairs inside both gates.
- `session` persists under `HUNTSMAN_VAR` or `./var`. Full terminate needs recover, candidate, falsification, verification, and a 64-hex tip.
- `check` fails closed if bindings are non-empty without a matching admission, or if a wall is treated as a result.

## Invariants

Binding table is empty. Interop admission requires Verified, evidence at or above direct observation, a non-empty component, a valid technique id, and `method_implements`. Store refuses a symlink session path.

## Disposition

| Element | Decision | Why |
| --- | --- | --- |
| 0.2.0 library gates | PRESERVE | Tests already killed name-merge, wall-as-hit, catalog score |
| Fixture-only CLI search | REPLACE | Operator path was not the library path |
| Phone key | REIMPLEMENT | People-centric gap. Conservative digits only |
| Clap CLI | REMOVE | Did not earn a second parser |
| HSE monolith, SeekNow, SearXNG | REMOVE from this crate | Closed. Not sources |
| Empty binding table | PRESERVE | No function here performs an ATT&CK technique |
