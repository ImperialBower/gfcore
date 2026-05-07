# gfarena0-web Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `gfarena0-web` — a browser Go Fish game (human vs 3 bots) backed by `gfcore`'s WASM API, with two surgical additions to `gfcore` first.

**Architecture:** Two gfcore changes (expose book ranks in `PlayerView`; add mixed human+bot game init) then a minimal cdylib crate whose `lib.rs` is `extern crate gfcore;`, with a single `www/index.html` containing all HTML, CSS, and JS.

**Tech Stack:** Rust 2024 / wasm-pack / wasm-bindgen, cardpack 0.7, vanilla JS ES modules, Playwright for smoke tests.

---

## File Map

### gfcore changes
| File | Change |
|------|--------|
| `gfcore/src/player/player.rs` | Add `book_ranks() -> Vec<String>` method |
| `gfcore/src/player/view.rs` | Add `completed_book_ranks: Vec<String>` to `PlayerView` |
| `gfcore/src/wasm_api.rs` | Change `PROFILES` type; update `new_bot_game` and `step_bot`; add `new_human_vs_bots_game` |
| `gfcore/tests/wasm.rs` | Add wasm tests for new function |

### gfarena0-web (all new files)
| File | Purpose |
|------|---------|
| `gfarena0-web/Cargo.toml` | cdylib package depending on gfcore with wasm+history |
| `gfarena0-web/src/lib.rs` | `extern crate gfcore;` — pulls in all WASM exports |
| `gfarena0-web/Makefile` | build / serve / test targets |
| `gfarena0-web/.gitignore` | ignore target/, www/pkg/, node_modules/ |
| `gfarena0-web/.tool-versions` | nodejs version pin |
| `gfarena0-web/package.json` | Playwright dev dependency |
| `gfarena0-web/playwright.config.ts` | Playwright config pointing at www/ |
| `gfarena0-web/www/index.html` | Complete single-file game UI |
| `gfarena0-web/tests/game.spec.ts` | Playwright smoke tests |

---

## Task 1: Add `Player::book_ranks()` and `PlayerView::completed_book_ranks`

**Files:**
- Modify: `gfcore/src/player/player.rs`
- Modify: `gfcore/src/player/view.rs`

- [ ] **Step 1.1: Write the failing unit test for `book_ranks()`**

Add inside the `#[cfg(test)]` block at the bottom of `gfcore/src/player/player.rs`:

```rust
#[test]
fn test_player_book_ranks_empty() {
    let player = Player::new("Alice");
    assert!(player.book_ranks().is_empty());
}

#[test]
fn test_player_book_ranks_one_book() {
    let mut player = Player::new("Alice");
    let book = BasicPile::from(vec![
        FrenchBasicCard::ACE_SPADES,
        FrenchBasicCard::ACE_HEARTS,
        FrenchBasicCard::ACE_DIAMONDS,
        FrenchBasicCard::ACE_CLUBS,
    ]);
    player.add_book(book);
    assert_eq!(player.book_ranks(), vec!["A".to_string()]);
}

#[test]
fn test_player_book_ranks_multiple() {
    let mut player = Player::new("Bob");
    let ace_book = BasicPile::from(vec![
        FrenchBasicCard::ACE_SPADES,
        FrenchBasicCard::ACE_HEARTS,
        FrenchBasicCard::ACE_DIAMONDS,
        FrenchBasicCard::ACE_CLUBS,
    ]);
    let king_book = BasicPile::from(vec![
        FrenchBasicCard::KING_SPADES,
        FrenchBasicCard::KING_HEARTS,
        FrenchBasicCard::KING_DIAMONDS,
        FrenchBasicCard::KING_CLUBS,
    ]);
    player.add_book(ace_book);
    player.add_book(king_book);
    assert_eq!(player.book_ranks(), vec!["A".to_string(), "K".to_string()]);
}
```

- [ ] **Step 1.2: Run tests to confirm they fail**

```bash
cd gfcore && cargo test test_player_book_ranks 2>&1 | tail -20
```
Expected: compile error — `method book_ranks not found`.

- [ ] **Step 1.3: Implement `Player::book_ranks()`**

Add after the `book_count` method in `gfcore/src/player/player.rs`:

