//! Offline case core plus two verified public parsers.
//! Live fetch is a single curl lookup. Challenge pages are not results.
//! A technique id enters Navigator only from an in-crate binding. The table is empty.

#![deny(unsafe_code)]

pub mod case;
pub mod classify;
pub mod error;
pub mod external;
pub mod geoint;
pub mod identity;
pub mod ledger;
pub mod navigator;
pub mod search;
pub mod session;
pub mod sha256;
pub mod stage;
pub mod stix;
pub mod store;

pub use error::Error;
pub use ledger::{admitted, append, chain_intact, load_chain, save_chain, seal, Claim, LedgerEntry};
pub use session::Session;
pub use stage::{EvidenceLevel, Status};
