//! WASM bindings for `gfcore`.
//!
//! This module is compiled only when the `wasm` feature is enabled.  It
//! exposes a thread-local game state and a set of `#[wasm_bindgen]`-exported
//! functions that JavaScript callers can use to drive a Go Fish game.
//!
//! All exported functions return JSON strings.  Errors are returned as
//! `{"error": "..."}` so JavaScript never receives a thrown exception from
//! this layer.
//!
//! # Quick example (JavaScript)
//!
//! ```javascript
//! import init, { new_game, act, get_state, step_bot } from './gfcore.js';
//! await init();
//! const state = JSON.parse(new_game("Standard", '["Alice","Bob"]', 42));
//! console.log(state.current_player); // 0
//! ```

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use crate::bot::BotProfile;
use crate::game::{Game, GameEvent, GamePhase, PlayerAction};
use crate::player::Player;
use crate::rules::GameVariant;

// ---------------------------------------------------------------------------
// Thread-locals
// ---------------------------------------------------------------------------

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
    static PROFILES: RefCell<Vec<Option<BotProfile>>> = const { RefCell::new(Vec::new()) };
    static LAST_EVENT: RefCell<Option<GameEvent>> = const { RefCell::new(None) };
}

// ---------------------------------------------------------------------------
// Panic hook
// ---------------------------------------------------------------------------

/// Installs the `console_error_panic_hook` so that Rust panics appear in the
/// browser console rather than being silently swallowed.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Formats a JSON error object from an error message string.
fn error_json(msg: &str) -> String {
    format!("{{\"error\":{}}}", serde_json::json!(msg))
}

/// Parses a variant name string into a [`GameVariant`].
fn parse_variant(s: &str) -> Result<GameVariant, String> {
    match s {
        "Standard" => Ok(GameVariant::Standard),
        "HappyFamilies" => Ok(GameVariant::HappyFamilies),
        "Quartet" => Ok(GameVariant::Quartet),
        _ => Err(format!("unknown variant: {s}")),
    }
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

/// Returns the crate version string.
///
/// # Examples (JavaScript)
///
/// ```javascript
/// console.log(version()); // "0.1.0"
/// ```
#[must_use]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Creates a new game with named human players.
///
/// - `variant`: `"Standard"`, `"HappyFamilies"`, or `"Quartet"`.
/// - `player_names_json`: JSON array of strings, e.g. `'["Alice","Bob"]'`.
/// - `_seed`: reserved for future reproducible shuffle support; currently
///   the deck shuffle uses the system RNG and this value is ignored.
///
/// Returns the initial [`crate::game::GameState`] as JSON, or
/// `{"error": "..."}` on failure.
#[must_use]
#[wasm_bindgen]
pub fn new_game(variant: &str, player_names_json: &str, _seed: f64) -> String {
    let names: Vec<String> = match serde_json::from_str(player_names_json) {
        Ok(n) => n,
        Err(e) => return error_json(&format!("invalid player_names_json: {e}")),
    };

    let game_variant = match parse_variant(variant) {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    let players: Vec<Player> = names.into_iter().map(Player::new).collect();

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
    PROFILES.with(|p| p.borrow_mut().clear());

    state_json
}

/// Creates a new game with `bot_count` bot players using the default profiles.
///
/// - `variant`: `"Standard"`, `"HappyFamilies"`, or `"Quartet"`.
/// - `bot_count`: number of bot players (1–4, bounded by available default profiles).
/// - `_seed`: reserved for future reproducible shuffle support; currently ignored.
///
/// Returns the initial [`crate::game::GameState`] as JSON, or
/// `{"error": "..."}` on failure.
#[must_use]
#[wasm_bindgen]
pub fn new_bot_game(variant: &str, bot_count: usize, _seed: f64) -> String {
    let game_variant = match parse_variant(variant) {
        Ok(v) => v,
        Err(e) => return error_json(&e),
    };

    let mut all_profiles = BotProfile::default_profiles();
    if bot_count > all_profiles.len() {
        return error_json(&format!(
            "bot_count {bot_count} exceeds available default profiles ({})",
            all_profiles.len()
        ));
    }
    let bot_profiles: Vec<BotProfile> = all_profiles.drain(..bot_count).collect();

    let players: Vec<Player> = bot_profiles
        .iter()
        .map(|p| Player::new_bot(p.name.clone(), p.clone()))
        .collect();

    let profiles: Vec<Option<BotProfile>> = bot_profiles.into_iter().map(Some).collect();

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

    state_json
}

/// Creates a new game with one human player (index 0) and `bot_count` bot players.
///
/// - `variant`: `"Standard"`, `"HappyFamilies"`, or `"Quartet"`.
/// - `human_name`: display name for player 0 (the human).
/// - `bot_count`: number of bot players (1–4); bots fill slots 1..=`bot_count`
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

/// Submits a player action and advances the game state.
///
/// `action_json` must be one of:
/// - `{"Ask":{"target":1,"rank":{"weight":12,"index":"A","value":0}}}` for an ask
/// - `"Draw"` for a draw
///
/// Returns the resulting [`crate::game::GameEvent`] as JSON, or
/// `{"error": "..."}` on failure.
#[must_use]
#[wasm_bindgen]
pub fn act(action_json: &str) -> String {
    let action: crate::game::PlayerAction = match serde_json::from_str(action_json) {
        Ok(a) => a,
        Err(e) => return error_json(&format!("invalid action JSON: {e}")),
    };

    GAME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        match borrow.as_mut() {
            None => error_json("no game in progress"),
            Some(game) => match game.act(action) {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => return error_json(&e.to_string()),
                    };
                    LAST_EVENT.with(|le| *le.borrow_mut() = Some(event));
                    json
                }
                Err(e) => error_json(&e.to_string()),
            },
        }
    })
}

