# Game History Audit & Replay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structural audit (`GameRecord::audit()`) and engine-replay (`GameRecord::replay()`) capabilities to `gfcore`'s history system, alongside a versioned `GameCollection` struct with file-save support and per-turn action recording in the engine.

**Architecture:** Split `src/history/record.rs` into three focused files: `record.rs` (data types + I/O), `audit.rs` (structural invariant checks), `replay.rs` (engine re-run). The engine (`state.rs`) gains a `pending_turn_actions` accumulator alongside the existing `pending_turn_events`. All new API is native-only; WASM stays unchanged.

**Tech Stack:** Rust 2021 edition; serde + serde_norway + serde_json (existing); uuid (existing); `std::fs` for save (native-only, `cfg`-gated).

---

### Task 1: GfError — add IoError and NoReplayData variants

**Files:**
- Modify: `src/error/mod.rs`

- [ ] **Step 1: Add failing tests**

In the `#[cfg(test)]` block at the bottom of `src/error/mod.rs`, add after the existing tests:

```rust
#[test]
fn test_io_error_display() {
    let err = GfError::IoError("permission denied".to_string());
    assert_eq!(err.to_string(), "io error: permission denied");
}

#[test]
fn test_no_replay_data_display() {
    let err = GfError::NoReplayData;
    assert_eq!(
        err.to_string(),
        "no replay data: at least one turn has no stored actions",
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --lib -- error 2>&1 | tail -20
```

Expected: compile error — `GfError::IoError` and `GfError::NoReplayData` do not exist.

- [ ] **Step 3: Add variants to GfError**

In `src/error/mod.rs`, add two variants before the closing `}` of the `GfError` enum, after `ParseError`:

```rust
/// A filesystem operation failed during save.
///
/// The inner `String` contains the OS error message.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::GfError;
///
/// let err = GfError::IoError("permission denied".to_string());
/// assert_eq!(err.to_string(), "io error: permission denied");
/// ```
IoError(String),

/// `replay()` was called on a record where at least one turn has no stored actions.
///
/// This error is expected when replaying records produced before action
/// recording was added (e.g., WASM games or old tests).
///
/// # Examples
///
/// ```
/// use gfcore::prelude::GfError;
///
/// let err = GfError::NoReplayData;
/// assert_eq!(
///     err.to_string(),
///     "no replay data: at least one turn has no stored actions",
/// );
/// ```
NoReplayData,
```

- [ ] **Step 4: Update Display**

In the `fmt::Display` impl, add arms after `Self::ParseError(msg)`:

```rust
Self::IoError(msg) => write!(f, "io error: {msg}"),
Self::NoReplayData => {
    f.write_str("no replay data: at least one turn has no stored actions")
}
```

- [ ] **Step 5: Update existing display coverage test**

The `test_display_messages_are_non_empty` test iterates a hardcoded array. Extend it to include the two new variants:

```rust
let variants = [
    GfError::InvalidAsk,
    GfError::InvalidTarget,
    GfError::OutOfTurn,
    GfError::NotEnoughPlayers,
    GfError::TooManyPlayers,
    GfError::GameAlreadyOver,
    GfError::EmptyDrawPile,
    GfError::ParseError("bad".into()),
    GfError::IoError("disk full".into()),
    GfError::NoReplayData,
];
```

- [ ] **Step 6: Run tests and clippy**

```bash
cargo test --lib -- error && cargo clippy --all-features -- -D warnings
```

Expected: all error tests pass, no warnings.

- [ ] **Step 7: Commit**

```
git add src/error/mod.rs
git commit -m "feat(error): add IoError and NoReplayData variants to GfError"
```

---

### Task 2: TurnRecord — add optional actions field

**Files:**
- Modify: `src/history/record.rs`
- Modify: `tests/history_integration.rs` (compile fix — TurnRecord struct literals)

- [ ] **Step 1: Add failing tests**

In the `#[cfg(test)]` block in `src/history/record.rs`, add:

```rust
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
```

- [ ] **Step 2: Run to verify compile failure**

```bash
cargo test --lib -- record 2>&1 | tail -10
```

Expected: compile error — struct literal missing `actions` field, or field not found.

- [ ] **Step 3: Update the import in record.rs**

Replace:

```rust
use crate::game::GameEvent;
```

with:

```rust
use crate::game::{GameEvent, PlayerAction};
```

- [ ] **Step 4: Add the actions field to TurnRecord**

In the `TurnRecord` struct, after `books_after_turn`:

```rust
/// Actions submitted by the player during this turn, in order.
///
/// `None` if this record was created without action recording (e.g., WASM
/// games or records pre-dating this feature). `Some(...)` enables
/// [`GameRecord::replay`].
#[serde(default, skip_serializing_if = "Option::is_none")]
pub actions: Option<Vec<PlayerAction>>,
```

- [ ] **Step 5: Update the TurnRecord doc test**

The existing doc example constructs the struct without `actions`. Add `actions: None` and an assertion:

```rust
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
```

- [ ] **Step 6: Fix TurnRecord construction in the existing record.rs unit test**

`test_game_record_with_turns_round_trip` constructs a `TurnRecord`. Add `actions: None`:

```rust
let turn = TurnRecord {
    player: 0,
    events: vec![
        GameEvent::Asked { asker: 0, target: 1, rank: "A".to_string() },
        GameEvent::GoFish { player: 0 },
        GameEvent::Drew { player: 0, matched: false },
    ],
    books_after_turn: vec![0, 0],
    actions: None,
};
```

- [ ] **Step 7: Fix TurnRecord construction in tests/history_integration.rs**

`play_and_record()` constructs two `TurnRecord`s. Add `actions: None` to both:

```rust
record.turns.push(TurnRecord {
    player: current_turn_player,
    events: std::mem::take(&mut current_turn_events),
    books_after_turn,
    actions: None,
});
```

And the flush guard at the end of `play_and_record()`:

```rust
record.turns.push(TurnRecord {
    player: current_turn_player,
    events: current_turn_events,
    books_after_turn,
    actions: None,
});
```

- [ ] **Step 8: Run all tests and clippy**

```bash
cargo test --all-features && cargo clippy --all-features -- -D warnings
```

Expected: all tests pass, no warnings. Round-trips work for both `None` (field omitted) and `Some(...)`.

- [ ] **Step 9: Commit**

```
git add src/history/record.rs tests/history_integration.rs
git commit -m "feat(history): add optional actions field to TurnRecord"
```

---

### Task 3: GameCollection — versioned struct, FORMAT\_VERSION, save()/save\_to()

**Files:**
- Modify: `src/history/record.rs`

This is the largest task. The newtype `struct GameCollection(Vec<GameRecord>)` becomes a named struct with metadata.

- [ ] **Step 1: Add failing tests**

In `record.rs` tests, add:

```rust
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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_game_collection_save_to_temp_dir() {
    let mut col = GameCollection::new();
    col.push(make_record());
    let path = std::env::temp_dir()
        .join("gfcore_test_save_to.yaml")
        .to_string_lossy()
        .to_string();
    let result = col.save_to(&path);
    assert!(result.is_ok(), "save_to failed: {:?}", result);
    assert!(std::path::Path::new(&path).exists());
    let yaml = std::fs::read_to_string(&path).unwrap();
    let loaded = GameCollection::from_yaml(&yaml).unwrap();
    assert_eq!(col, loaded);
    let _ = std::fs::remove_file(&path);
}
```

- [ ] **Step 2: Run to verify compile failure**

```bash
cargo test --lib -- record 2>&1 | tail -10
```

Expected: errors — `FORMAT_VERSION`, `col.format_version`, `col.gfcore_version`, `save_to` undefined.

- [ ] **Step 3: Add FORMAT\_VERSION and serde helper functions**

After the existing imports at the top of `src/history/record.rs`, add:

```rust
/// The serialization format version written into every new [`GameCollection`].
pub const FORMAT_VERSION: u32 = 1;

fn default_gfcore_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_format_version() -> u32 {
    FORMAT_VERSION
}
```

- [ ] **Step 4: Replace the GameCollection newtype with a versioned struct**

Remove the old definition:

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GameCollection(Vec<GameRecord>);
```

Replace with:

```rust
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
```

- [ ] **Step 5: Update all GameCollection impl methods**

Replace every method that accessed `self.0` to use `self.games`. Replace `new()` to initialize all three fields:

`new()`:
```rust
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
```

`push()`, `len()`, `is_empty()`, `iter()` — replace `self.0` with `self.games` in each body.

`Index<usize>` impl:
```rust
impl std::ops::Index<usize> for GameCollection {
    type Output = GameRecord;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.games[idx]
    }
}
```

Add `Default` impl (the struct cannot derive it because `gfcore_version` needs a runtime call):
```rust
impl Default for GameCollection {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 6: Add save() and save\_to() methods**

Add to the `impl GameCollection` block, below `from_json`:

```rust
/// Writes this collection to `generated/<run_name>_<unix_ts>.yaml`.
///
/// Creates the `generated/` directory if it does not already exist.
/// Returns the path written on success.
///
/// # Errors
///
/// - [`GfError::IoError`] — directory creation or file write failed.
/// - [`GfError::ParseError`] — YAML serialization failed.
///
/// # Examples
///
/// ```no_run
/// use gfcore::history::GameCollection;
///
/// let col = GameCollection::new();
/// let path = col.save("my_session").expect("save must succeed");
/// assert!(path.contains("my_session"));
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn save(&self, run_name: &str) -> Result<String, GfError> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = format!("generated/{run_name}_{ts}.yaml");
    self.save_to(&path)
}

