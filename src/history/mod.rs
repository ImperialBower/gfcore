//! Game history recording and YAML/JSON serialization.
//!
//! Requires the `history` feature (enabled by default).
//!
//! # Overview
//!
//! - [`TurnRecord`] — one player's turn: events emitted, book counts, and
//!   optionally the actions taken (enables replay).
//! - [`GameRecord`] — full game record with UUID, timestamp, players, turns, winner.
//! - [`GameCollection`] — versioned, ordered list of [`GameRecord`]s with
//!   round-trip serialization and file-save support.
//! - [`AuditResult`] — structural invariant check result from [`GameRecord::audit`].
//! - [`ReplayResult`] — engine-replay check result from [`GameRecord::replay`].
//!
//! # Examples
//!
//! ```
//! use gfcore::history::{GameCollection, GameRecord, TurnRecord};
//!
//! let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
//! let yaml = record.to_yaml().expect("serialize");
//! let parsed = GameRecord::from_yaml(&yaml).expect("deserialize");
//! assert_eq!(record, parsed);
//!
//! let mut col = GameCollection::new();
//! col.push(record);
//! assert_eq!(col.len(), 1);
//!
//! let results = col.audit_all();
//! assert!(results[0].is_consistent);
//! ```

pub mod audit;
pub mod record;
pub mod replay;

pub use audit::AuditResult;
pub use record::{FORMAT_VERSION, GameCollection, GameRecord, TurnRecord};
pub use replay::ReplayResult;