```rust
/// Returns the rank index character of each completed book, in collection order.
///
/// Each entry is the `index` of the first card in the book pile
/// (e.g., `"A"` for Aces, `"7"` for Sevens).
///
/// # Examples
///
/// ```
/// use cardpack::prelude::{BasicPile, FrenchBasicCard};
/// use gfcore::prelude::Player;
///
/// let mut player = Player::new("Alice");
/// let book = BasicPile::from(vec![
///     FrenchBasicCard::ACE_SPADES,
///     FrenchBasicCard::ACE_HEARTS,
///     FrenchBasicCard::ACE_DIAMONDS,
///     FrenchBasicCard::ACE_CLUBS,
/// ]);
/// player.add_book(book);
/// assert_eq!(player.book_ranks(), vec!["A".to_string()]);
/// ```
#[must_use]
pub fn book_ranks(&self) -> Vec<String> {
    self.books
        .iter()
        .filter_map(|pile| pile.v().first())
        .map(|card| card.rank.index.to_string())
        .collect()
}
```

- [ ] **Step 1.4: Run tests to confirm they pass**

```bash
cd gfcore && cargo test test_player_book_ranks 2>&1 | tail -10
```
Expected: `3 passed`.

- [ ] **Step 1.5: Add `completed_book_ranks` to `PlayerView`**

In `gfcore/src/player/view.rs`, replace the struct definition:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub index: usize,
    pub name: String,
    pub hand_size: usize,
    pub hand: Option<BasicPile>,
    pub books: usize,
}
```
with:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    pub index: usize,
    pub name: String,
    pub hand_size: usize,
    pub hand: Option<BasicPile>,
    pub books: usize,
    /// Rank index character of each completed book (e.g. `["A","7"]`).
    pub completed_book_ranks: Vec<String>,
}
```

In `PlayerView::from_perspective`, replace the struct literal inside the `.map()` closure:
```rust
// before
PlayerView {
    index,
    name: player.name.clone(),
    hand_size: player.hand_size(),
    hand,
    books: player.book_count(),
}
```
with:
```rust
// after
PlayerView {
    index,
    name: player.name.clone(),
    hand_size: player.hand_size(),
    hand,
    books: player.book_count(),
    completed_book_ranks: player.book_ranks(),
}
```

- [ ] **Step 1.6: Run all tests and doc tests**

```bash
cd gfcore && cargo test --doc 2>&1 | tail -10
cd gfcore && cargo test 2>&1 | tail -10
```
Expected: all pass. The existing `PlayerView` tests in `view.rs` use `from_perspective` which returns the full struct — they compare field by field and do not hard-code the struct literal, so they compile without change.

- [ ] **Step 1.7: Run clippy**

```bash
cd gfcore && cargo clippy --all-features 2>&1 | grep -E "^error|^warning" | head -20
```
Expected: no new warnings.

- [ ] **Step 1.8: Commit**

```bash
cd gfcore
git add src/player/player.rs src/player/view.rs
git commit -m "feat(player): add book_ranks() and PlayerView::completed_book_ranks"
```

---

## Task 2: Update `PROFILES` type in `wasm_api.rs`

**Files:**
- Modify: `gfcore/src/wasm_api.rs`

This task changes `PROFILES: RefCell<Vec<BotProfile>>` to `RefCell<Vec<Option<BotProfile>>>` so that player indices with `None` are treated as human (no bot profile). Only `new_bot_game` and `step_bot` need edits; `new_game` already clears the Vec.

- [ ] **Step 2.1: Change the `PROFILES` thread-local type**

In `gfcore/src/wasm_api.rs`, replace:
```rust
static PROFILES: RefCell<Vec<BotProfile>> = const { RefCell::new(Vec::new()) };
```
with:
```rust
static PROFILES: RefCell<Vec<Option<BotProfile>>> = const { RefCell::new(Vec::new()) };
```

- [ ] **Step 2.2: Update `new_bot_game` to wrap profiles in `Some`**

In `new_bot_game`, replace:
```rust
let profiles: Vec<BotProfile> = all_profiles.drain(..bot_count).collect();
```
with:
```rust
let profiles: Vec<Option<BotProfile>> = all_profiles.drain(..bot_count).map(Some).collect();
```

- [ ] **Step 2.3: Update `step_bot`'s profile lookup**

In `step_bot`, replace:
```rust
let profile_opt: Option<BotProfile> = PROFILES.with(|p| {
    let profiles = p.borrow();
    profiles.get(current_player).cloned()
});
```
with:
```rust
let profile_opt: Option<BotProfile> = PROFILES.with(|p| {
    let profiles = p.borrow();
    profiles.get(current_player).and_then(|opt| opt.clone())
});
```

- [ ] **Step 2.4: Verify existing wasm tests still compile**

```bash
cd gfcore && cargo build --features wasm 2>&1 | tail -10
```
Expected: compiles cleanly.

- [ ] **Step 2.5: Run clippy**

```bash
cd gfcore && cargo clippy --all-features 2>&1 | grep -E "^error|^warning" | head -20
```
Expected: no new warnings.

- [ ] **Step 2.6: Commit**

```bash
cd gfcore
git add src/wasm_api.rs
git commit -m "feat(wasm): change PROFILES to Vec<Option<BotProfile>> for mixed human+bot games"
```

---

## Task 3: Add `new_human_vs_bots_game` + wasm test

**Files:**
- Modify: `gfcore/src/wasm_api.rs`
- Modify: `gfcore/tests/wasm.rs`

- [ ] **Step 3.1: Add `new_human_vs_bots_game` to the wasm.rs import**

In `gfcore/tests/wasm.rs`, replace:
```rust
use gfcore::{act, get_state, new_bot_game, new_game, step_bot, version};
```
with:
```rust
use gfcore::{act, get_state, new_bot_game, new_game, new_human_vs_bots_game, step_bot, version};
```

- [ ] **Step 3.2: Write failing wasm tests**

Add at the end of `gfcore/tests/wasm.rs`:

```rust
// ---------------------------------------------------------------------------
// new_human_vs_bots_game()
// ---------------------------------------------------------------------------