/// Writes this collection to `path`, creating parent directories as needed.
///
/// Returns `path` as a `String` on success.
///
/// # Errors
///
/// - [`GfError::IoError`] — directory creation or file write failed.
/// - [`GfError::ParseError`] — YAML serialization failed.
///
/// # Examples
///
/// ```no_run
/// use gfcore::history::GameCollection;
///
/// let col = GameCollection::new();
/// let path = col.save_to("/tmp/test_collection.yaml").expect("save must succeed");
/// assert_eq!(path, "/tmp/test_collection.yaml");
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub fn save_to(&self, path: &str) -> Result<String, GfError> {
    let yaml = self.to_yaml()?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GfError::IoError(e.to_string()))?;
        }
    }
    std::fs::write(path, &yaml).map_err(|e| GfError::IoError(e.to_string()))?;
    Ok(path.to_string())
}
```

- [ ] **Step 7: Run all tests and clippy**

```bash
cargo test --all-features && cargo clippy --all-features -- -D warnings
```

Expected: all tests pass. The three new tests (format\_version, gfcore\_version, yaml\_contains\_fields) all pass. The `save_to` test writes and reads back correctly. No warnings.

- [ ] **Step 8: Commit**

```
git add src/history/record.rs
git commit -m "feat(history): GameCollection versioned struct, FORMAT_VERSION, save()/save_to()"
```

---

### Task 4: history/mod.rs — scaffold audit and replay submodules

**Files:**
- Modify: `src/history/mod.rs`
- Create: `src/history/audit.rs` (stub)
- Create: `src/history/replay.rs` (stub)

The goal of this task is to get a clean compile and passing doc tests with stubs. The full implementations come in Tasks 5 and 7.

- [ ] **Step 1: Create stub src/history/audit.rs**

```rust
//! Structural audit of [`GameRecord`] and [`GameCollection`].

use serde::{Deserialize, Serialize};

use super::record::{GameCollection, GameRecord};

/// The result of a structural audit of a single [`GameRecord`].
///
/// Produced by [`GameRecord::audit`] and collected by [`GameCollection::audit_all`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditResult {
    /// The game ID from the audited record.
    pub game_id: String,
    /// `true` iff no violations were found.
    pub is_consistent: bool,
    /// Book counts per player as of the last recorded turn (empty if no turns).
    pub final_books: Vec<usize>,
    /// Human-readable violation descriptions. Empty when `is_consistent`.
    pub violations: Vec<String>,
}

impl GameRecord {
    /// Validates structural invariants of this record.
    ///
    /// Infallible — violations accumulate in [`AuditResult::violations`]
    /// rather than returning an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let result = record.audit();
    /// assert!(result.is_consistent);
    /// assert!(result.violations.is_empty());
    /// ```
    pub fn audit(&self) -> AuditResult {
        AuditResult {
            game_id: self.id.clone(),
            is_consistent: true,
            final_books: vec![],
            violations: vec![],
        }
    }
}

impl GameCollection {
    /// Audits every game in this collection and returns one [`AuditResult`] per game.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]));
    /// let results = col.audit_all();
    /// assert_eq!(results.len(), 1);
    /// assert!(results[0].is_consistent);
    /// ```
    pub fn audit_all(&self) -> Vec<AuditResult> {
        self.games.iter().map(GameRecord::audit).collect()
    }
}
```

- [ ] **Step 2: Create stub src/history/replay.rs**

```rust
//! Engine-replay verification for [`GameRecord`] and [`GameCollection`].

use crate::error::GfError;

use super::record::{GameCollection, GameRecord};

/// The result of replaying a [`GameRecord`] through a fresh engine instance.
///
/// Produced by [`GameRecord::replay`] and collected by [`GameCollection::replay_all`].
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    /// The game ID from the replayed record.
    pub game_id: String,
    /// `true` iff every turn's replayed book counts match the stored counts
    /// and the final winner matches.
    pub is_consistent: bool,
    /// Book counts per player as of the last replayed turn.
    pub final_books: Vec<usize>,
    /// Index of the first turn where replayed state diverged, or `None`.
    pub mismatch_at_turn: Option<usize>,
}

impl GameRecord {
    /// Re-runs stored actions through a fresh [`crate::game::Game`] engine
    /// and compares results to stored turn data.
    ///
    /// # Errors
    ///
    /// - [`GfError::NoReplayData`] — at least one turn has `actions: None`.
    /// - [`GfError::ParseError`] — the variant name is not recognised.
    /// - Engine errors propagated from [`crate::game::Game::act`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// // A record with no turns is trivially consistent.
    /// let record = GameRecord::new("Standard Go Fish", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let result = record.replay().expect("empty record replay must succeed");
    /// assert!(result.is_consistent);
    /// ```
    pub fn replay(&self) -> Result<ReplayResult, GfError> {
        // Stub — full implementation in Task 7.
        Ok(ReplayResult {
            game_id: self.id.clone(),
            is_consistent: true,
            final_books: vec![],
            mismatch_at_turn: None,
        })
    }
}

impl GameCollection {
    /// Replays every game in this collection, returning one `Result` per game.
    ///
    /// Individual failures do not abort the batch.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameCollection;
    ///
    /// let col = GameCollection::new();
    /// let results = col.replay_all();
    /// assert!(results.is_empty());
    /// ```
    pub fn replay_all(&self) -> Vec<Result<ReplayResult, GfError>> {
        self.games.iter().map(GameRecord::replay).collect()
    }
}
```

- [ ] **Step 3: Update src/history/mod.rs**

Replace the entire file contents:

```rust
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

pub mod record;
pub mod audit;
pub mod replay;

