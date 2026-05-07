//! [`TurnRecord`], [`GameRecord`], and [`GameCollection`] — the core history types.
//!
//! Build a [`GameRecord`] while playing by appending [`TurnRecord`]s, then
//! call [`GameRecord::to_yaml`] or [`GameRecord::to_json`] to persist it.
//! Load it back with [`GameRecord::from_yaml`] or [`GameRecord::from_json`].

use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::GfError;
use crate::game::GameEvent;

// ---------------------------------------------------------------------------
// TurnRecord
// ---------------------------------------------------------------------------

/// A record of a single player's turn: all events emitted and book counts
/// after the turn ends.
///
/// # Examples
///
/// ```
/// use gfcore::history::TurnRecord;
///
/// let turn = TurnRecord {
///     player: 0,
///     events: vec![],
///     books_after_turn: vec![0, 0],
/// };
/// assert_eq!(turn.player, 0);
/// assert!(turn.events.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRecord {
    /// Index of the player who took this turn.
    pub player: usize,
    /// All events emitted during this turn, in order.
    pub events: Vec<GameEvent>,
    /// Book counts per player after this turn completes.
    /// Index matches the player index.
    pub books_after_turn: Vec<usize>,
}

// ---------------------------------------------------------------------------
// GameRecord
// ---------------------------------------------------------------------------

/// A complete record of a finished (or in-progress) game.
///
/// Created with [`GameRecord::new`]; build it up by pushing [`TurnRecord`]s
/// and setting `winner` once the game ends.  Serialize to YAML or JSON with
/// [`GameRecord::to_yaml`] / [`GameRecord::to_json`]; deserialize with the
/// corresponding `from_*` methods.
///
/// # Examples
///
/// ```
/// use gfcore::history::GameRecord;
///
/// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
/// assert_eq!(record.variant, "Standard");
/// assert_eq!(record.players, vec!["Alice", "Bob"]);
/// assert!(record.turns.is_empty());
/// assert!(record.winner.is_none());
/// // id is a UUID v4 string; timestamp is a Unix epoch seconds string.
/// assert!(!record.id.is_empty());
/// assert!(!record.timestamp.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    /// UUID v4 string uniquely identifying this game.
    pub id: String,
    /// Variant name, e.g. `"Standard"`.
    pub variant: String,
    /// Unix epoch seconds as a string (set at record creation time).
    pub timestamp: String,
    /// Display names of all players, in turn order.
    pub players: Vec<String>,
    /// Ordered list of completed turns.
    pub turns: Vec<TurnRecord>,
    /// Index of the winning player once the game is over, or `None` for a tie.
    pub winner: Option<usize>,
}

impl GameRecord {
    /// Creates a new [`GameRecord`] with a fresh UUID and the current timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// assert_eq!(record.variant, "Standard");
    /// assert_eq!(record.players.len(), 2);
    /// assert!(record.turns.is_empty());
    /// assert!(record.winner.is_none());
    /// ```
    #[must_use]
    pub fn new(variant: impl Into<String>, players: Vec<String>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        #[cfg(target_arch = "wasm32")]
        let ts = "0".to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            variant: variant.into(),
            timestamp: ts,
            players,
            turns: Vec::new(),
            winner: None,
        }
    }

    /// Serializes this record to a YAML string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let yaml = record.to_yaml().expect("serialization must succeed");
    /// assert!(yaml.contains("Standard"));
    /// assert!(yaml.contains("Alice"));
    /// ```
    pub fn to_yaml(&self) -> Result<String, GfError> {
        serde_norway::to_string(self).map_err(GfError::from)
    }

    /// Deserializes a [`GameRecord`] from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if the input is not valid YAML or does
    /// not match the expected structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let yaml = record.to_yaml().expect("serialize");
    /// let parsed = GameRecord::from_yaml(&yaml).expect("deserialize");
    /// assert_eq!(record, parsed);
    /// ```
    pub fn from_yaml(s: &str) -> Result<Self, GfError> {
        serde_norway::from_str(s).map_err(GfError::from)
    }

    /// Serializes this record to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let json = record.to_json().expect("serialization must succeed");
    /// assert!(json.contains("Standard"));
    /// assert!(json.contains("Alice"));
    /// ```
    pub fn to_json(&self) -> Result<String, GfError> {
        serde_json::to_string(self).map_err(GfError::from)
    }

    /// Deserializes a [`GameRecord`] from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if the input is not valid JSON or does
    /// not match the expected structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let json = record.to_json().expect("serialize");
    /// let parsed = GameRecord::from_json(&json).expect("deserialize");
    /// assert_eq!(record, parsed);
    /// ```
    pub fn from_json(s: &str) -> Result<Self, GfError> {
        serde_json::from_str(s).map_err(GfError::from)
    }
}

// ---------------------------------------------------------------------------
// GameCollection
// ---------------------------------------------------------------------------

/// An ordered collection of [`GameRecord`]s.
///
/// Serializes as a YAML/JSON sequence of records, not a wrapped object.
///
/// # Examples
///
/// ```
/// use gfcore::history::{GameCollection, GameRecord};
///
/// let mut col = GameCollection::new();
/// assert!(col.is_empty());
/// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
/// assert_eq!(col.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GameCollection(Vec<GameRecord>);

impl GameCollection {
    /// Creates an empty [`GameCollection`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameCollection;
    ///
    /// let col = GameCollection::new();
    /// assert!(col.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Appends a [`GameRecord`] to the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// assert_eq!(col.len(), 1);
    /// ```
    pub fn push(&mut self, record: GameRecord) {
        self.0.push(record);
    }

