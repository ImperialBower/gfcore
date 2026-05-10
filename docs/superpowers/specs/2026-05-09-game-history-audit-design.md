# Game History Audit & Replay — Design Spec

**Date:** 2026-05-09
**Status:** Approved
**Scope:** `gfcore` — `src/history/`, `src/error/`, `src/game/state.rs`, `src/prelude.rs`, integration tests

---

## Problem

Apps like `gfarena` can export a game's YAML history via `get_game_yaml()` (single `GameRecord`)
or accumulate a session into a `GameCollection`. There is currently no way to:

1. Validate that an exported file is internally consistent (book conservation, winner correctness, etc.)
2. Re-run recorded actions through a fresh engine instance and confirm the outcomes match
3. Save a `GameCollection` to a timestamped file on disk

This spec adds all three capabilities, modelled after pkcore's `HandHistory::replay` /
`HandCollection::replay_all` pattern.

---

## Decisions

| Question | Decision |
|---|---|
| Audit depth | Both structural (any file) + engine replay (requires stored actions) |
| Version handling | No migration burden — no files exist in the wild yet |
| Save location | `save(run_name)` → `generated/<run_name>_<ts>.yaml`; `save_to(path)` for explicit paths |
| WASM exposure | None — audit/replay is native-only |

---

## Module Structure

Current `src/history/` has only `mod.rs` + `record.rs`.  After this change:

```
src/history/
  mod.rs     — re-exports: GameRecord, GameCollection, TurnRecord,
                           AuditResult, ReplayResult, FORMAT_VERSION
  record.rs  — data types, serde, GameCollection versioning, save()/save_to()
  audit.rs   — AuditResult; GameRecord::audit(); GameCollection::audit_all()
  replay.rs  — ReplayResult; GameRecord::replay(); GameCollection::replay_all()
```

`audit.rs` and `replay.rs` extend types from `record.rs` via `impl` blocks.
`record.rs` stays focused on data definition and I/O.

---

## Data Model Changes

### `FORMAT_VERSION`

```rust
pub const FORMAT_VERSION: u32 = 1;
```

The initial versioned format. Written into every new `GameCollection`.

### `TurnRecord` — new field

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub actions: Option<Vec<PlayerAction>>,
```

- `None` — recorded without the replay path (e.g. WASM games, old unit tests).
- `Some(...)` — recorded by the full native engine; enables `replay()`.

`PlayerAction` is already `Serialize + Deserialize`.

### `GameCollection` — structural change

From newtype `(Vec<GameRecord>)` to:

```rust
pub struct GameCollection {
    #[serde(default = "default_gfcore_version")]
    pub gfcore_version: String,       // e.g. "0.0.2"
    #[serde(default = "default_format_version")]
    pub format_version: u32,          // always FORMAT_VERSION (1) for now
    pub games: Vec<GameRecord>,
}
```

**Breaking YAML format change:** bare array → keyed object. Acceptable at `0.0.x` with no
existing files in the wild. `Index<usize>` and `iter()` are preserved via updated impls.

### `GameRecord` — no format_version field

The presence or absence of `TurnRecord::actions` is the capability signal. No separate
`format_version` field is needed on `GameRecord`.

### `GfError` — two new variants

```rust
/// A filesystem operation failed during save.
IoError(String),

/// replay() was called on a record where at least one turn has no stored actions.
NoReplayData,
```

`From<std::io::Error>` is **not** added. Conversion is done manually in `save`/`save_to`
to keep `GfError: Clone + PartialEq`.

### `Game` (`state.rs`) — action accumulator

`pending_turn_actions: Vec<PlayerAction>` is added alongside `pending_turn_events`.
Populated in `handle_ask()` and `handle_draw()`, flushed into `TurnRecord::actions`
by `flush_turn()`. Both fields are gated `#[cfg(feature = "history")]`.

---

## API Surface

### `audit.rs`

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditResult {
    pub game_id: String,
    pub is_consistent: bool,
    pub final_books: Vec<usize>,   // per-player book counts at last turn
    pub violations: Vec<String>,   // empty when is_consistent
}

impl GameRecord {
    /// Validates structural invariants. Infallible — violations go into AuditResult.
    pub fn audit(&self) -> AuditResult { ... }
}

impl GameCollection {
    pub fn audit_all(&self) -> Vec<AuditResult> { ... }
}
```

**`audit()` checks (in order):**

1. `players.len() >= 2`
2. For each turn: `turn.player < players.len()`
3. For each turn: `turn.books_after_turn.len() == players.len()`
4. For each turn: `!turn.events.is_empty()`
5. Book counts non-decreasing per player across turns
6. `total_books <= 13` (conservative max for all built-in 52-card variants)
7. Winner consistent with final book counts:
   - `Some(w)`: player `w` in range and holds the unique max book count
   - `None` with turns present: no single player holds a unique max (tie or all-zero)

### `replay.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    pub game_id: String,
    pub is_consistent: bool,
    pub final_books: Vec<usize>,
    pub mismatch_at_turn: Option<usize>,  // first turn where replay diverged
}