pub use record::{FORMAT_VERSION, GameCollection, GameRecord, TurnRecord};
pub use audit::AuditResult;
pub use replay::ReplayResult;
```

- [ ] **Step 4: Run all tests and clippy**

```bash
cargo test --all-features && cargo clippy --all-features -- -D warnings
```

Expected: all tests pass (stubs compile cleanly). No warnings.

- [ ] **Step 5: Commit**

```
git add src/history/mod.rs src/history/audit.rs src/history/replay.rs
git commit -m "feat(history): scaffold audit and replay submodules with stubs"
```

---

### Task 5: audit.rs — full implementation

**Files:**
- Modify: `src/history/audit.rs`

- [ ] **Step 1: Add unit tests for every violation type**

Add a `#[cfg(test)]` block at the bottom of `src/history/audit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameEvent;
    use crate::history::record::{GameCollection, TurnRecord};

    fn make_clean_record(player_count: usize, turn_count: usize) -> GameRecord {
        let players: Vec<String> = (0..player_count).map(|i| format!("P{i}")).collect();
        let mut r = GameRecord::new("Standard", players);
        for t in 0..turn_count {
            r.turns.push(TurnRecord {
                player: t % player_count,
                events: vec![GameEvent::Drew { player: t % player_count, matched: false }],
                books_after_turn: vec![0; player_count],
                actions: None,
            });
        }
        r
    }

    #[test]
    fn test_audit_clean_record_is_consistent() {
        let record = make_clean_record(2, 3);
        let result = record.audit();
        assert!(result.is_consistent);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_audit_empty_record_no_turns_is_consistent() {
        let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
        let result = record.audit();
        assert!(result.is_consistent);
        assert!(result.violations.is_empty());
        assert!(result.final_books.is_empty());
    }

    #[test]
    fn test_audit_single_player_is_violation() {
        let record = GameRecord::new("Standard", vec!["Solo".to_string()]);
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("player")));
    }

    #[test]
    fn test_audit_turn_player_out_of_range() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 5,
            events: vec![GameEvent::Drew { player: 5, matched: false }],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_books_after_turn_wrong_length() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![0],   // should be length 2
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_empty_events_in_turn() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_book_counts_decreasing_is_violation() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![2, 0],
            actions: None,
        });
        record.turns.push(TurnRecord {
            player: 1,
            events: vec![GameEvent::Drew { player: 1, matched: false }],
            books_after_turn: vec![1, 0],   // player 0 decreased
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("decreas")));
    }

    #[test]
    fn test_audit_total_books_exceeds_13() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![10, 5],   // 15 > 13
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        // The violation message includes the actual total (15).
        assert!(result.violations.iter().any(|v| v.contains("15")));
    }

    #[test]
    fn test_audit_winner_some_but_another_has_more_books() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![3, 5],
            actions: None,
        });
        record.winner = Some(0);   // player 1 has more books
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_some_but_tied() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![5, 5],
            actions: None,
        });
        record.winner = Some(0);   // tied — no unique winner
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_none_but_clear_leader() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![5, 3],
            actions: None,
        });
        record.winner = None;   // player 0 has unique max — must be declared winner
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_correct_unique_max() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![5, 3],
            actions: None,
        });
        record.winner = Some(0);
        let result = record.audit();
        assert!(result.is_consistent, "violations: {:?}", result.violations);
    }

    #[test]
    fn test_audit_winner_none_correct_when_tied() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![4, 4],
            actions: None,
        });
        record.winner = None;
        let result = record.audit();
        assert!(result.is_consistent, "violations: {:?}", result.violations);
    }

    #[test]
    fn test_audit_final_books_populated_from_last_turn() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![2, 3],
            actions: None,
        });
        record.winner = Some(1);
        let result = record.audit();
        assert_eq!(result.final_books, vec![2, 3]);
    }

    #[test]
    fn test_audit_all_consistent_collection() {
        let mut col = GameCollection::new();
        col.push(make_clean_record(2, 3));
        col.push(make_clean_record(3, 4));
        let results = col.audit_all();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_consistent));
    }
}
```

- [ ] **Step 2: Run tests to see most violation tests fail**

```bash
cargo test --lib -- audit 2>&1 | tail -30
```

Expected: the stub returns `is_consistent: true` for everything, so violation tests fail. `test_audit_clean_record_is_consistent`, `test_audit_empty_record_no_turns_is_consistent`, and `test_audit_all_consistent_collection` may pass; all others fail.

- [ ] **Step 3: Implement GameRecord::audit()**

Replace the stub body of `audit()`:

```rust
pub fn audit(&self) -> AuditResult {
    let mut violations: Vec<String> = Vec::new();
    let player_count = self.players.len();

    // Check 1: at least 2 players.
    if player_count < 2 {
        violations.push(format!(
            "player count is {player_count}; must be at least 2"
        ));
    }

    for (i, turn) in self.turns.iter().enumerate() {
        // Check 2: turn player index in range.
        if turn.player >= player_count {
            violations.push(format!(
                "turn {i}: player index {} out of range (players: {player_count})",
                turn.player
            ));
        }

        // Check 3: books_after_turn length matches player count.
        if turn.books_after_turn.len() != player_count {
            violations.push(format!(
                "turn {i}: books_after_turn length {} != player count {player_count}",
                turn.books_after_turn.len()
            ));
        }

        // Check 4: events must not be empty.
        if turn.events.is_empty() {
            violations.push(format!("turn {i}: events list is empty"));
        }
    }

    // Check 5: book counts non-decreasing per player.
    for (i, turn) in self.turns.iter().enumerate().skip(1) {
        let prev = &self.turns[i - 1].books_after_turn;
        let curr = &turn.books_after_turn;
        if prev.len() == player_count && curr.len() == player_count {
            for p in 0..player_count {
                if curr[p] < prev[p] {
                    violations.push(format!(
                        "player {p} book count decreased from {} to {} at turn {i}",
                        prev[p], curr[p]
                    ));
                }
            }
        }
    }

    // Check 6: total books <= 13 (Standard52 deck maximum).
    if let Some(last) = self.turns.last() {
        let total: usize = last.books_after_turn.iter().sum();
        if total > 13 {
            violations.push(format!(
                "total books {total} exceeds maximum of 13 (Standard52 deck yields at most 13 books)"
            ));
        }
    }

    // Check 7: winner consistent with final book counts.
    if let Some(last) = self.turns.last() {
        let books = &last.books_after_turn;
        if books.len() == player_count && player_count >= 2 {
            let max_books = *books.iter().max().unwrap_or(&0);
            let leaders: Vec<usize> = (0..player_count)
                .filter(|&p| books[p] == max_books)
                .collect();
            let unique_leader = leaders.len() == 1;

            match self.winner {
                Some(w) => {
                    if w >= player_count {
                        violations.push(format!(
                            "winner index {w} is out of range (players: {player_count})"
                        ));
                    } else if !unique_leader {
                        violations.push(format!(
                            "winner declared as player {w} but final books are tied (books: {books:?})"
                        ));
                    } else if leaders[0] != w {
                        violations.push(format!(
                            "winner declared as player {w} but player {} has more books (books: {books:?})",
                            leaders[0]
                        ));
                    }
                }
                None => {
                    if unique_leader && max_books > 0 {
                        violations.push(format!(
                            "winner is None but player {} has unique max book count of \
                             {max_books} (books: {books:?})",
                            leaders[0]
                        ));
                    }
                }
            }
        }
    }

    let final_books = self
        .turns
        .last()
        .map(|t| t.books_after_turn.clone())
        .unwrap_or_default();

    AuditResult {
        game_id: self.id.clone(),
        is_consistent: violations.is_empty(),
        final_books,
        violations,
    }
}
```

- [ ] **Step 4: Run audit tests**

```bash
cargo test --lib -- audit && cargo clippy --all-features -- -D warnings
```

Expected: all 14 audit tests pass. No warnings.

- [ ] **Step 5: Run all tests to check no regressions**

```bash
cargo test --all-features
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```
git add src/history/audit.rs
git commit -m "feat(history): implement GameRecord::audit() with full structural checks"
```

