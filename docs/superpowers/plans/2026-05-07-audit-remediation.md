# Audit Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve all five findings from the GPT-5.4 and Gemini 2.5 Pro audits (2026-05-07): one high-severity API contract gap, one medium wasm workflow breakage, and three low-severity doc/API drift issues.

**Architecture:** Finding 1 routes `is_valid_ask()` and `is_book()` through the `GoFishRules` trait so `GameVariant::Custom` works as advertised. Findings 2–5 are targeted doc, config, and CI fixes.

**Tech Stack:** Rust / Cargo, `wasm-bindgen` 0.2.121, GitHub Actions

---

## File Map

| File | Change |
|------|--------|
| `src/game/state.rs` | Route ask validation and book detection through `GoFishRules` trait |
| `src/error/mod.rs` | Update `EmptyDrawPile` doc — clarify it is never emitted by the current engine |
| `src/wasm_api.rs` | Fix stale version example; fix `get_game_yaml()` doc |
| `tests/wasm.rs` | Fix one `unused_must_use` warning (line 222) |
| `.cargo/config.toml` | **Create** — adds `wasm-bindgen-test-runner` as the wasm32 test runner |
| `.github/workflows/CI.yaml` | Add `wasm` CI job |

---

## Task 1: Honor `is_valid_ask()` and `is_book()` in the engine (HIGH)

**Files:**
- Modify: `src/game/state.rs`

### Background

The `GoFishRules` trait exposes `is_valid_ask(hand, rank) -> bool` and `is_book(cards) -> bool`.
`GameVariant::Custom` is documented as "a fully custom variant supplied by the caller."
But the engine ignores those two methods:

- `handle_ask()` calls `self.players[cp].has_rank(&rank)` directly instead of `rules.is_valid_ask()`.
- `check_and_collect_book()` counts `cards_of_rank.len() >= book_size` instead of calling `rules.is_book()`.
- `collect_books_for_player()` does the same count directly.

The fix: route both checks through the trait. Behavior is identical for all three built-in variants (their `is_valid_ask` / `is_book` implementations reproduce the hard-coded logic exactly), so no existing tests break.

---

- [ ] **Step 1: Write the first failing test**

Add to the `#[cfg(test)]` block near the bottom of `src/game/state.rs`, after the existing custom-rules tests:

```rust
#[test]
fn test_custom_is_valid_ask_honored_by_engine() {
    // Custom rule: every ask is valid regardless of hand contents.
    // Without the fix, the engine short-circuits with GfError::InvalidAsk
    // before consulting is_valid_ask, so asking for an unheld rank fails.
    // After the fix it must succeed.
    use cardpack::prelude::{BasicPile, FrenchBasicCard, Pip};
    use crate::rules::{GameVariant, GoFishRules};
    use crate::player::Player;
    use crate::game::action::PlayerAction;

    struct AnyAskValid;
    impl GoFishRules for AnyAskValid {
        fn name(&self) -> &'static str { "AnyAskValid" }
        fn deck(&self) -> BasicPile {
            // Round-robin deal (hand_size=2):
            //   A: [ACE_SPADES, ACE_HEARTS]
            //   B: [KING_SPADES, KING_HEARTS]
            //   draw: [QUEEN_SPADES, QUEEN_HEARTS]
            BasicPile::from(vec![
                FrenchBasicCard::ACE_SPADES, FrenchBasicCard::KING_SPADES,
                FrenchBasicCard::ACE_HEARTS, FrenchBasicCard::KING_HEARTS,
                FrenchBasicCard::QUEEN_SPADES, FrenchBasicCard::QUEEN_HEARTS,
            ])
        }
        fn book_size(&self) -> usize { 4 }
        fn initial_hand_size(&self, _: usize) -> usize { 2 }
        fn min_players(&self) -> usize { 2 }
        fn max_players(&self) -> usize { 4 }
        fn is_valid_ask(&self, _hand: &BasicPile, _rank: &Pip) -> bool { true }
        fn is_book(&self, cards: &BasicPile) -> bool {
            if cards.len() != 4 { return false; }
            #[allow(clippy::indexing_slicing)]
            let first = cards.v()[0].rank;
            cards.v().iter().all(|c| c.rank == first)
        }
    }

    let players = vec![Player::new("A"), Player::new("B")];
    let mut game = Game::new(GameVariant::Custom(Box::new(AnyAskValid)), players).unwrap();

    // A holds [ACE_SPADES, ACE_HEARTS] — does NOT hold King rank.
    let king_rank = FrenchBasicCard::KING_SPADES.rank;
    let result = game.act(PlayerAction::Ask { target: 1, rank: king_rank });
    assert!(
        result.is_ok(),
        "custom is_valid_ask (always true) must permit asking for unheld rank, got: {result:?}"
    );
}
```

