//! [`TurnRecord`], [`GameRecord`], and [`GameCollection`] — the core history types.
//!
//! Build a [`GameRecord`] while playing by appending [`TurnRecord`]s, then
//! call [`GameRecord::to_yaml`] or [`GameRecord::to_json`] to persist it.
//! Load it back with [`GameRecord::from_yaml`] or [`GameRecord::from_json`].

use cardpack::prelude::BasicPile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::GfError;
use crate::game::{GameEvent, PlayerAction};

/// The serialization format version written into every new [`GameCollection`].
pub const FORMAT_VERSION: u32 = 1;

fn default_gfcore_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_format_version() -> u32 {
    FORMAT_VERSION
}

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
///     actions: None,
/// };
/// assert_eq!(turn.player, 0);
/// assert!(turn.events.is_empty());
/// assert!(turn.actions.is_none());
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
    /// Actions submitted by the player during this turn, in order.
    ///
    /// `None` if this record was created without action recording (e.g., WASM
    /// games or records pre-dating this feature). `Some(...)` enables
    /// [`GameRecord::replay`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<PlayerAction>>,
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
/// // id is a fresh UUID v4; the timestamp defaults to "0" — inject a real one
/// // with `with_timestamp` from code that owns a clock.
/// assert!(!record.id.is_empty());
/// assert_eq!(record.timestamp, "0");
/// assert_eq!(record.with_timestamp(1_700_000_000).timestamp, "1700000000");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    /// UUID v4 string uniquely identifying this game.
    pub id: String,
    /// Variant name, e.g. `"Standard"`.
    pub variant: String,
    /// Unix epoch seconds as a string. Defaults to `"0"`; set a real value via
    /// [`GameRecord::with_timestamp`] (the kernel keeps no clock of its own).
    pub timestamp: String,
    /// Display names of all players, in turn order.
    pub players: Vec<String>,
    /// Ordered list of completed turns.
    pub turns: Vec<TurnRecord>,
    /// Index of the winning player once the game is over, or `None` for a tie.
    pub winner: Option<usize>,
    /// The full draw pile before the initial deal, enabling deterministic replay.
    /// `None` for records created without the replay path (e.g. WASM, old unit tests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_draw_pile: Option<BasicPile>,
}

impl GameRecord {
    /// Creates a new [`GameRecord`] with a fresh UUID and an unset timestamp.
    ///
    /// The record reads no clock of its own — a kernel is deterministic — so the
    /// timestamp defaults to `"0"`. To stamp a real time, chain
    /// [`GameRecord::with_timestamp`] from the delivery layer that owns a clock.
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
    /// assert_eq!(record.timestamp, "0");
    /// ```
    #[must_use]
    pub fn new(variant: impl Into<String>, players: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            variant: variant.into(),
            timestamp: "0".to_string(),
            players,
            turns: Vec::new(),
            winner: None,
            initial_draw_pile: None,
        }
    }

    /// Returns this record with its timestamp set to `timestamp` (Unix epoch
    /// seconds).
    ///
    /// Time is injected rather than read from a clock, keeping the kernel
    /// deterministic; call this from the delivery layer (CLI, service, harness)
    /// that owns a clock.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string()])
    ///     .with_timestamp(1_700_000_000);
    /// assert_eq!(record.timestamp, "1700000000");
    /// ```
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp.to_string();
        self
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

/// An ordered, versioned collection of [`GameRecord`]s.
///
/// Serializes as a YAML/JSON object with `gfcore_version`, `format_version`,
/// and `games` keys.
///
/// # Examples
///
/// ```
/// use gfcore::history::{GameCollection, GameRecord, FORMAT_VERSION};
///
/// let mut col = GameCollection::new();
/// assert!(col.is_empty());
/// assert_eq!(col.format_version, FORMAT_VERSION);
/// col.push(GameRecord::new("Standard", vec!["Alice".to_string()]));
/// assert_eq!(col.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameCollection {
    /// The `gfcore` crate version that created this collection (baked in at compile time).
    #[serde(default = "default_gfcore_version")]
    pub gfcore_version: String,
    /// The serialization format version. Always [`FORMAT_VERSION`] for newly created collections.
    #[serde(default = "default_format_version")]
    pub format_version: u32,
    /// The game records in this collection, in insertion order.
    pub games: Vec<GameRecord>,
}

impl GameCollection {
    /// Creates an empty [`GameCollection`] with the current crate version and
    /// [`FORMAT_VERSION`] set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, FORMAT_VERSION};
    ///
    /// let col = GameCollection::new();
    /// assert!(col.is_empty());
    /// assert_eq!(col.format_version, FORMAT_VERSION);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            gfcore_version: env!("CARGO_PKG_VERSION").to_string(),
            format_version: FORMAT_VERSION,
            games: Vec::new(),
        }
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
        self.games.push(record);
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
        self.games.len()
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
        self.games.is_empty()
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
        self.games.iter()
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

    // Filesystem persistence (`save`/`save_to`) lives in the `history::persist`
    // adapter (feature `persistence`), keeping this kernel type free of I/O.
}

impl Default for GameCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for GameCollection {
    type Output = GameRecord;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.games[idx]
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
    fn test_game_record_new_has_unset_timestamp() {
        let r = make_record();
        // The kernel reads no clock; the timestamp is unset until injected.
        assert_eq!(r.timestamp, "0");
    }

    #[test]
    fn test_game_record_with_timestamp_injects_value() {
        let r = make_record().with_timestamp(1_700_000_000);
        assert_eq!(r.timestamp, "1700000000");
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
            actions: None,
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

    #[test]
    fn test_turn_record_actions_default_is_none() {
        let turn = TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: None,
        };
        assert!(turn.actions.is_none());
    }

    #[test]
    fn test_turn_record_with_actions_yaml_round_trip() {
        use crate::game::PlayerAction;
        use cardpack::prelude::{DeckedBase, Standard52};
        let rank = Standard52::basic_pile().v()[0].rank;
        let turn = TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: Some(vec![
                PlayerAction::Ask { target: 1, rank },
                PlayerAction::Draw,
            ]),
        };
        let yaml = serde_norway::to_string(&turn).unwrap();
        let back: TurnRecord = serde_norway::from_str(&yaml).unwrap();
        assert_eq!(turn, back);
    }

    #[test]
    fn test_turn_record_none_actions_omitted_from_yaml() {
        let turn = TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: None,
        };
        let yaml = serde_norway::to_string(&turn).unwrap();
        assert!(!yaml.contains("actions"));
    }

    #[test]
    fn test_game_collection_has_format_version() {
        let col = GameCollection::new();
        assert_eq!(col.format_version, FORMAT_VERSION);
    }

    #[test]
    fn test_game_collection_has_gfcore_version() {
        let col = GameCollection::new();
        assert!(!col.gfcore_version.is_empty());
    }

    #[test]
    fn test_game_collection_yaml_contains_version_fields() {
        let col = GameCollection::new();
        let yaml = col.to_yaml().unwrap();
        assert!(yaml.contains("format_version"));
        assert!(yaml.contains("gfcore_version"));
        assert!(yaml.contains("games"));
    }
}