---

### Task 6: state.rs — pending\_turn\_actions accumulator

**Files:**
- Modify: `src/game/state.rs`

This task wires action recording into the engine. After this task, every `TurnRecord` produced by `game.record()` will have `actions: Some(...)`.

- [ ] **Step 1: Run existing tests as baseline**

```bash
cargo test --all-features 2>&1 | tail -5
```

Expected: all pass.

- [ ] **Step 2: Add pending\_turn\_actions field to Game struct**

In the `Game` struct, after `pending_turn_events`:

```rust
/// Actions submitted during the current turn, flushed into [`TurnRecord::actions`].
#[cfg(feature = "history")]
pending_turn_actions: Vec<PlayerAction>,
```

- [ ] **Step 3: Initialize in Game::new()**

In the `let mut game = Self { ... }` initializer, after `pending_turn_events: Vec::new()`:

```rust
#[cfg(feature = "history")]
pending_turn_actions: Vec::new(),
```

- [ ] **Step 4: Push Ask action in handle\_ask() after validation**

In `handle_ask()`, after the `InvalidAsk` check and before `self.ask_log.push(...)`, add:

```rust
// Record the validated action for replay.
#[cfg(feature = "history")]
self.pending_turn_actions.push(PlayerAction::Ask { target, rank });
```

- [ ] **Step 5: Push Draw action in handle\_draw() after validation**

In `handle_draw()`, after the phase check and before `let asked_rank = self.last_asked_rank.take();`, add:

```rust
// Record the validated action for replay.
#[cfg(feature = "history")]
self.pending_turn_actions.push(PlayerAction::Draw);
```

- [ ] **Step 6: Flush pending\_turn\_actions in flush\_turn()**

Update the full `flush_turn()` method to include `actions`:

```rust
#[cfg(feature = "history")]
fn flush_turn(&mut self) {
    if self.pending_turn_events.is_empty() {
        return;
    }
    let books: Vec<usize> = self.players.iter().map(Player::book_count).collect();
    self.history.turns.push(TurnRecord {
        player: self.pending_turn_player,
        events: std::mem::take(&mut self.pending_turn_events),
        books_after_turn: books,
        actions: Some(std::mem::take(&mut self.pending_turn_actions)),
    });
}
```

- [ ] **Step 7: Include pending\_turn\_actions in the record() snapshot**

Update `record()` to include in-flight actions:

```rust
#[cfg(feature = "history")]
pub fn record(&self) -> GameRecord {
    let mut record = self.history.clone();
    if !self.pending_turn_events.is_empty() {
        let books: Vec<usize> = self.players.iter().map(Player::book_count).collect();
        record.turns.push(TurnRecord {
            player: self.pending_turn_player,
            events: self.pending_turn_events.clone(),
            books_after_turn: books,
            actions: Some(self.pending_turn_actions.clone()),
        });
    }
    record
}
```