- [ ] **Step 2: Write the second failing test**

In the same `#[cfg(test)]` block:

```rust
#[test]
fn test_custom_is_book_honored_by_engine() {
    // Custom rule: a pair of the same rank is a book (book_size=2).
    // Without the fix, the engine counts `cards.len() >= 4`, so a pair is
    // never collected. After the fix it must emit GameEvent::Book.
    use cardpack::prelude::{BasicPile, FrenchBasicCard, Pip};
    use crate::rules::{GameVariant, GoFishRules};
    use crate::player::Player;
    use crate::game::action::PlayerAction;

    struct PairIsBook;
    impl GoFishRules for PairIsBook {
        fn name(&self) -> &'static str { "PairIsBook" }
        fn deck(&self) -> BasicPile {
            // Round-robin deal (hand_size=2):
            //   A: [ACE_SPADES, ACE_HEARTS]
            //   B: [KING_SPADES, KING_HEARTS]
            //   draw: [QUEEN_SPADES, QUEEN_HEARTS]
            BasicPile::from(vec![
                FrenchBasicCard::ACE_SPADES, FrenchBasicCard::KING_SPADES,
                FrenchBasicCard::ACE_HEARTS, FrenchBasicCard::KING_HEARTS,
                FrenchBasicCard::QUEEN_SPADES, FrenchBasicCard::QUEEN_HEARTS,
            ])
        }
        fn book_size(&self) -> usize { 2 }
        fn initial_hand_size(&self, _: usize) -> usize { 2 }
        fn min_players(&self) -> usize { 2 }
        fn max_players(&self) -> usize { 4 }
        fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
            hand.iter().any(|c| &c.rank == rank)
        }
        fn is_book(&self, cards: &BasicPile) -> bool {
            // A pair of cards sharing the same rank is a book.
            if cards.len() != 2 { return false; }
            #[allow(clippy::indexing_slicing)]
            cards.v()[0].rank == cards.v()[1].rank
        }
    }

    // A: [ACE_SPADES, ACE_HEARTS] — 1 ace each (no initial book because
    // startup collect_books_for_player calls is_book([single ace]) = false).
    // Wait — A has TWO aces in hand. PairIsBook.is_book([ACE_SPADES, ACE_HEARTS])
    // = true, so collect_books_for_player collects them at startup.
    // A ends up with an empty hand and must draw to replenish.
    //
    // Use a different deck to avoid startup books: interleave ranks so
    // no player gets two of the same rank in the initial deal.
    //
    // Revised deck:
    //   A: [ACE_SPADES, KING_SPADES]
    //   B: [ACE_HEARTS, KING_HEARTS]
    //   draw: [QUEEN_SPADES, QUEEN_HEARTS]
    struct PairIsBook2;
    impl GoFishRules for PairIsBook2 {
        fn name(&self) -> &'static str { "PairIsBook2" }
        fn deck(&self) -> BasicPile {
            BasicPile::from(vec![
                FrenchBasicCard::ACE_SPADES, FrenchBasicCard::ACE_HEARTS,
                FrenchBasicCard::KING_SPADES, FrenchBasicCard::KING_HEARTS,
                FrenchBasicCard::QUEEN_SPADES, FrenchBasicCard::QUEEN_HEARTS,
            ])
        }
        fn book_size(&self) -> usize { 2 }
        fn initial_hand_size(&self, _: usize) -> usize { 2 }
        fn min_players(&self) -> usize { 2 }
        fn max_players(&self) -> usize { 4 }
        fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
            hand.iter().any(|c| &c.rank == rank)
        }
        fn is_book(&self, cards: &BasicPile) -> bool {
            if cards.len() != 2 { return false; }
            #[allow(clippy::indexing_slicing)]
            cards.v()[0].rank == cards.v()[1].rank
        }
    }

    // With deck [ACE_SPADES, ACE_HEARTS, KING_SPADES, KING_HEARTS, ...]:
    // Round-robin deal (hand_size=2):
    //   iter1: A gets ACE_SPADES, B gets ACE_HEARTS
    //   iter2: A gets KING_SPADES, B gets KING_HEARTS
    // A: [ACE_SPADES, KING_SPADES]
    // B: [ACE_HEARTS, KING_HEARTS]
    // No startup books (each player has 1 ace + 1 king, pairs don't exist yet).

    let players = vec![Player::new("A"), Player::new("B")];
    let mut game = Game::new(GameVariant::Custom(Box::new(PairIsBook2)), players).unwrap();

    // A asks B for ACE rank. B gives ACE_HEARTS. A now has [ACE_SPADES, ACE_HEARTS, KING_SPADES].
    // check_and_collect_book sees 2 aces → is_book([ACE_SPADES, ACE_HEARTS]) = true → Book!
    let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
    let event = game.act(PlayerAction::Ask { target: 1, rank: ace_rank }).unwrap();
    assert!(
        matches!(event, GameEvent::Book { .. }),
        "custom is_book (pair=book) must collect a 2-card book, got: {event:?}"
    );
}
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cargo test test_custom_is_valid_ask_honored_by_engine test_custom_is_book_honored_by_engine -- --nocapture 2>&1 | tail -20
```