/// Creates a 4-player game: human at index 0, three named bots at 1-3.
#[wasm_bindgen_test]
fn new_human_vs_bots_game_returns_valid_state() {
    let json = new_human_vs_bots_game("Standard", "You", 3, 0.0);
    let state = parse(&json);
    assert!(!is_error(&state), "unexpected error: {json}");
    assert_eq!(state["current_player"], 0);
    assert_eq!(state["phase"], "WaitingForAsk");
    assert_eq!(
        state["players"].as_array().unwrap().len(),
        4,
        "must have 4 players"
    );
    assert_eq!(state["players"][0]["name"], "You");
}

/// `step_bot` must return `done:true` immediately when player 0 (human) is
/// the current player — confirming no bot profile is set for slot 0.
#[wasm_bindgen_test]
fn new_human_vs_bots_game_step_bot_done_on_human_turn() {
    new_human_vs_bots_game("Standard", "You", 3, 0.0);
    // Game always starts with player 0's turn.
    let result = parse(&step_bot());
    assert!(!is_error(&result), "step_bot must not error");
    assert_eq!(
        result["done"], true,
        "step_bot must return done:true on human player's turn"
    );
}

/// Unknown variant returns an error.
#[wasm_bindgen_test]
fn new_human_vs_bots_game_unknown_variant_errors() {
    let json = new_human_vs_bots_game("Bogus", "You", 3, 0.0);
    assert!(is_error(&parse(&json)));
}