- [ ] **Step 8: Run all tests and clippy**

```bash
cargo test --all-features && cargo clippy --all-features -- -D warnings
```

Expected: all tests pass. `game.record().turns` now have `actions: Some(...)`. The existing integration tests (which check turn count, winner, and YAML round-trip) all still pass because the `actions` field round-trips correctly. No warnings.

- [ ] **Step 9: Commit**

```
git add src/game/state.rs
git commit -m "feat(game): record player actions per turn for engine replay"
```

---

### Task 7: replay.rs — full implementation

**Files:**
- Modify: `src/history/replay.rs`

- [ ] **Step 1: Add unit tests**

Add a `#[cfg(test)]` block at the bottom of `src/history/replay.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::BotProfile;
    use crate::error::GfError;
    use crate::game::{Game, GameEvent, GamePhase, PlayerAction};
    use crate::history::record::{GameCollection, TurnRecord};
    use crate::player::Player;
    use crate::rules::GameVariant;

    fn play_full_game() -> GameRecord {
        let profiles = BotProfile::default_profiles();
        let players: Vec<Player> = profiles.iter().map(|p| Player::new(p.name.clone())).collect();
        let mut game = Game::new(GameVariant::Standard, players).unwrap();
        for _ in 0..13_000 {
            if game.is_over() { break; }
            let state = game.state().unwrap();
            let cp = state.current_player;
            match state.phase {
                GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                    let hand = state.players.iter()
                        .find(|v| v.index == cp)
                        .and_then(|v| v.hand.as_ref())
                        .cloned()
                        .unwrap_or_default();
                    let action = profiles[cp % profiles.len()]
                        .decide(&hand, &state.players, &state.ask_log);
                    game.act(action).unwrap();
                }
                GamePhase::WaitingForDraw => { game.act(PlayerAction::Draw).unwrap(); }
                GamePhase::GameOver => break,
            }
        }
        assert!(game.is_over(), "game must finish within budget");
        game.record()
    }

    #[test]
    fn test_replay_empty_record_is_consistent() {
        let record = GameRecord::new(
            "Standard Go Fish",
            vec!["Alice".to_string(), "Bob".to_string()],
        );
        let result = record.replay().unwrap();
        assert!(result.is_consistent);
        assert!(result.mismatch_at_turn.is_none());
    }

    #[test]
    fn test_replay_played_game_is_consistent() {
        let record = play_full_game();
        let result = record.replay().expect("replay must succeed on engine-recorded game");
        assert!(
            result.is_consistent,
            "mismatch_at_turn={:?}, final_books={:?}",
            result.mismatch_at_turn, result.final_books
        );
    }

    #[test]
    fn test_replay_returns_no_replay_data_when_actions_none() {
        let mut record = GameRecord::new(
            "Standard Go Fish",
            vec!["Alice".to_string(), "Bob".to_string()],
        );
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let err = record.replay().unwrap_err();
        assert_eq!(err, GfError::NoReplayData);
    }

    #[test]
    fn test_replay_unknown_variant_returns_parse_error() {
        let mut record = GameRecord::new(
            "Unknown Variant",
            vec!["Alice".to_string(), "Bob".to_string()],
        );
        // Must have a turn with actions so we reach parse_variant().
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew { player: 0, matched: false }],
            books_after_turn: vec![0, 0],
            actions: Some(vec![PlayerAction::Draw]),
        });
        let err = record.replay().unwrap_err();
        assert!(matches!(err, GfError::ParseError(_)));
    }

    #[test]
    fn test_replay_all_played_collection_all_consistent() {
        let r1 = play_full_game();
        let r2 = play_full_game();
        let mut col = GameCollection::new();
        col.push(r1);
        col.push(r2);
        let results = col.replay_all();
        assert_eq!(results.len(), 2);
        for (i, res) in results.iter().enumerate() {
            assert!(
                res.as_ref().unwrap().is_consistent,
                "game {i} replay was inconsistent"
            );
        }
    }
}
```

- [ ] **Step 2: Run to verify failures**

```bash
cargo test --lib -- replay 2>&1 | tail -20
```

Expected: `test_replay_empty_record_is_consistent` passes (stub returns Ok). `test_replay_returns_no_replay_data_when_actions_none` FAILS (stub always returns Ok). `test_replay_unknown_variant_returns_parse_error` FAILS. `test_replay_played_game_is_consistent` superficially passes (stub says consistent) but doesn't actually verify anything.

- [ ] **Step 3: Add imports and the variant parser**

At the top of `src/history/replay.rs`, add the required imports:

```rust
use crate::error::GfError;
use crate::game::{Game, PlayerAction};
use crate::player::Player;
use crate::rules::GameVariant;

use super::record::{GameCollection, GameRecord};
```

And a private helper that maps the stored variant name string to a `GameVariant`:

```rust
fn parse_variant(name: &str) -> Result<GameVariant, GfError> {
    match name {
        "Standard Go Fish" => Ok(GameVariant::Standard),
        "Happy Families"   => Ok(GameVariant::HappyFamilies),
        "Quartet"          => Ok(GameVariant::Quartet),
        _ => Err(GfError::ParseError(format!("unknown game variant: {name}"))),
    }
}
```