Expected: both tests FAIL (one with `Err(InvalidAsk)`, one with `Gave` not `Book`).

- [ ] **Step 4: Add `GoFishRules` to the import in `state.rs`**

In `src/game/state.rs`, change line 27:

```rust
// OLD
use crate::rules::GameVariant;

// NEW
use crate::rules::{GameVariant, GoFishRules};
```

- [ ] **Step 5: Update `handle_ask()` to use `is_valid_ask()`**

In `src/game/state.rs`, inside `handle_ask()`, replace the direct `has_rank` guard:

```rust
// OLD (around line 501-503)
        // The asker must hold the requested rank.
        if !self.players[cp].has_rank(&rank) {
            return Err(GfError::InvalidAsk);
        }

// NEW
        // The asker must satisfy the variant's ask rule.
        if !self.variant.rules().is_valid_ask(self.players[cp].hand(), &rank) {
            return Err(GfError::InvalidAsk);
        }
```

- [ ] **Step 6: Update `check_and_collect_book()` to use `is_book()`**

Replace the entire `check_and_collect_book` method (around line 840):

```rust
// OLD
    fn check_and_collect_book(&mut self, player: usize, rank: &Pip, book_size: usize) -> bool {
        if player >= self.players.len() {
            return false;
        }
        let cards_of_rank = self.players[player].cards_of_rank(rank);
        if cards_of_rank.len() >= book_size {
            let book = self.players[player].give_cards_of_rank(rank);
            self.players[player].add_book(book);
            return true;
        }
        false
    }

// NEW
    fn check_and_collect_book(&mut self, player: usize, rank: &Pip) -> bool {
        if player >= self.players.len() {
            return false;
        }
        let cards_of_rank = self.players[player].cards_of_rank(rank);
        if self.variant.rules().is_book(&cards_of_rank) {
            let book = self.players[player].give_cards_of_rank(rank);
            self.players[player].add_book(book);
            return true;
        }
        false
    }
```

