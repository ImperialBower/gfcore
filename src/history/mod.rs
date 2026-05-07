//! Game history recording and YAML/JSON serialization.
//!
//! Requires the `history` feature (enabled by default).
//!
//! # Overview
//!
//! - [`TurnRecord`] — one player's turn: events emitted and book counts after.
//! - [`GameRecord`] — full game record with UUID, timestamp, players, turns, winner.
//! - [`GameCollection`] — ordered list of [`GameRecord`]s with round-trip serialization.
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
//! ```

pub mod record;

pub use record::{GameCollection, GameRecord, TurnRecord};