    /// Returns the number of records in the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameCollection;
    ///
    /// let col = GameCollection::new();
    /// assert_eq!(col.len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the collection contains no records.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameCollection;
    ///
    /// let col = GameCollection::new();
    /// assert!(col.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the records in this collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// assert_eq!(col.iter().count(), 1);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &GameRecord> {
        self.0.iter()
    }

    /// Serializes this collection to a YAML string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// let yaml = col.to_yaml().expect("serialize");
    /// assert!(!yaml.is_empty());
    /// ```
    pub fn to_yaml(&self) -> Result<String, GfError> {
        serde_norway::to_string(self).map_err(GfError::from)
    }

    /// Deserializes a [`GameCollection`] from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if the input is not valid YAML or does
    /// not match the expected structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// let yaml = col.to_yaml().expect("serialize");
    /// let parsed = GameCollection::from_yaml(&yaml).expect("deserialize");
    /// assert_eq!(col, parsed);
    /// ```
    pub fn from_yaml(s: &str) -> Result<Self, GfError> {
        serde_norway::from_str(s).map_err(GfError::from)
    }

    /// Serializes this collection to a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// let json = col.to_json().expect("serialize");
    /// assert!(!json.is_empty());
    /// ```
    pub fn to_json(&self) -> Result<String, GfError> {
        serde_json::to_string(self).map_err(GfError::from)
    }

    /// Deserializes a [`GameCollection`] from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::ParseError`] if the input is not valid JSON or does
    /// not match the expected structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
    /// let json = col.to_json().expect("serialize");
    /// let parsed = GameCollection::from_json(&json).expect("deserialize");
    /// assert_eq!(col, parsed);
    /// ```
    pub fn from_json(s: &str) -> Result<Self, GfError> {
        serde_json::from_str(s).map_err(GfError::from)
    }
}

impl std::ops::Index<usize> for GameCollection {
    type Output = GameRecord;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameEvent;

    fn make_record() -> GameRecord {
        GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()])
    }

    #[test]
    fn test_game_record_new_has_uuid() {
        let r = make_record();
        // UUID v4 strings are 36 chars: xxxxxxxx-xxxx-4xxx-xxxx-xxxxxxxxxxxx
        assert_eq!(r.id.len(), 36);
    }

    #[test]
    fn test_game_record_new_has_timestamp() {
        let r = make_record();
        let ts: u64 = r.timestamp.parse().unwrap();
        // timestamp must be a plausible Unix epoch (after year 2020)
        assert!(ts > 1_600_000_000);
    }

    #[test]
    fn test_game_record_new_players_and_variant() {
        let r = make_record();
        assert_eq!(r.variant, "Standard");
        assert_eq!(r.players, ["Alice", "Bob"]);
        assert!(r.turns.is_empty());
        assert!(r.winner.is_none());
    }

    #[test]
    fn test_game_record_yaml_round_trip() {
        let r = make_record();
        let yaml = r.to_yaml().unwrap();
        let back = GameRecord::from_yaml(&yaml).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn test_game_record_json_round_trip() {
        let r = make_record();
        let json = r.to_json().unwrap();
        let back = GameRecord::from_json(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn test_game_record_with_turns_round_trip() {
        let mut r = make_record();
        let turn = TurnRecord {
            player: 0,
            events: vec![
                GameEvent::Asked {
                    asker: 0,
                    target: 1,
                    rank: "A".to_string(),
                },
                GameEvent::GoFish { player: 0 },
                GameEvent::Drew {
                    player: 0,
                    matched: false,
                },
            ],
            books_after_turn: vec![0, 0],
        };
        r.turns.push(turn);
        r.winner = Some(0);

        let yaml = r.to_yaml().unwrap();
        let back = GameRecord::from_yaml(&yaml).unwrap();
        assert_eq!(r, back);

        let json = r.to_json().unwrap();
        let back_json = GameRecord::from_json(&json).unwrap();
        assert_eq!(r, back_json);
    }

    #[test]
    fn test_game_record_from_yaml_bad_input_returns_error() {
        let result = GameRecord::from_yaml("not: valid: yaml: [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_game_record_from_json_bad_input_returns_error() {
        let result = GameRecord::from_json("{not json}");
        assert!(result.is_err());
    }

    #[test]
    fn test_game_collection_new_is_empty() {
        let col = GameCollection::new();
        assert!(col.is_empty());
        assert_eq!(col.len(), 0);
    }

    #[test]
    fn test_game_collection_push_and_len() {
        let mut col = GameCollection::new();
        col.push(make_record());
        assert_eq!(col.len(), 1);
        assert!(!col.is_empty());
        col.push(make_record());
        assert_eq!(col.len(), 2);
    }

    #[test]
    fn test_game_collection_yaml_round_trip() {
        let mut col = GameCollection::new();
        col.push(make_record());
        col.push(make_record());
        let yaml = col.to_yaml().unwrap();
        let back = GameCollection::from_yaml(&yaml).unwrap();
        assert_eq!(col, back);
    }

    #[test]
    fn test_game_collection_json_round_trip() {
        let mut col = GameCollection::new();
        col.push(make_record());
        let json = col.to_json().unwrap();
        let back = GameCollection::from_json(&json).unwrap();
        assert_eq!(col, back);
    }

    #[test]
    fn test_game_collection_empty_round_trip() {
        let col = GameCollection::new();
        let yaml = col.to_yaml().unwrap();
        let back = GameCollection::from_yaml(&yaml).unwrap();
        assert_eq!(col, back);
    }
}