- [ ] **Step 7: Update `collect_books_for_player()` to use `is_book()`**

Replace the entire `collect_books_for_player` method (around line 854):

```rust
// OLD
    fn collect_books_for_player(player: &mut Player, book_size: usize) {
        let ranks: Vec<Pip> = player.held_ranks();
        for rank in ranks {
            let count = player.cards_of_rank(&rank).len();
            if count >= book_size {
                let book = player.give_cards_of_rank(&rank);
                player.add_book(book);
            }
        }
    }

// NEW
    fn collect_books_for_player(player: &mut Player, rules: &dyn GoFishRules) {
        let ranks: Vec<Pip> = player.held_ranks();
        for rank in ranks {
            let cards = player.cards_of_rank(&rank);
            if rules.is_book(&cards) {
                let book = player.give_cards_of_rank(&rank);
                player.add_book(book);
            }
        }
    }
```

- [ ] **Step 8: Update all call sites**

There are four call sites to update in `src/game/state.rs`:

**In `Game::new()` (around line 276–278):**
```rust
// OLD
        let book_size = rules.book_size();
        for player in &mut players {
            Self::collect_books_for_player(player, book_size);
        }

// NEW
        for player in &mut players {
            Self::collect_books_for_player(player, rules);
        }
```

**In `handle_ask()` (around line 553–554):**
```rust
// OLD
        let book_size = self.variant.rules().book_size();
        let last_event = if self.check_and_collect_book(cp, &rank, book_size) {

// NEW
        let last_event = if self.check_and_collect_book(cp, &rank) {
```

**In `handle_draw()` (around line 649–654):**
```rust
// OLD
        let book_size = self.variant.rules().book_size();

        let last_event = if matched {
            if let Some(ref r) = asked_rank {
                if self.check_and_collect_book(cp, r, book_size) {

// NEW
        let last_event = if matched {
            if let Some(ref r) = asked_rank {
                if self.check_and_collect_book(cp, r) {
```

**In `replenish_until_has_cards()` (around line 706–721):**
```rust
// OLD
        let book_size = self.variant.rules().book_size();
        // ...
                self.check_and_collect_book(player_index, &r, book_size);

// NEW  (remove the book_size line; update the call)
                self.check_and_collect_book(player_index, &r);
```

- [ ] **Step 9: Run the two new tests — expect them to pass**

```bash
cargo test test_custom_is_valid_ask_honored_by_engine test_custom_is_book_honored_by_engine -- --nocapture
```

Expected: both PASS.

- [ ] **Step 10: Run full test suite and clippy — expect clean**

```bash
cargo test --all-features 2>&1 | tail -5
cargo clippy --all-features -- -D warnings 2>&1 | tail -5
```

Expected: all tests pass, no clippy warnings.

---

## Task 2: Fix wasm test workflow (MEDIUM)

**Files:**
- Create: `.cargo/config.toml`
- Modify: `tests/wasm.rs` (line 222)
- Modify: `.github/workflows/CI.yaml`

- [ ] **Step 1: Create `.cargo/config.toml`**

Create the file `/Users/christoph/src/github.com/ImperialBower/gfcore/.cargo/config.toml`:

```toml
[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
```

This is the missing file that `tests/wasm.rs` references in its module doc and is required for `cargo test --target wasm32-unknown-unknown` to dispatch the tests automatically.

- [ ] **Step 2: Fix the `unused_must_use` warning in `tests/wasm.rs`**

Line 222 calls `new_human_vs_bots_game(...)` and discards the `#[must_use]` return value. Bind it:

```rust
// OLD (line 222)
    new_human_vs_bots_game("Standard", "You", 3, 0.0);

// NEW
    let _init = new_human_vs_bots_game("Standard", "You", 3, 0.0);
```

- [ ] **Step 3: Add the wasm CI job**

Append a new job to `.github/workflows/CI.yaml` (after the `miri` job, before the final blank line):

