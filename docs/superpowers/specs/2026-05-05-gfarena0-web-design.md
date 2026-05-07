# gfarena0-web Design

**Date:** 2026-05-05
**Status:** Approved

## Overview

`gfarena0-web` is a browser-based Go Fish game: one human player vs 3 bots,
Standard variant, built on `gfcore`'s WASM API. It mirrors `pkarena0-web`'s
project structure (cdylib Rust shell + single-file HTML/CSS/JS frontend +
Playwright tests) but contains almost no Rust code of its own — all game logic
lives in `gfcore`.

## Scope

- Single game variant: Standard Go Fish
- 4 players: human (player 0, named "You") + 3 bots (Harriet, Bertram, Lucky)
- No replay viewer, no YAML download in v0.1
- Dark theme matching pkarena0-web aesthetic

---

## Part 1 — gfcore changes

Two surgical additions to `gfcore`. No existing behaviour changes.

### 1a. `PlayerView`: expose completed book ranks

Add `completed_book_ranks: Vec<String>` to `PlayerView` (in
`src/player/view.rs`). Populated from the player's completed books in
`PlayerView::from_perspective` — each entry is the `rank.index` character of
the first card in each completed book pile (e.g. `["A", "7", "K"]`).

This requires a `Player::book_ranks() -> Vec<String>` helper (or equivalent
inline logic) that iterates the internal books pile-list.

### 1b. `wasm_api`: mixed human+bot game support

**Change `PROFILES` type:**

```rust
// before
static PROFILES: RefCell<Vec<BotProfile>> = const { RefCell::new(Vec::new()) };

// after
static PROFILES: RefCell<Vec<Option<BotProfile>>> = const { RefCell::new(Vec::new()) };
```

Update `new_bot_game` to wrap each profile in `Some(...)`.

Update `step_bot`'s profile lookup:

```rust
// before
profiles.get(current_player).cloned()

// after
profiles.get(current_player).and_then(|opt| opt.clone())
```

**Add `new_human_vs_bots_game`:**

```rust
#[wasm_bindgen]
pub fn new_human_vs_bots_game(
    variant: &str,
    human_name: &str,
    bot_count: usize,
    _seed: f64,
) -> String
```

- Creates player 0 as a human with name `human_name`.
- Creates players 1..=bot_count from the first `bot_count` entries of
  `BotProfile::default_profiles()`.
- Sets `PROFILES = [None, Some(p1), Some(p2), …]`.
- Returns initial `GameState` JSON, or `{"error":"…"}` on failure.
- `bot_count` must be ≥ 1 and ≤ `BotProfile::default_profiles().len()`.

---

## Part 2 — gfarena0-web project

### File structure

```
gfarena0-web/
├── Cargo.toml
├── Cargo.lock
├── Makefile
├── src/
│   └── lib.rs              # extern crate gfcore; — intentionally minimal
├── www/
│   └── index.html          # complete single-file game UI
├── tests/
│   └── game.spec.ts        # Playwright smoke tests
├── package.json
└── playwright.config.ts
```

### Cargo.toml

```toml
[package]
name    = "gfarena0-web"
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-or-later"

[lib]
crate-type = ["cdylib"]

[dependencies]
gfcore = { path = "../gfcore", features = ["wasm", "history"] }
```

`src/lib.rs` contains `extern crate gfcore;`. Because `gfcore/src/lib.rs`
does `pub use wasm_api::*` under the `wasm` feature, all `#[wasm_bindgen]`
exports are included in the final WASM module automatically.

### Makefile targets

| Target          | Action                                              |
|-----------------|-----------------------------------------------------|
| `build`         | `wasm-pack build --target web --out-dir www/pkg`    |
| `build-release` | same with `--release`                               |
| `serve`         | `build` + `python3 -m http.server 8080`             |
| `kill`          | kill process on :8080                               |
| `test`          | `build` + `npx playwright test`                     |
| `test-ui`       | `build` + `npx playwright test --ui`                |
| `clean`         | `cargo clean` + `rm -rf www/pkg`                    |

---

## Part 3 — JS game loop

### WASM functions used

| Function                                              | Purpose                       |
|-------------------------------------------------------|-------------------------------|
| `new_human_vs_bots_game("Standard", "You", 3, seed)`  | Start a new game              |
| `get_state()`                                         | Read current `GameState` JSON |
| `act(actionJson)`                                     | Submit human action           |
| `step_bot()`                                          | Advance one bot action        |
| `version()`                                           | Display crate version         |

### State machine

```
init
 └─ call new_human_vs_bots_game() → render
     ↓
 check state.current_player
     ├─ 0 (human)
     │   ├─ WaitingForAsk  → show hand buttons + target selector
     │   └─ WaitingForDraw → show "Draw" button only
     └─ 1-3 (bot)
         └─ call step_bot() every 700 ms
             ├─ done:false → update status, continue loop
             └─ done:true  → call get_state(), re-enter check
```

`BookCompleted` phase is transient — the game engine returns immediately to
`WaitingForAsk` on the next `act()`. The JS loop treats it identically to
`WaitingForAsk`.

### Action encoding

**Ask:**
```js
// rankObj is state.players[0].hand.cards[i].rank verbatim
act(JSON.stringify({ Ask: { target: targetIdx, rank: rankObj } }))
```

**Draw:**
```js
act('"Draw"')
```

---

## Part 4 — UI layout

Single `index.html`, dark theme (`#0d0d1a` background).

```
┌─────────────────────────────────────────┐
│  gfarena0  v0.1.0        Draw pile: 18  │  score bar
├─────────────────────────────────────────┤
│  "Harriet asks Lucky for 7s — Go Fish!" │  status line
├──────────┬───────────┬──────────────────┤
│ Harriet  │  Bertram  │     Lucky        │  bot panels
│ ✋ 6    │  ✋ 5     │    ✋ 4          │  hand size
│ A  7  K  │  3  Q     │    —             │  completed book ranks
├──────────┴───────────┴──────────────────┤
│  Your hand:                             │
│  [A×2]  [7×1]  [K×3]  [Q×1]           │  rank-group buttons
│                                         │
│  Ask:  [Harriet] [Bertram] [Lucky]      │  target selector
│                                         │
│  Your books:  5  J                      │  human completed books
│                                         │
│  [Go Fish — Draw a card]                │  shown only in WaitingForDraw
└─────────────────────────────────────────┘
```

**Interaction rules:**
- Clicking a rank button selects it (highlight); target buttons activate.
- Clicking a target immediately submits the ask (two-tap: rank → target).
- In `WaitingForDraw`: hand buttons and target buttons hidden; only "Draw" shown.
- While bots act: all controls disabled; status line updates each step.
- On `GameOver`: overlay with winner name + final book counts + "Play Again" button.

---

## Playwright tests (smoke)

- Game loads and `version()` is non-empty.
- New game initialises with 4 players, player 0 named "You".
- Human can select a rank and a target; `act()` returns a valid event.
- Bot stepping runs until human's turn without error.
- Full game run: Playwright drives human turns (select first available rank → first available target, or click Draw) until `state.phase === "GameOver"` — verifies no JS errors and winner is displayed.