impl GameRecord {
    /// Re-runs stored actions through a fresh Game engine.
    /// Returns Err(GfError::NoReplayData) if any turn lacks actions.
    pub fn replay(&self) -> Result<ReplayResult, GfError> { ... }
}

impl GameCollection {
    /// Replays all games; per-record Results allow partial success.
    pub fn replay_all(&self) -> Vec<Result<ReplayResult, GfError>> { ... }
}
```

**`replay()` logic:**

1. If any `turn.actions` is `None` → `Err(GfError::NoReplayData)`
2. Create `Game::new(variant, players)` with the same players as the record
3. For each `TurnRecord`:
   - Feed `turn.actions.unwrap()` through `game.act()` in order
   - Compare resulting `books_after_turn` to stored `turn.books_after_turn`
   - On first mismatch: record `mismatch_at_turn = Some(i)`, set `is_consistent = false`, stop
4. Compare final `game.record().winner` to `self.winner`

The variant name string (`record.variant`) is mapped back to `GameVariant` for `Game::new()`.
Unknown variants → `Err(GfError::ParseError(...))`.

### `record.rs` additions

```rust
impl GameCollection {
    /// Writes to generated/<run_name>_<unix_ts>.yaml. Creates the directory if needed.
    /// Returns the path written on success.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, run_name: &str) -> Result<String, GfError> { ... }

    /// Writes to the caller-supplied path. Creates parent directories if needed.
    /// Returns the path written on success.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to(&self, path: &str) -> Result<String, GfError> { ... }
}
```

### `prelude.rs` additions

```rust
pub use crate::history::{AuditResult, ReplayResult};
```

---

## Error Handling

- `audit()` is infallible. Violations accumulate in `AuditResult::violations`; `is_consistent`
  is `false` iff any violation exists.
- `replay()` returns `Result`. `GfError::NoReplayData` is the expected "not recorded with actions"
  case. Other errors (`ParseError`, `InvalidAsk`, etc.) indicate corrupt or engine-inconsistent data.
- `save()`/`save_to()` return `Result<String, GfError>` where `Err` wraps `GfError::IoError`.
- `replay_all()` returns `Vec<Result<...>>` — one entry per game — so a single un-replayable game
  does not abort the batch.

---

## Testing

### Unit tests (in each new file)

**`audit.rs`** — one test per violation type:
- player count < 2
- turn player index out of range
- `books_after_turn` length mismatch
- empty events in a turn
- book counts decreasing
- total books > 13
- `winner: Some(w)` but another player has more books
- `winner: Some(w)` but multiple players are tied
- `winner: None` with a clear unique leader
- clean record with turns → `is_consistent: true`
- empty record (no turns) → `is_consistent: true`

**`replay.rs`** — construct `GameRecord`s with known actions, assert `is_consistent`; assert
`NoReplayData` for records without actions.

**`record.rs`** — update existing `GameCollection` unit tests for the new struct shape
(YAML now has `gfcore_version`/`format_version`/`games` keys). Add `save_to()` test writing
to `std::env::temp_dir()`.

### Integration tests (`tests/history_integration.rs`)

- `test_audit_all_on_played_collection` — plays 5 bot games, calls `audit_all()`, asserts all
  `is_consistent`.
- `test_replay_all_on_played_collection` — same 5 games, calls `replay_all()`, asserts all
  `Ok` and `is_consistent`. Proves action-recording and event-recording paths agree.

### Marathon update (`tests/bot_marathon.rs`)

`validate_last_game` gains `record.audit()` alongside the YAML round-trip, so every one of
the 100 marathon games is structurally audited automatically.

---

## Files Changed

| File | Change |
|---|---|
| `src/history/mod.rs` | add `pub mod audit; pub mod replay;`; update re-exports |
| `src/history/record.rs` | `GameCollection` newtype → struct; add `FORMAT_VERSION`; add `save()`/`save_to()`; update all impls and unit tests |
| `src/history/audit.rs` | new file |
| `src/history/replay.rs` | new file |
| `src/error/mod.rs` | add `IoError`, `NoReplayData`; update `Display` and tests |
| `src/game/state.rs` | add `pending_turn_actions`; populate in `handle_ask`/`handle_draw`; flush in `flush_turn` |
| `src/prelude.rs` | re-export `AuditResult`, `ReplayResult` |
| `tests/history_integration.rs` | update existing tests for new `GameCollection` shape; add audit/replay integration tests |
| `tests/bot_marathon.rs` | add `record.audit()` call in `validate_last_game` |
