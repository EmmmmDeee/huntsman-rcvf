# Reconstruction decision — 2026-10-02

Target: an unprivileged offline Rust core that records an RCVF session, refuses an unsupported claim, resolves identity only on a shared email or handle, computes geodesic distance and co-location, seals claims in a hashed ledger, and emits STIX or an ATT&CK Navigator layer only when this crate implements that technique.

Search is local retrieval over operator-supplied documents. Every query term must match. A challenge page or a 429 is not a hit. Paid SeekNow and public SearXNG JSON are not sources.

A verified capability is not a technique score. The binding table is empty. Haversine is not T1591. Challenge classification is not T1592.

## Disposition

| Legacy | Decision | Why |
| --- | --- | --- |
| v1 recorder | MIGRATE contract | Terminate gaps earned survival. Clap CLI did not. |
| Level-1 classifier | MIGRATE invariant | 429 beats a vendor string. A challenge page is Blocked. |
| HSE monolith | REMOVE from current tree | History retained. Credentials and live providers do not earn a place. |
| Self-assigned T1591 layer | REPLACE | Unsupported acceptance. Gate now requires an implemented binding. |
| SeekNow keyless | REMOVE | NOT APPLICABLE. |
| Public SearXNG JSON | CLOSED, excluded | 403/429 on this egress. Not a source. No further probe. |
| Termux aarch64 | CLOSED, excluded | No device in this session. Not a blocker for the offline core. |

## Acceptance

`cargo test` — 16 passed (14 unit + 2 accept), 2026-10-02 15:55 AEST, `CARGO_TARGET_DIR=/tmp/hse-recon-target`. `huntsman-recon check` — techniques=0, Brisbane–Sydney 732379 m.

No Termux run. No live harvest. No survey-grade geodesy. Catalog presence is not a score.

## Zip extract 2026-10-02 16:49 AEST

`Huntsman-Search-Engine-HSE-Termux-Android-Aarch64-Rust--main (9).zip` is being copied to `artifacts/hse-zip`. `.agent/state.json` and `.env` files are excluded. At the last count, 502 files were on disk and the extract had not finished. That tree is history of the monolith. It does not replace `huntsman-recon` 0.2.0.

Scan `83b8aa50e577` (HSE 1.41.0, username `bandito`) is evidence, not a spec. It stopped at `max_entities=2500`, then the correlation pass died at 116 of 122 rules. A restaurant name sharing the handle was stored as an address. ATT&CK tags were attached without a binding. One harvested token was listed. That confirms the recon rules: namesake is not identity, a catalog tag is not a technique, a wall or a budget stop is not a complete answer.

| Attachment | Decision |
| --- | --- |
| Debug dossier | EVIDENCE only. Not absorbed. |
| Organised repair plan | HIBP runtime-only key and secret scan earn a place if applied to a checkout. Not applied here. |
| Outstanding bundle `wire_seeknow_bulk.py` | REJECTED. Same closed SeekNow path. |
| Credential audit | PRESERVE the lesson: no live token in source. No history rewrite from this tree. |
| Dependency graph | HISTORY of the monolith. Not the current architecture. |

| Item | Decision | Why |
| --- | --- | --- |
| Current binary | `huntsman-recon` 0.2.0 | 16 tests, check prints techniques=0 and 732379 m. |
| Root `huntsman` 0.1.0 Clap crate | RETIRE as current binary | Contract migrated. Clap did not earn a second CLI. Files kept as history. |
| `hse-level1` | RETIRE as duplicate | Invariant lives in `huntsman-recon` classify. Floor crate compiled, tests not executed (noexec). |
| HSE zip | HISTORY | Not absorbed. |
| Git push | CLOSED, not from this tree | Not a git checkout. |
| Termux aarch64 | CLOSED, excluded | No device in this session. |
| SeekNow, DeHashed, SearXNG JSON | CLOSED, excluded | Not sources. |