- [ ] **Step 4: Implement GameRecord::replay()**

Replace the stub body:

```rust
pub fn replay(&self) -> Result<ReplayResult, GfError> {
    // No turns: trivially consistent.
    if self.turns.is_empty() {
        return Ok(ReplayResult {
            game_id: self.id.clone(),
            is_consistent: true,
            final_books: vec![],
            mismatch_at_turn: None,
        });
    }

    // All turns must have stored actions before we do any work.
    if self.turns.iter().any(|t| t.actions.is_none()) {
        return Err(GfError::NoReplayData);
    }

    let variant = parse_variant(&self.variant)?;
    let players: Vec<Player> = self.players.iter().map(|n| Player::new(n)).collect();
    let mut game = Game::new(variant, players)?;

    let mut final_books = vec![0usize; self.players.len()];
    let mut mismatch_at_turn: Option<usize> = None;

    'turns: for (i, turn) in self.turns.iter().enumerate() {
        let actions = turn.actions.as_ref().expect("checked above");
        for &action in actions {
            game.act(action)?;
        }

        let state = game.state()?;
        let replayed_books: Vec<usize> = state.players.iter().map(|p| p.books).collect();
        final_books = replayed_books.clone();

        if replayed_books != turn.books_after_turn {
            mismatch_at_turn = Some(i);
            break 'turns;
        }
    }

    let winner_matches = game.record().winner == self.winner;
    let is_consistent = mismatch_at_turn.is_none() && winner_matches;

    Ok(ReplayResult {
        game_id: self.id.clone(),
        is_consistent,
        final_books,
        mismatch_at_turn,
    })
}
```

- [ ] **Step 5: Run replay tests**

```bash
cargo test --lib -- replay && cargo clippy --all-features -- -D warnings
```

Expected: all 5 replay tests pass. No warnings.

- [ ] **Step 6: Run all tests to check no regressions**

```bash
cargo test --all-features
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```
git add src/history/replay.rs
git commit -m "feat(history): implement GameRecord::replay() and GameCollection::replay_all()"
```

---

### Task 8: prelude.rs — re-export AuditResult and ReplayResult

**Files:**
- Modify: `src/prelude.rs`

- [ ] **Step 1: Update the history re-export block**

Replace:

```rust
#[cfg(feature = "history")]
pub use crate::history::{GameCollection, GameRecord, TurnRecord};
```

with:

```rust
#[cfg(feature = "history")]
pub use crate::history::{AuditResult, GameCollection, GameRecord, ReplayResult, TurnRecord};
```

- [ ] **Step 2: Run doc tests and all tests**

```bash
cargo test --doc --all-features && cargo test --all-features
```

Expected: all pass. `AuditResult` and `ReplayResult` are now accessible via `use gfcore::prelude::*`.

- [ ] **Step 3: Commit**

```
git add src/prelude.rs
git commit -m "feat(prelude): re-export AuditResult and ReplayResult"
```

---

### Task 9: tests/history\_integration.rs — update and extend

**Files:**
- Modify: `tests/history_integration.rs`

- [ ] **Step 1: Verify existing tests still pass**

The `GameCollection` round-trip tests (`test_game_collection_yaml_round_trip`, `test_game_collection_json_round_trip`) serialize with `new()`, round-trip through serde, and compare with `PartialEq`. Both sides use the new struct shape, so they pass without body changes.

```bash
cargo test --test history_integration --all-features 2>&1 | tail -15
```

Expected: all existing tests pass.

- [ ] **Step 2: Add imports to history\_integration.rs**

Add at the top of the file alongside the existing imports:

```rust
use gfcore::bot::BotProfile;
use gfcore::prelude::AuditResult;
```

(These are needed by the new tests below.)

- [ ] **Step 3: Add audit integration test**

Append at the bottom of `tests/history_integration.rs`:

```rust
// ---------------------------------------------------------------------------
// Audit integration tests
// ---------------------------------------------------------------------------

/// Play 5 bot games and assert that audit_all() reports all consistent.
#[test]
fn test_audit_all_on_played_collection() {
    let profiles = BotProfile::default_profiles();
    let mut collection = gfcore::history::GameCollection::new();

    for _ in 0..5 {
        let players: Vec<Player> = profiles
            .iter()
            .map(|p| Player::new(p.name.clone()))
            .collect();
        let mut game = Game::new(GameVariant::Standard, players).expect("valid game");

        for _ in 0..13_000 {
            if game.is_over() { break; }
            let state = game.state().expect("state");
            let cp = state.current_player;
            match state.phase {
                GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                    let hand = state.players.iter()
                        .find(|v| v.index == cp)
                        .and_then(|v| v.hand.as_ref())
                        .cloned()
                        .unwrap_or_default();
                    let action = profiles[cp % profiles.len()]
                        .decide(&hand, &state.players, &state.ask_log);
                    game.act(action).expect("act");
                }
                GamePhase::WaitingForDraw => { game.act(PlayerAction::Draw).expect("draw"); }
                GamePhase::GameOver => break,
            }
        }
        assert!(game.is_over());
        collection.push(game.record());
    }

    let results = collection.audit_all();
    assert_eq!(results.len(), 5);
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_consistent,
            "game {i} failed audit: {:?}",
            result.violations
        );
    }
}
```

- [ ] **Step 4: Add replay integration test**

```rust
// ---------------------------------------------------------------------------
// Replay integration tests
// ---------------------------------------------------------------------------

