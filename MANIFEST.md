# huntsman — MANIFEST

Purpose of this file: project restatement, install root, tree, emission
order, decisions, flagged ambiguities, sanctioned stubs.

## Project restatement

Build a local command-line tool named huntsman that operationalizes the
Recursive Challenge-Verification Framework (RCVF) as durable session
records on an unprivileged Termux Android aarch64 device.

The operator creates a session, fills Recover fields, records competing
candidates, logs falsification attempts and executed tests, assigns an
evidence-ladder level to each claim, records repair / reproduce /
re-attack / simplify steps, and terminates with named residual
uncertainty. The program enforces stage presence and refuse-to-claim
rules. It does not invent findings, call models, or treat a recorded
pass as system correctness.

Objective of the software: strongest verifiable RCVF record under the
platform contract, with the least complexity necessary.

Required outcome: a paste-installable Rust package the operator can
clone from GitHub or paste-install, then build with Termux pkg + cargo
and run without root or network.

## INSTALL_ROOT

INSTALL_ROOT=$HOME/huntsman

All emitted paths are relative to INSTALL_ROOT. Writable runtime paths
stay under $HOME or $TMPDIR.

    $HOME/huntsman/             source and declared install root
    $HOME/huntsman/var/         created at runtime; session JSON
    $HOME/huntsman/config.toml  optional; defaults if absent

## ORIGIN

Public GitHub repository (created 2026-09-15):

    https://github.com/EmmmmDeee/huntsman-rcvf

Default branch: main
Owner: EmmmmDeee (authenticated GitHub account)

Clone:

    git clone https://github.com/EmmmmDeee/huntsman-rcvf.git "$HOME/huntsman"

The crate package name remains `huntsman`. The repository name is
`huntsman-rcvf` so it is not confused with
`EmmmmDeee/Huntsman-Search-Engine-HSE-Termux-Android-Aarch64-Rust-`,
`EmmmmDeee/Huntsman-`, or `EmmmmDeee/rcvf` (spec-only).

## File tree

    MANIFEST.md              this document
    rust-toolchain.toml      MSRV pin (stable 1.87.0)
    .gitignore               target/, editor, var/
    Cargo.toml               single package, not a workspace
    src/lib.rs               library surface and crate attrs
    src/main.rs              binary entry; anyhow
    src/error.rs             thiserror library errors
    src/config.rs            sole env/path/config reader
    src/stage.rs             RCVF stages, ladder, priority
    src/session.rs           session document and transitions
    src/store.rs             filesystem JSON store + bounds
    src/cli.rs               clap derive CLI
    tests/session_flow.rs    end-to-end record transitions
    Cargo.lock               emitted only after cargo generate-lockfile
                             on a 1.87+ toolchain (see ambiguities)
    .github/workflows/ci.yml after source files, if continue requests CI

No workspace members. No extra crates.io packages beyond clap, serde,
serde_json, thiserror, anyhow. Optional config.toml is parsed by a
minimal subset reader in src/config.rs (no `toml` crate).

## Emission order

1. MANIFEST.md                 (this file; stop)
2. rust-toolchain.toml
3. .gitignore
4. Cargo.toml
5. src/error.rs
6. src/config.rs
7. src/stage.rs
8. src/session.rs
9. src/store.rs
10. src/cli.rs
11. src/lib.rs
12. src/main.rs
13. tests/session_flow.rs
14. Cargo.lock                 only if a 1.87+ toolchain is available
15. .github/workflows/ci.yml   GitHub-hosted check on rust-toolchain pin

Stop after each file. continue advances one file. chain continues until
stop or DONE. revise PATH re-emits that file in full.

Each accepted file is also pushed to ORIGIN on main when the GitHub
write primitive succeeds. A 403 is a named limitation, not a silent
success.

## System dependencies

Exact Termux command:

    pkg install rust git

The Termux package name is `rust`. It installs `rustc` and `cargo` into
`$PREFIX/bin`. There are no `rustc` or `cargo` packages in
termux-main. `git` is a separate package.

No other system libraries for this crate. No OpenSSL link. No proot.
No sudo. rustup is not used and must not be installed by this project:
the stable channel has no host binaries for `aarch64-linux-android`.

`rust-toolchain.toml` is a pin for hosts that already have rustup
(GitHub Actions). Termux `pkg` rust ignores that file.

If the installed `rustc` is older than 1.87, that is an unresolved
device limitation — do not emit rustup instructions.

## Termux host facts (investigation 2026-09-15)