/// Returns the current [`crate::game::GameState`] as JSON.
///
/// Returns `{"error": "no game in progress"}` if no game has been started.
#[must_use]
#[wasm_bindgen]
pub fn get_state() -> String {
    GAME.with(|cell| {
        let borrow = cell.borrow();
        match borrow.as_ref() {
            None => error_json("no game in progress"),
            Some(game) => match game.state() {
                Ok(s) => match serde_json::to_string(&s) {
                    Ok(j) => j,
                    Err(e) => error_json(&e.to_string()),
                },
                Err(e) => error_json(&e.to_string()),
            },
        }
    })
}

/// If the current player is a bot, computes and applies their action.
///
/// Returns:
/// - `{"done":false,"event":<GameEvent JSON>}` after the bot acted.
/// - `{"done":true}` if it is not a bot's turn or the game is over.
/// - `{"error":"..."}` on failure.
#[must_use]
#[wasm_bindgen]
pub fn step_bot() -> String {
    // Check game-over and current player index first.
    let (is_over, current_player) = GAME.with(|cell| {
        let borrow = cell.borrow();
        match borrow.as_ref() {
            None => (true, 0), // treat missing game as "done"
            Some(game) => (game.is_over(), game.current_player()),
        }
    });

    if is_over {
        return "{\"done\":true}".to_string();
    }

    // Check whether the current player has a bot profile.
    let profile_opt: Option<BotProfile> = PROFILES.with(|p| {
        let profiles = p.borrow();
        profiles.get(current_player).and_then(Clone::clone)
    });

    let Some(profile) = profile_opt else {
        return "{\"done\":true}".to_string();
    };

    // Get the current game state so the bot can see its hand and the ask log.
    let state_opt = GAME.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().and_then(|game| game.state().ok())
    });

    let Some(state) = state_opt else {
        return error_json("no game in progress");
    };

    // The bot's hand is visible in the observer-perspective view.
    let hand = match state.players.get(current_player) {
        None => return error_json("current_player index out of range"),
        Some(view) => match view.hand.clone() {
            None => return error_json("bot hand not visible in state"),
            Some(h) => h,
        },
    };

    // In WaitingForDraw the bot must draw, not ask.
    let action = if state.phase == GamePhase::WaitingForDraw {
        PlayerAction::Draw
    } else {
        profile.decide(&hand, &state.players, &state.ask_log)
    };

    // Apply the action.
    let event_json = GAME.with(|cell| {
        let mut borrow = cell.borrow_mut();
        match borrow.as_mut() {
            None => error_json("no game in progress"),
            Some(game) => match game.act(action) {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => return error_json(&e.to_string()),
                    };
                    LAST_EVENT.with(|le| *le.borrow_mut() = Some(event));
                    json
                }
                Err(e) => error_json(&e.to_string()),
            },
        }
    });

    // If the event_json itself is an error object, return it directly.
    if event_json.starts_with("{\"error\"") {
        return event_json;
    }

    format!("{{\"done\":false,\"event\":{event_json}}}")
}

/// Returns the full game history as YAML.
///
/// Returns `{"error":"..."}` if no game is in progress, the game is not yet
/// over, or the `history` feature is not enabled.
///
/// # Examples (JavaScript)
///
/// ```javascript
/// const yaml = get_game_yaml();
/// if (!yaml.startsWith('{"error"')) console.log(yaml);
/// ```
#[must_use]
#[wasm_bindgen]
pub fn get_game_yaml() -> String {
    #[cfg(feature = "history")]
    {
        GAME.with(|cell| {
            let borrow = cell.borrow();
            match borrow.as_ref() {
                None => error_json("no game in progress"),
                Some(game) => match game.record().to_yaml() {
                    Ok(yaml) => yaml,
                    Err(e) => error_json(&e.to_string()),
                },
            }
        })
    }
    #[cfg(not(feature = "history"))]
    {
        error_json("history feature is not enabled")
    }
}

/// Parses a YAML [`crate::history::GameCollection`] string and returns it as JSON.
///
/// Requires the `history` feature.  Returns `{"error":"..."}` when the
/// feature is absent or when parsing fails.
#[must_use]
#[wasm_bindgen]
pub fn parse_game_collection(yaml: &str) -> String {
    #[cfg(feature = "history")]
    {
        match crate::history::GameCollection::from_yaml(yaml) {
            Ok(col) => match serde_json::to_string(&col) {
                Ok(j) => j,
                Err(e) => error_json(&e.to_string()),
            },
            Err(e) => error_json(&e.to_string()),
        }
    }
    #[cfg(not(feature = "history"))]
    {
        let _ = yaml;
        error_json("history feature is not enabled")
    }
}