/// Play 5 bot games with action recording and assert replay_all() is consistent.
#[test]
fn test_replay_all_on_played_collection() {
    let profiles = BotProfile::default_profiles();
    let mut collection = gfcore::history::GameCollection::new();

    for _ in 0..5 {
        let players: Vec<Player> = profiles
            .iter()
            .map(|p| Player::new(p.name.clone()))
            .collect();
        let mut game = Game::new(GameVariant::Standard, players).expect("valid game");

        for _ in 0..13_000 {
            if game.is_over() { break; }
            let state = game.state().expect("state");
            let cp = state.current_player;
            match state.phase {
                GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                    let hand = state.players.iter()
                        .find(|v| v.index == cp)
                        .and_then(|v| v.hand.as_ref())
                        .cloned()
                        .unwrap_or_default();
                    let action = profiles[cp % profiles.len()]
                        .decide(&hand, &state.players, &state.ask_log);
                    game.act(action).expect("act");
                }
                GamePhase::WaitingForDraw => { game.act(PlayerAction::Draw).expect("draw"); }
                GamePhase::GameOver => break,
            }
        }
        assert!(game.is_over());
        collection.push(game.record());
    }

    let results = collection.replay_all();
    assert_eq!(results.len(), 5);
    for (i, result) in results.iter().enumerate() {
        let replay = result
            .as_ref()
            .unwrap_or_else(|e| panic!("game {i} replay error: {e}"));
        assert!(
            replay.is_consistent,
            "game {i} replay inconsistent: mismatch_at_turn={:?}",
            replay.mismatch_at_turn
        );
    }
}
```

- [ ] **Step 5: Run integration tests**

```bash
cargo test --test history_integration --all-features 2>&1 | tail -20
```

Expected: all tests pass including the two new integration tests.

- [ ] **Step 6: Run full test suite**

```bash
cargo test --all-features
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```
git add tests/history_integration.rs
git commit -m "test(history): add audit_all and replay_all integration tests"
```

---

### Task 10: bot\_marathon.rs — structural audit in validate\_last\_game

**Files:**
- Modify: `tests/bot_marathon.rs`

- [ ] **Step 1: Add audit call to validate\_last\_game()**

In `validate_last_game`, after the YAML round-trip block, add:

```rust
// Every completed game must pass a structural audit.
let audit = record.audit();
if !audit.is_consistent {
    dump_and_panic(
        game_num,
        "audit",
        format!("structural audit failed: {:?}", audit.violations),
        collection,
    );
}
```

No new imports are needed — `record.audit()` is a method on `&GameRecord` (already in scope), and `audit.is_consistent` / `audit.violations` are plain field accesses.

- [ ] **Step 2: Run the marathon test**

```bash
cargo test --test bot_marathon -- --include-ignored --nocapture 2>&1 | tail -10
```

Expected:
```
bot_marathon: 10/100 games complete
...
bot_marathon: complete — 100 games played without error
```

- [ ] **Step 3: Run full test suite and clippy**

```bash
cargo test --all-features && cargo clippy --all-features -- -D warnings
```

Expected: all tests pass, no warnings.

- [ ] **Step 4: Commit**

```
git add tests/bot_marathon.rs
git commit -m "test(marathon): add structural audit to validate_last_game"
```

---

## Self-Review

### Spec coverage

| Spec requirement | Task |
|---|---|
| `FORMAT_VERSION: u32 = 1` | Task 3 |
| `TurnRecord::actions: Option<Vec<PlayerAction>>` | Task 2 |
| `GameCollection` newtype → versioned struct | Task 3 |
| `GfError::IoError`, `GfError::NoReplayData` | Task 1 |
| `Game::pending_turn_actions` accumulator | Task 6 |
| `AuditResult`, `GameRecord::audit()`, `audit_all()` | Tasks 4 + 5 |
| All 11 audit violation types from spec | Task 5 |
| `ReplayResult`, `GameRecord::replay()`, `replay_all()` | Tasks 4 + 7 |
| `save()`, `save_to()` | Task 3 |
| `prelude.rs` re-exports | Task 8 |
| `history_integration.rs` updated + new tests | Task 9 |
| `bot_marathon.rs` audit call | Task 10 |

### Type consistency

- `TurnRecord::actions: Option<Vec<PlayerAction>>` — defined Task 2, used in Tasks 5, 6, 7
- `GameCollection::games: Vec<GameRecord>` — defined Task 3, accessed in Tasks 4, 5, 7, 9
- `FORMAT_VERSION: u32` — defined Task 3, re-exported in Task 4, tested in Task 3
- `AuditResult` — stub in Task 4, implemented in Task 5, re-exported in Tasks 4 + 8
- `ReplayResult` — stub in Task 4, implemented in Task 7, re-exported in Tasks 4 + 8
- `parse_variant()` maps `"Standard Go Fish"` / `"Happy Families"` / `"Quartet"` — defined and used only in Task 7