FACT (pool): termux-main currently ships
`rust_1.98.1_aarch64.deb` (packages.termux.dev pool listing dated
2026-09-04). Edition 2024 is valid on that compiler (edition 2024
stabilized in 1.85.0).

FACT (recipe): termux-packages `packages/rust` is one package plus
std/docs/src subpackages. Separate main-pool packages observed:
`rust-analyzer`. No separate `clippy` or `rustfmt` packages in the
r/ index listing.

FACT (host): Termux prefix is
`/data/data/com.termux/files/usr`. Shell is
`/data/data/com.termux/files/usr/bin/bash`. Unprivileged Android
app user. Writable paths: `$HOME`, `$TMPDIR`, `$PREFIX` only where
pkg owns files.

FACT (sandbox): this emission host is rustc 1.75.0 and cannot
generate an edition-2024 Cargo.lock.

FACT (not this crate): HSE repair script
`$HOME/huntsman/hse-fix-all.sh` targets a different binary (`hse`,
port 8080, `$HOME/.huntsman.env`). D9 still forbids absorbing that
tree.

INFERENCE: a current Termux `pkg upgrade && pkg install rust` is
expected to satisfy MSRV 1.87. Not yet observed on the operator
handset.

ASSUMPTION still open: the operator device actually has rust 1.98.1
installed. Confirm with `rustc --version` on device.

## Language and quality contract (applied)

- Rust stable 1.87+, edition 2024, resolver 3
- #![deny(unsafe_code)] in lib and bins
- #![warn(clippy::pedantic)] ; warnings fail the build
- Public types: Debug
- Serializable types: Serialize, Deserialize
- Library errors: thiserror 2.x
- Binary errors: anyhow 1.x
- CLI: clap 4.x derive
- No async / no tokio / no reqwest in v1 (no external I/O that needs them)
- Secrets and paths only through src/config.rs
- Every filesystem call has a size bound and a typed error path
- Header comment on every source file stating purpose
- Runtime still requires no network. GitHub is distribution, not a
  runtime dependency.

## CLI surface (v1, assumed — see A1 / A4)

    huntsman new [--title TEXT]
    huntsman list
    huntsman show [SESSION]
    huntsman recover --session ID  (flags for recover fields)
    huntsman compete --session ID
    huntsman falsify --session ID
    huntsman execute --session ID
    huntsman verify --session ID
    huntsman repair --session ID
    huntsman reproduce --session ID
    huntsman reattack --session ID
    huntsman simplify --session ID
    huntsman terminate --session ID
    huntsman path [SESSION]     print store path (debug)

SESSION defaults to the most recently touched session in var/.

A session is one JSON document. Stages may be revisited. terminate is
allowed only when required recover fields and at least one candidate,
one falsification record, and one verify record exist — or when the
operator passes --partial and must name residual uncertainty.

The tool never fail-opens a missing session as success.

## Decisions (accepted)

D1. Product is a recorder and rule-checker, not an agent.
    Rejected: LLM loop, HTTP API, background daemon.

D2. Single package crate huntsman.
    Rejected: cargo workspace, huntsman-core + huntsman-cli split
    (no shared consumers yet; complexity unjustified).

D3. Storage = pretty JSON files under $HOME/huntsman/var/sessions/.
    Rejected: SQLite (extra native dep), git notes, markdown-only
    (harder to validate). File names: <utc>-<shortid>.json plus
    a pointer file var/current.txt holding the id. No symlinks
    required.

D4. No tokio, no reqwest, no async in v1.
    Rejected: following the language-contract HTTP/async rows as
    mandatory dependencies. Those rows bind when the concern exists.
    Adding unused async runtime fails SIMPLIFY.

D5. No unsafe, no FFI, no bundled C.
    Rejected: any crate that needs C toolchain beyond rustc.

D6. Config is optional TOML plus HUNTSMAN_* env, read only in
    src/config.rs. Default root = $HOME/huntsman.

D7. Evidence ladder and stage names are enums in source, not strings
    the operator may misspell without error.

D8. Tests are integration tests on a temp dir ($TMPDIR), not the
    real HOME store.

D9. Attached HSE / query-pack / handoff zip archives are evidence
    of a different existing product (Huntsman Search Engine). They
    are not this crate's source tree and are not copied into
    INSTALL_ROOT unless A1 is overturned.

D10. Host on GitHub at EmmmmDeee/huntsman-rcvf, public, branch main.
     Rejected: commit into the HSE search-engine repo (wrong product).
     Rejected: overwrite EmmmmDeee/rcvf (that repo is the spec and
     operator checklist, not this crate).
     Rejected: reuse empty placeholder EmmmmDeee/Huntsman- (name
     collides with HSE branding; trailing hyphen).
     Rejected: private default without an operator secrecy
     requirement.