/// bot_count = 0 returns an error.
#[wasm_bindgen_test]
fn new_human_vs_bots_game_zero_bots_errors() {
    let json = new_human_vs_bots_game("Standard", "You", 0, 0.0);
    assert!(is_error(&parse(&json)));
}
```

- [ ] **Step 3.3: Confirm tests fail to compile**

```bash
cd gfcore && cargo build --features wasm 2>&1 | grep "new_human_vs_bots_game" | head -5
```
Expected: `unresolved import` or `not found in gfcore`.

- [ ] **Step 3.4: Implement `new_human_vs_bots_game` in `wasm_api.rs`**

Add after `new_bot_game` in `gfcore/src/wasm_api.rs`:

```rust
/// Creates a new game with one human player and `bot_count` bot players.
///
/// - `variant`: `"Standard"`, `"HappyFamilies"`, or `"Quartet"`.
/// - `human_name`: display name for player 0 (the human).
/// - `bot_count`: number of bot players (1–4); bots fill slots 1..=bot_count
///   from [`BotProfile::default_profiles`].
/// - `_seed`: reserved for future reproducible shuffle support; currently ignored.
///
/// [`step_bot`] returns `{"done":true}` when it is player 0's turn, so the
/// caller is responsible for collecting the human's action and calling [`act`].
///
/// Returns the initial [`crate::game::GameState`] as JSON, or
/// `{"error": "..."}` on failure.
#[must_use]
#[wasm_bindgen]
pub fn new_human_vs_bots_game(
    variant: &str,
    human_name: &str,
    bot_count: usize,
    _seed: f64,
) -> String {
    let game_variant = match parse_variant(variant) {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    if bot_count == 0 {
        return error_json("bot_count must be at least 1");
    }

    let mut all_profiles = BotProfile::default_profiles();
    if bot_count > all_profiles.len() {
        return error_json(&format!(
            "bot_count {bot_count} exceeds available default profiles ({})",
            all_profiles.len()
        ));
    }
    let bot_profiles: Vec<BotProfile> = all_profiles.drain(..bot_count).collect();

    let mut players = vec![Player::new(human_name)];
    let mut profiles: Vec<Option<BotProfile>> = vec![None];

    for profile in bot_profiles {
        players.push(Player::new_bot(profile.name.clone(), profile.clone()));
        profiles.push(Some(profile));
    }

    let game = match Game::new(game_variant, players) {
        Ok(g) => g,
        Err(e) => return error_json(&e.to_string()),
    };

    let state_json = match game.state() {
        Ok(s) => match serde_json::to_string(&s) {
            Ok(j) => j,
            Err(e) => return error_json(&e.to_string()),
        },
        Err(e) => return error_json(&e.to_string()),
    };

    GAME.with(|g| *g.borrow_mut() = Some(game));
    PROFILES.with(|p| *p.borrow_mut() = profiles);
    LAST_EVENT.with(|le| *le.borrow_mut() = None);

    state_json
}
```

Verify the imports at the top of `wasm_api.rs` include `Player` and `Game` (they are already used in `new_bot_game` so they should be present):
```rust
use crate::player::Player;
use crate::game::{Game, GameEvent};
```

- [ ] **Step 3.5: Run all tests**

```bash
cd gfcore && cargo test 2>&1 | tail -10
```
Expected: all tests pass.

- [ ] **Step 3.6: Run clippy**

```bash
cd gfcore && cargo clippy --all-features 2>&1 | grep -E "^error|^warning" | head -20
```
Expected: no new warnings.

- [ ] **Step 3.7: Commit**

```bash
cd gfcore
git add src/wasm_api.rs tests/wasm.rs
git commit -m "feat(wasm): add new_human_vs_bots_game for human-vs-bot play"
```

---

## Task 4: Create `gfarena0-web` scaffolding

**Files:**
- Create: `gfarena0-web/Cargo.toml`
- Create: `gfarena0-web/src/lib.rs`
- Create: `gfarena0-web/Makefile`
- Create: `gfarena0-web/.gitignore`
- Create: `gfarena0-web/.tool-versions`
- Create: `gfarena0-web/www/` (directory)

- [ ] **Step 4.1: Create the directory structure**

```bash
mkdir -p gfarena0-web/src gfarena0-web/www gfarena0-web/tests
```

- [ ] **Step 4.2: Create `Cargo.toml`**

`gfarena0-web/Cargo.toml`:
```toml
[package]
name        = "gfarena0-web"
version     = "0.1.0"
edition     = "2024"
rust-version = "1.85"
description = "Go Fish in the browser — human vs bots, powered by gfcore."
license     = "GPL-3.0-or-later"

[lib]
crate-type = ["cdylib"]

[dependencies]
gfcore = { path = "../gfcore", features = ["wasm", "history"] }
```

- [ ] **Step 4.3: Create `src/lib.rs`**

`gfarena0-web/src/lib.rs`:
```rust
// This crate exists solely to produce a WASM module from gfcore.
// All game logic and WASM exports live in gfcore::wasm_api (re-exported
// via gfcore's pub use wasm_api::* under the wasm feature).
extern crate gfcore;
```

- [ ] **Step 4.4: Verify the crate builds for wasm32**

```bash
cd gfarena0-web && cargo build --target wasm32-unknown-unknown 2>&1 | tail -15
```
Expected: compiles with no errors.

- [ ] **Step 4.5: Create `Makefile`**

`gfarena0-web/Makefile`:
```makefile
.PHONY: help build serve kill build-release clean install-playwright test test-ui default

default: build

help:
	@echo "gfarena0-web — available targets:"
	@echo ""
	@echo "  build               wasm-pack dev build -> www/pkg/"
	@echo "  build-release       wasm-pack release build (optimised)"
	@echo "  serve               dev build + python3 http.server on :8080"
	@echo "  kill                kill the http.server on :8080"
	@echo "  clean               cargo clean + remove www/pkg/"
	@echo "  install-playwright  npm install + playwright install chromium"
	@echo "  test                dev build + playwright headless tests"
	@echo "  test-ui             dev build + playwright interactive UI"

build:
	wasm-pack build --target web --out-dir www/pkg

serve: build
	@echo "Serving at http://localhost:8080"
	cd www && python3 -m http.server 8080

kill:
	@lsof -ti :8080 | xargs kill 2>/dev/null || echo "Nothing running on :8080"

build-release:
	wasm-pack build --release --target web --out-dir www/pkg

clean:
	cargo clean
	rm -rf www/pkg

install-playwright:
	npm install
	npx playwright install chromium

test: build
	npx playwright test

test-ui: build
	npx playwright test --ui
```

- [ ] **Step 4.6: Create `.gitignore` and `.tool-versions`**

`gfarena0-web/.gitignore`:
```
/target
/www/pkg
/node_modules
/test-results
/playwright-report
```

`gfarena0-web/.tool-versions`:
```
nodejs 24.14.1
```

- [ ] **Step 4.7: Run `make build` and verify `www/pkg/` is populated**

```bash
cd gfarena0-web && make build 2>&1 | tail -10
```
Expected: `[INFO]: Done in Xs` and `www/pkg/gfarena0_web.js` exists.

```bash
ls gfarena0-web/www/pkg/
```
Expected: `gfarena0_web.js`, `gfarena0_web_bg.wasm`, `gfarena0_web.d.ts`, `package.json`.

- [ ] **Step 4.8: Commit**

```bash
cd gfarena0-web
git init
git add Cargo.toml src/lib.rs Makefile .gitignore .tool-versions
git commit -m "chore: scaffold gfarena0-web cdylib crate"
```

---

## Task 5: Add JS tooling

**Files:**
- Create: `gfarena0-web/package.json`
- Create: `gfarena0-web/playwright.config.ts`

- [ ] **Step 5.1: Create `package.json`**

`gfarena0-web/package.json`:
```json
{
  "name": "gfarena0-web-tests",
  "private": true,
  "scripts": {
    "test": "playwright test",
    "test:ui": "playwright test --ui"
  },
  "devDependencies": {
    "@playwright/test": "^1.48.0"
  }
}
```

- [ ] **Step 5.2: Create `playwright.config.ts`**

`gfarena0-web/playwright.config.ts`:
```typescript
import { defineConfig, devices } from '@playwright/test';
import path from 'path';

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI
    ? [['github'], ['html', { open: 'never' }]]
    : [['html', { open: 'never' }]],

  use: {
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],

  webServer: {
    command: 'python3 -m http.server 8080',
    cwd: path.join(__dirname, 'www'),
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 10_000,
  },
});
```

- [ ] **Step 5.3: Install Playwright**

```bash
cd gfarena0-web && make install-playwright 2>&1 | tail -5
```
Expected: `chromium downloaded`.

- [ ] **Step 5.4: Commit**

```bash
cd gfarena0-web
git add package.json package-lock.json playwright.config.ts
git commit -m "chore: add Playwright test tooling"
```

---

## Task 6: Create `www/index.html` — HTML + CSS skeleton

**Files:**
- Create: `gfarena0-web/www/index.html`

- [ ] **Step 6.1: Create the HTML + CSS file**

`gfarena0-web/www/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="dark">
  <title>gfarena0 - Go Fish</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    :root { color-scheme: dark; }
    body {
      background: #0d0d1a;
      color: #ccc;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
      display: flex;
      flex-direction: column;
      align-items: center;
      min-height: 100vh;
      padding: 8px;
      gap: 8px;
    }

    /* Score bar */
    #score-bar {
      display: flex;
      gap: 20px;
      padding: 6px 20px;
      background: #1a1a2e;
      border: 1px solid #2a2a44;
      border-radius: 8px;
      font-size: 13px;
      color: #aaa;
      align-items: center;
      flex-wrap: wrap;
      justify-content: center;
    }
    #sc-version { color: #666; }
    #score-bar strong { color: #f0d060; }
    #new-game-btn {
      font-size: 11px;
      padding: 2px 10px;
      background: #1a2a3e;
      border: 1px solid #4a7bb9;
      color: #aad4ff;
      border-radius: 4px;
      cursor: pointer;
    }
    #new-game-btn:hover { background: #243954; }

    /* Status line */
    #status-msg {
      font-size: 14px;
      color: #bbb;
      min-height: 1.4em;
      text-align: center;
      padding: 2px 8px;
    }

    /* Bot panels */
    #bots {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      justify-content: center;
    }
    .bot-panel {
      background: #12122a;
      border: 1px solid #2a2a44;
      border-radius: 8px;
      padding: 10px 14px;
      min-width: 110px;
      text-align: center;
      transition: border-color 0.2s;
    }
    .bot-panel.active-turn { border-color: #f0d060; }
    .bot-name { font-size: 14px; font-weight: 600; color: #ddd; margin-bottom: 4px; }
    .bot-hand-size { font-size: 13px; color: #aaa; margin-bottom: 4px; }
    .bot-books { font-size: 13px; color: #88bbff; letter-spacing: 2px; min-height: 1.2em; }

    /* Human area */
    #human-area {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 10px;
      width: 100%;
      max-width: 520px;
    }
    #hand-label { font-size: 13px; color: #888; align-self: flex-start; }
    #hand-cards { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }
    .rank-btn {
      background: #1a1a3a;
      border: 2px solid #3a3a6a;
      color: #ddd;
      border-radius: 6px;
      padding: 8px 14px;
      font-size: 16px;
      font-weight: 600;
      cursor: pointer;
      transition: background 0.15s, border-color 0.15s;
    }
    .rank-btn:hover:not(:disabled) { background: #22224a; border-color: #5a5a9a; }
    .rank-btn.selected { background: #2a2a10; border-color: #f0d060; color: #f0d060; }
    .rank-btn:disabled { opacity: 0.35; cursor: default; }

    #ask-section {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-wrap: wrap;
      justify-content: center;
      font-size: 13px;
      color: #888;
    }
    .target-btn {
      background: #1a2a1a;
      border: 1px solid #3a6a3a;
      color: #aaddaa;
      border-radius: 6px;
      padding: 6px 14px;
      font-size: 14px;
      cursor: pointer;
      transition: background 0.15s;
    }
    .target-btn:hover:not(:disabled) { background: #223322; }
    .target-btn:disabled { opacity: 0.35; cursor: default; }

    #books-section { font-size: 13px; color: #888; }
    #human-books { color: #88bbff; letter-spacing: 2px; margin-left: 4px; }

    #draw-btn {
      background: #1a2a3e;
      border: 2px solid #4a7bb9;
      color: #aad4ff;
      border-radius: 8px;
      padding: 10px 24px;
      font-size: 15px;
      cursor: pointer;
    }
    #draw-btn:hover { background: #243954; }

    /* Game-over overlay */
    #game-over {
      position: fixed;
      inset: 0;
      background: rgba(0,0,0,0.75);
      display: none;
      align-items: center;
      justify-content: center;
      z-index: 10;
    }
    #game-over-box {
      background: #1a1a2e;
      border: 1px solid #4a4a7a;
      border-radius: 12px;
      padding: 32px 40px;
      text-align: center;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    #game-over-title { font-size: 28px; color: #f0d060; }
    #final-scores { font-size: 14px; color: #aaa; line-height: 1.8; }
    #play-again-btn {
      background: #1a2a3e;
      border: 1px solid #4a7bb9;
      color: #aad4ff;
      border-radius: 6px;
      padding: 8px 24px;
      font-size: 15px;
      cursor: pointer;
    }
    #play-again-btn:hover { background: #243954; }
  </style>
</head>
<body>

  <div id="score-bar">
    <span id="sc-version">gfarena0</span>
    <span>Draw pile: <strong id="sc-draw-pile">--</strong></span>
    <button id="new-game-btn">New Game</button>
  </div>

  <div id="status-msg">Initialising...</div>

  <div id="bots">
    <div class="bot-panel" id="bot-0">
      <div class="bot-name">--</div>
      <div class="bot-hand-size"></div>
      <div class="bot-books"></div>
    </div>
    <div class="bot-panel" id="bot-1">
      <div class="bot-name">--</div>
      <div class="bot-hand-size"></div>
      <div class="bot-books"></div>
    </div>
    <div class="bot-panel" id="bot-2">
      <div class="bot-name">--</div>
      <div class="bot-hand-size"></div>
      <div class="bot-books"></div>
    </div>
  </div>

  <div id="human-area">
    <div id="hand-section">
      <div id="hand-label">Your hand:</div>
      <div id="hand-cards"></div>
    </div>

    <div id="ask-section">
      <span>Ask:</span>
      <button class="target-btn" id="target-1" data-idx="1" disabled>--</button>
      <button class="target-btn" id="target-2" data-idx="2" disabled>--</button>
      <button class="target-btn" id="target-3" data-idx="3" disabled>--</button>
    </div>

    <div id="books-section">
      Your books: <span id="human-books">--</span>
    </div>

    <button id="draw-btn" style="display:none">Go Fish - Draw a card</button>
  </div>

  <div id="game-over">
    <div id="game-over-box">
      <h2 id="game-over-title"></h2>
      <div id="final-scores"></div>
      <button id="play-again-btn">Play Again</button>
    </div>
  </div>

  <script type="module">
    /* JS added in Task 7 */
  </script>
</body>
</html>
```

- [ ] **Step 6.2: Verify skeleton loads in browser**

```bash
cd gfarena0-web && make serve
```
Open `http://localhost:8080` — dark skeleton with three bot panels and "Initialising..." should appear. No JS errors.

- [ ] **Step 6.3: Commit**

```bash
cd gfarena0-web
git add www/index.html
git commit -m "feat: add index.html HTML and CSS skeleton"
```

---

## Task 7: Implement JS game logic in `www/index.html`

**Files:**
- Modify: `gfarena0-web/www/index.html`

**One-time: verify `BasicPile` JSON shape in browser console.**
After `make build`, open `http://localhost:8080` dev tools and run:
```javascript
import init, { new_human_vs_bots_game, get_state } from './pkg/gfarena0_web.js';
await init();
new_human_vs_bots_game('Standard', 'You', 3, 0);
console.log(JSON.stringify(JSON.parse(get_state()).players[0].hand, null, 2));
```
The JS code below treats `hand` as a JSON array of `{rank:{index,weight,value}, suit:{...}}` objects — the standard serde newtype serialisation for `BasicPile(Vec<BasicCard>)`. If the output is instead `{cards:[...]}` then change `for (const card of hand)` to `for (const card of hand.cards)`.

- [ ] **Step 7.1: Replace the empty script block**

Replace `/* JS added in Task 7 */` inside the `<script type="module">` block in `www/index.html`:

```javascript
import init, { version, new_human_vs_bots_game, get_state, act, step_bot }
  from './pkg/gfarena0_web.js';

// ── State ────────────────────────────────────────────────────────────────────

let state = null;
let selectedRank = null;  // rank object { index, weight, value } or null
let botLoopTimer = null;

// ── Bootstrap ────────────────────────────────────────────────────────────────

async function main() {
  await init();
  document.getElementById('sc-version').textContent = 'gfarena0 v' + version();
  document.getElementById('new-game-btn').addEventListener('click', startGame);
  document.getElementById('draw-btn').addEventListener('click', onDraw);
  document.getElementById('play-again-btn').addEventListener('click', startGame);
  document.querySelectorAll('.target-btn').forEach(function(btn) {
    btn.addEventListener('click', function() {
      onAsk(parseInt(btn.dataset.idx, 10));
    });
  });
  startGame();
}

// ── Game lifecycle ────────────────────────────────────────────────────────────

function startGame() {
  if (botLoopTimer) { clearTimeout(botLoopTimer); botLoopTimer = null; }
  selectedRank = null;
  document.getElementById('game-over').style.display = 'none';
  var json = new_human_vs_bots_game('Standard', 'You', 3, Date.now());
  state = JSON.parse(json);
  if (state.error) { setStatus('Error: ' + state.error); return; }
  setStatus('Game started - your turn!');
  render();
  scheduleLoop();
}

// ── Game loop ────────────────────────────────────────────────────────────────

function scheduleLoop() {
  state = JSON.parse(get_state());

  if (state.phase === 'GameOver') {
    render();
    showGameOver();
    return;
  }

  if (state.current_player === 0) {
    render();
    return;
  }

  render();
  botLoopTimer = setTimeout(stepBot, 700);
}

function stepBot() {
  var result = JSON.parse(step_bot());
  if (!result.done) {
    state = JSON.parse(get_state());
    setStatus(eventToStr(result.event, state.players));
    render();
  }
  scheduleLoop();
}

// ── Human actions ────────────────────────────────────────────────────────────

function onAsk(targetIdx) {
  if (!selectedRank) return;
  var json = act(JSON.stringify({ Ask: { target: targetIdx, rank: selectedRank } }));
  var event = JSON.parse(json);
  if (event.error) { setStatus('Error: ' + event.error); return; }
  selectedRank = null;
  state = JSON.parse(get_state());
  setStatus(eventToStr(event, state.players));
  scheduleLoop();
}

function onDraw() {
  var json = act('"Draw"');
  var event = JSON.parse(json);
  if (event.error) { setStatus('Error: ' + event.error); return; }
  state = JSON.parse(get_state());
  setStatus(eventToStr(event, state.players));
  scheduleLoop();
}

// ── Render ────────────────────────────────────────────────────────────────────

function render() {
  if (!state) return;

  document.getElementById('sc-draw-pile').textContent = state.draw_pile_size;

  var isHumanTurn = (state.current_player === 0);
  var isWaitingForDraw = (state.phase === 'WaitingForDraw');

  for (var i = 1; i <= 3; i++) {
    var p = state.players[i];
    var panel = document.getElementById('bot-' + (i - 1));
    panel.querySelector('.bot-name').textContent = p.name;
    panel.querySelector('.bot-hand-size').textContent = '✋ ' + p.hand_size;
    panel.querySelector('.bot-books').textContent =
      p.completed_book_ranks.length ? p.completed_book_ranks.join('  ') : '--';
    if (state.current_player === i) {
      panel.classList.add('active-turn');
    } else {
      panel.classList.remove('active-turn');
    }
  }

  document.getElementById('hand-section').style.display =
    (isHumanTurn && !isWaitingForDraw) ? 'block' : 'none';
  document.getElementById('ask-section').style.display =
    (isHumanTurn && !isWaitingForDraw) ? 'flex' : 'none';
  document.getElementById('draw-btn').style.display =
    (isHumanTurn && isWaitingForDraw) ? 'block' : 'none';

  if (isHumanTurn && !isWaitingForDraw) {
    renderHand(state.players[0].hand);
    for (var j = 1; j <= 3; j++) {
      var btn = document.getElementById('target-' + j);
      btn.textContent = state.players[j].name;
      btn.disabled = !selectedRank;
    }
  }

  var human = state.players[0];
  document.getElementById('human-books').textContent =
    human.completed_book_ranks.length
      ? human.completed_book_ranks.join('  ')
      : '--';
}

function renderHand(hand) {
  var container = document.getElementById('hand-cards');
  container.textContent = '';  // clear children safely
  if (!hand) return;

  // Group cards by rank index
  var groups = {};
  for (var i = 0; i < hand.length; i++) {
    var card = hand[i];
    var idx = card.rank.index;
    if (!groups[idx]) groups[idx] = { rank: card.rank, count: 0 };
    groups[idx].count++;
  }

  // Sort highest weight first (A, K, Q, ... 2)
  var sorted = Object.values(groups).sort(function(a, b) {
    return b.rank.weight - a.rank.weight;
  });

  sorted.forEach(function(item) {
    var btn = document.createElement('button');
    btn.className = 'rank-btn';
    if (selectedRank && selectedRank.index === item.rank.index) {
      btn.classList.add('selected');
    }
    btn.textContent = item.rank.index + ' x' + item.count;
    btn.addEventListener('click', function() {
      selectedRank = item.rank;
      renderHand(hand);
      document.querySelectorAll('.target-btn').forEach(function(b) {
        b.disabled = false;
      });
    });
    container.appendChild(btn);
  });
}

// ── Game over ─────────────────────────────────────────────────────────────────

function showGameOver() {
  var winner = state.winner;
  var title = document.getElementById('game-over-title');
  if (winner !== null) {
    var name = (state.players[winner] && state.players[winner].name)
      ? state.players[winner].name
      : ('Player ' + winner);
    title.textContent = (name === 'You') ? 'You win!' : (name + ' wins!');
  } else {
    title.textContent = "It's a tie!";
  }

  var finalScores = document.getElementById('final-scores');
  finalScores.textContent = '';
  state.players.forEach(function(p) {
    var div = document.createElement('div');
    div.textContent = p.name + ': ' + p.books + ' book' + (p.books !== 1 ? 's' : '');
    finalScores.appendChild(div);
  });

  document.getElementById('game-over').style.display = 'flex';
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function eventToStr(event, players) {
  if (!event) return '';
  function n(i) {
    return (players[i] && players[i].name) ? players[i].name : ('Player ' + i);
  }
  if (event.GoFish) {
    return 'Go Fish! ' + n(event.GoFish.player) + ' must draw.';
  }
  if (event.Gave) {
    var plural = event.Gave.count !== 1 ? 's' : '';
    return n(event.Gave.from) + ' gives ' + event.Gave.count + ' ' + event.Gave.rank + plural
      + ' to ' + n(event.Gave.to) + '.';
  }
  if (event.Drew) {
    return n(event.Drew.player) + ' draws a card'
      + (event.Drew.matched ? ' - matched!' : '') + '.';
  }
  if (event.Book) {
    return n(event.Book.player) + ' completes a book of ' + event.Book.rank + 's!';
  }
  if (event.GameOver) {
    return event.GameOver.winner !== null
      ? n(event.GameOver.winner) + ' wins!'
      : "It's a tie!";
  }
  if (event.Asked) {
    return n(event.Asked.asker) + ' asks ' + n(event.Asked.target)
      + ' for ' + event.Asked.rank + 's.';
  }
  return '';
}

function setStatus(msg) {
  document.getElementById('status-msg').textContent = msg;
}

main().catch(function(err) {
  console.error('Fatal init error:', err);
  document.getElementById('status-msg').textContent = 'Failed to load: ' + err.message;
});
```

- [ ] **Step 7.2: Rebuild and smoke-test in browser**

```bash
cd gfarena0-web && make serve
```
Open `http://localhost:8080`. Verify:
- Version label shows `gfarena0 v0.x.x`
- Three bot panels show names (Harriet, Bertram, Lucky)
- Your hand shows rank buttons like `A x2`, `7 x1`
- Clicking a rank highlights it; target buttons activate
- Clicking a target submits the ask; status updates
- Bots take turns automatically with 700ms pauses
- Game-over overlay appears at end with scores and Play Again

- [ ] **Step 7.3: Commit**

```bash
cd gfarena0-web
git add www/index.html
git commit -m "feat: implement JS game loop and UI in index.html"
```

---

## Task 8: Playwright smoke tests

**Files:**
- Create: `gfarena0-web/tests/game.spec.ts`

- [ ] **Step 8.1: Write the test file**

`gfarena0-web/tests/game.spec.ts`:
```typescript
import { test, expect } from '@playwright/test';

test.describe('gfarena0-web smoke tests', () => {

  test('page loads and version is shown', async ({ page }) => {
    await page.goto('/');
    await page.waitForFunction(
      () => {
        const el = document.getElementById('sc-version');
        return el != null && el.textContent != null && el.textContent.includes('v');
      },
      { timeout: 10_000 }
    );
    const version = await page.textContent('#sc-version');
    expect(version).toMatch(/gfarena0 v\d+\.\d+/);
  });

  test('game initialises with bot names visible', async ({ page }) => {
    await page.goto('/');
    await page.waitForFunction(
      () => {
        const el = document.querySelector('#bot-0 .bot-name');
        return el != null && el.textContent !== '--';
      },
      { timeout: 10_000 }
    );
    const bot0 = await page.textContent('#bot-0 .bot-name');
    expect(bot0).toBe('Harriet');
  });

  test('human hand shows rank buttons on their turn', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#hand-cards .rank-btn', { timeout: 15_000 });
    const count = await page.locator('#hand-cards .rank-btn').count();
    expect(count).toBeGreaterThan(0);
  });

  test('clicking rank enables target buttons', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#hand-cards .rank-btn', { timeout: 15_000 });

    await expect(page.locator('#target-1')).toBeDisabled();
    await page.locator('#hand-cards .rank-btn').first().click();
    await expect(page.locator('#target-1')).toBeEnabled();
  });

  test('submitting ask updates status line', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#hand-cards .rank-btn', { timeout: 15_000 });

    await page.locator('#hand-cards .rank-btn').first().click();
    await page.locator('#target-1').click();

    await page.waitForFunction(
      () => {
        const el = document.getElementById('status-msg');
        const txt = (el != null && el.textContent != null) ? el.textContent : '';
        return txt.length > 5 && txt !== 'Game started - your turn!';
      },
      { timeout: 5_000 }
    );
    const status = await page.textContent('#status-msg');
    expect(status!.length).toBeGreaterThan(5);
  });

  test('game reaches GameOver and shows overlay', async ({ page }) => {
    await page.goto('/');

    for (let turn = 0; turn < 200; turn++) {
      // Wait for human turn controls or game-over overlay
      try {
        await page.waitForFunction(
          () => {
            const over = document.getElementById('game-over');
            const rank = document.querySelector('#hand-cards .rank-btn');
            const draw = document.getElementById('draw-btn');
            return (over != null && over.style.display !== 'none')
              || rank != null
              || (draw != null && draw.style.display !== 'none');
          },
          { timeout: 8_000 }
        );
      } catch (_e) {
        break;
      }

      const isOver = await page.evaluate(
        () => {
          const el = document.getElementById('game-over');
          return el != null && el.style.display !== 'none';
        }
      );
      if (isOver) break;

      const drawVisible = await page.evaluate(
        () => {
          const btn = document.getElementById('draw-btn');
          return btn != null && btn.style.display !== 'none';
        }
      );
      if (drawVisible) {
        await page.click('#draw-btn');
        continue;
      }

      const hasRank = await page.locator('#hand-cards .rank-btn').count();
      if (hasRank > 0) {
        await page.locator('#hand-cards .rank-btn').first().click();
        await page.locator('.target-btn:not([disabled])').first().click();
      }
    }

    await expect(page.locator('#game-over')).toBeVisible({ timeout: 5_000 });
    const title = await page.textContent('#game-over-title');
    expect(title!.length).toBeGreaterThan(3);
  });

});
```

- [ ] **Step 8.2: Run the tests**

```bash
cd gfarena0-web && make test 2>&1 | tail -20
```
Expected: `5 passed`.

- [ ] **Step 8.3: Commit**

```bash
cd gfarena0-web
git add tests/game.spec.ts
git commit -m "test: add Playwright smoke tests for gfarena0-web"
```

---

## Self-Review Notes

- **Spec coverage:** All four spec sections covered — gfcore changes (Tasks 1-3), project structure (Tasks 4-5), JS game loop (Task 7), UI layout (Tasks 6-7), Playwright tests (Task 8).
- **BasicPile serialisation:** Step 7.1 includes an explicit browser-console verification step before the JS code is relied on. If `hand` is `{cards:[...]}` rather than an array, change `hand[i]` iteration to `hand.cards[i]`.
- **Type consistency:** `selectedRank` is always the `rank` field object from a card; passed verbatim to `act()`. `completed_book_ranks` is always `Vec<String>` in Rust / JS string array.
- **Safe DOM:** `textContent` and `appendChild` used throughout instead of `innerHTML` to prevent XSS.
- **TDD sequence:** Tasks 1 and 3 follow red-green-commit. Task 2 is a type-safe refactor verified by existing tests.