```yaml
  wasm:
    name: Wasm
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
          components: rust-src
      - run: cargo install wasm-bindgen-cli --version 0.2.121 --locked
      - run: cargo test --target wasm32-unknown-unknown --test wasm --features wasm
```

The `--version 0.2.121` pins the CLI to match the `wasm-bindgen` version in `Cargo.lock`, preventing the schema-mismatch error the auditor observed locally.

- [ ] **Step 4: Verify locally (if wasm-bindgen-cli is installed)**

If `wasm-bindgen-test-runner` is already on PATH at the correct version:

```bash
cargo test --target wasm32-unknown-unknown --test wasm --features wasm 2>&1 | tail -10
```

If the CLI is not installed locally, skip this step — CI will validate it.

---

## Task 3: Fix `get_game_yaml()` doc drift (LOW)

**Files:**
- Modify: `src/wasm_api.rs` (lines 389–392)

The doc comment says the function returns an error "if the game is not yet over." The implementation does not enforce this — it serializes in-progress history just fine. Update the doc to match reality.

- [ ] **Step 1: Update the doc comment**

In `src/wasm_api.rs`, replace the doc block above `get_game_yaml`:

```rust
// OLD
/// Returns the full game history as YAML.
///
/// Returns `{"error":"..."}` if no game is in progress, the game is not yet
/// over, or the `history` feature is not enabled.

// NEW
/// Returns the full game history as YAML.
///
/// Works for both in-progress and completed games; call at any time after
/// [`new_game`] to snapshot the current history.
///
/// Returns `{"error":"..."}` if no game is in progress or the `history`
/// feature is not enabled.
```

- [ ] **Step 2: Run doc tests**

```bash
cargo test --doc --all-features 2>&1 | tail -5
```

Expected: all doc tests pass.

---

## Task 4: Clarify `GfError::EmptyDrawPile` doc (LOW)

**Files:**
- Modify: `src/error/mod.rs` (lines 115–131)

The variant is publicly defined but the engine never emits it. The existing doc implies it is a live safety guard, which misleads API consumers. Update the doc to be honest about current behavior.

- [ ] **Step 1: Update the variant doc**

In `src/error/mod.rs`, replace the doc comment for `EmptyDrawPile`:

```rust
// OLD
    /// A draw was attempted on an empty draw pile.
    ///
    /// Under normal game flow the draw pile should never be exhausted unexpectedly;
    /// this variant acts as a defensive guard.

// NEW
    /// Reserved for callers or future engine versions that need to signal a
    /// draw on an empty pile as a hard error.
    ///
    /// The current engine does not emit this variant; when the draw pile is
    /// exhausted it advances the turn and emits [`GameEvent::Drew`] with
    /// `matched: false` instead of returning an error.
```

- [ ] **Step 2: Run full tests**

```bash
cargo test --all-features 2>&1 | tail -5
```

Expected: all tests pass.

---

## Task 5: Fix stale version example in wasm docs (LOW)

**Files:**
- Modify: `src/wasm_api.rs` (line 79)

The example shows the hardcoded string `"0.1.0"` but `Cargo.toml` declares `0.0.1`. Remove the hardcoded value so the example never drifts again.

- [ ] **Step 1: Update the example**

In `src/wasm_api.rs`, replace line 79:

```rust
// OLD
/// console.log(version()); // "0.1.0"

// NEW
/// console.log(version()); // e.g. "0.0.1" — returns the crate version string
```

- [ ] **Step 2: Run doc tests**

```bash
cargo test --doc --all-features 2>&1 | tail -5
```

Expected: all doc tests pass.

---

## Verification

Run this sequence after all tasks are complete:

```bash
cargo test --all-features
cargo test --doc --all-features
cargo clippy --all-features -- -D warnings
```

All three must exit 0 with no warnings.

If `wasm-bindgen-cli 0.2.121` is installed locally, also run:

```bash
cargo test --target wasm32-unknown-unknown --test wasm --features wasm
```