D11. Termux system package is `rust`, command `pkg install rust git`.
     Rejected: `pkg install rustc cargo` (those package names do not
     exist in termux-main; pkg will tell the operator to install
     `rust`).

## Rejected alternatives (summary)

- Markdown templates with no binary: not software under this contract.
- Interactive TUI: extra crates, unclear aarch64 terminal assumptions.
- Server mode / bind port: unnecessary attack surface on a phone.
- rustup install of 1.87: uncommitted toolchain; forbidden; also
  cannot host on aarch64-linux-android.
- Hand-written Cargo.lock without cargo: false pin.
- Absorbing HSE query-pack into this crate: different binary, HTTP,
  credentials, and provider policy; violates D1, D4, D5, SIMPLIFY.
- GitHub as a runtime store for sessions: would require network and
  credentials at run time; violates the offline Termux contract.
- TUR `rustc-nightly`: extra repo, extra path, unjustified for MSRV.

## Flagged ambiguities

A1. Product identity is the strongest way this plan could be wrong.
    FACT: the operator pasted the RCVF loop and the build contract.
    FACT: HSE source and handoff zips were attached in the same
    project.
    FACT: operator then required GitHub hosting.
    ASSUMPTION in force: this emission is the RCVF recorder hosted at
    EmmmmDeee/huntsman-rcvf, not a patch of HSE `hse query-pack`.
    If the intended host is the HSE repo instead, say so and this
    MANIFEST must be revised before further pushes.

A2. Operator-device rustc is still unobserved. Current termux-main
    pool rust is 1.98.1 aarch64 (2026-09-04), which would satisfy
    MSRV. This sandbox rustc is 1.75.0 and cannot generate a valid
    edition-2024 lockfile. Cargo.lock may remain unemitted until a
    1.87+ host runs `cargo generate-lockfile`. Clippy-on-device is
    unverified (no separate clippy package in the r/ index).

A3. clap 4 / serde 1 / thiserror 2 / anyhow 1 are pure Rust and
    treated as aarch64-safe. Config TOML is a hand parser, not the
    `toml` crate. If a later file needs any other crate, stop and
    resolve aarch64 before emitting that file.

A4. Session identity UX (short id vs title vs path) is assumed as
    above. Not specified by the operator.

A5. Whether huntsman must export markdown reports is unspecified.
    v1 JSON only.

A6. Multi-session merge / import / sync is unspecified. v1 local
    files only.

A7. "Every external call has an explicit timeout" — portable
    wall-clock IO timeouts need an async runtime. v1 uses std::fs
    with a size cap and typed errors instead. Flagged deviation
    from the letter of the quality clause; no silent failure.

A8. Visibility is public. Operator did not request private.

A9. LICENSE file text was not chosen beyond Cargo.toml
    `MIT OR Apache-2.0`. No LICENSE file until the operator names one.

## Assumptions (material)

- Operator can run `pkg install rust git` and `cargo build --release`.
- $HOME is writable. $HOME/huntsman/var may be created by the binary.
- One operator, one device, no concurrent writers to the same session.
- RCVF stage semantics follow the operator paste and EmmmmDeee/rcvf
  SPEC.md (Recover through Terminate, evidence ladder, terminate
  rules).
- English CLI help text is acceptable.
- HSE zip contents are not required to compile or run v1.
- GitHub hosting does not change the offline runtime contract.

## Sanctioned stubs

None. No todo!(), no unimplemented!(), no ignored tests.

## Success criteria (observable)

- Fresh Termux user pastes each installer block and obtains the file.
- `git clone https://github.com/EmmmmDeee/huntsman-rcvf.git` retrieves
  the same tree.
- cargo build and cargo test succeed on rustc 1.87+ edition 2024.
- huntsman new creates a JSON session under var/sessions/.
- A session missing recover fields is rejected by terminate without
  --partial.
- Missing paths fail closed with a typed error.
- No network is required at runtime.

## Residual uncertainty (this manifest)

A1 (recorder vs HSE) remains open.
A2 (device rustc version) narrowed to pool 1.98.1, still unobserved
on the handset.
A8 (public vs private) assumed public.
A9 (LICENSE file) unset.

If A1 is overturned, revise this MANIFEST.md before further GitHub
pushes into huntsman-rcvf or HSE.

Stop. Wait for continue, chain, revise, or a spec correction.
