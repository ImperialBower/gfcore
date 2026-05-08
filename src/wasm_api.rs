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
#[mutants::skip] // panic-hook installation is not observable from Rust tests
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
/// console.log(version()); // e.g. "0.0.1" — returns the crate version string
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
/// Works for both in-progress and completed games; call at any time after
/// [`new_game`] to snapshot the current history.
///
/// Returns `{"error":"..."}` if no game is in progress or the `history`
/// feature is not enabled.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Clears all thread-local game state between tests.
    ///
    /// Rust's test runner reuses OS threads across tests, so thread-locals can
    /// carry state from one test into another.  Call this at the top of any
    /// test that depends on a clean initial state.
    fn reset_state() {
        GAME.with(|g| *g.borrow_mut() = None);
        PROFILES.with(|p| p.borrow_mut().clear());
        LAST_EVENT.with(|le| *le.borrow_mut() = None);
    }

    fn is_error_json(s: &str) -> bool {
        s.contains("\"error\"")
    }

    fn parse(json: &str) -> serde_json::Value {
        serde_json::from_str(json).unwrap_or(serde_json::Value::Null)
    }

    // --- error_json ---

    #[test]
    fn test_error_json_contains_error_key_and_message() {
        let result = error_json("something went wrong");
        assert!(is_error_json(&result));
        let v = parse(&result);
        assert!(
            v["error"].as_str().is_some(),
            "error field must be a string: {result}"
        );
    }

    // --- parse_variant ---

    #[test]
    fn test_parse_variant_standard() {
        assert!(matches!(
            parse_variant("Standard"),
            Ok(GameVariant::Standard)
        ));
    }

    #[test]
    fn test_parse_variant_happy_families() {
        assert!(matches!(
            parse_variant("HappyFamilies"),
            Ok(GameVariant::HappyFamilies)
        ));
    }

    #[test]
    fn test_parse_variant_quartet() {
        assert!(matches!(parse_variant("Quartet"), Ok(GameVariant::Quartet)));
    }

    #[test]
    fn test_parse_variant_unknown_returns_err() {
        assert!(parse_variant("Unknown").is_err());
    }

    // --- version ---

    #[test]
    fn test_version_matches_cargo_pkg_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    // --- new_game ---

    #[test]
    fn test_new_game_valid_returns_state_json() {
        reset_state();
        let result = new_game("Standard", r#"["Alice","Bob"]"#, 0.0);
        assert!(!is_error_json(&result), "unexpected error: {result}");
        let v = parse(&result);
        assert_eq!(v["phase"], "WaitingForAsk");
        assert_eq!(v["current_player"], 0);
    }

    #[test]
    fn test_new_game_unknown_variant_returns_error() {
        let result = new_game("Bogus", r#"["Alice","Bob"]"#, 0.0);
        assert!(is_error_json(&result));
    }

    #[test]
    fn test_new_game_bad_names_json_returns_error() {
        let result = new_game("Standard", "not-json", 0.0);
        assert!(is_error_json(&result));
    }

    // --- new_bot_game ---

    #[test]
    fn test_new_bot_game_valid_returns_state_json() {
        reset_state();
        let result = new_bot_game("Standard", 2, 0.0);
        assert!(!is_error_json(&result), "unexpected error: {result}");
        let v = parse(&result);
        assert_eq!(v["phase"], "WaitingForAsk");
        assert_eq!(v["players"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_new_bot_game_at_profile_limit_succeeds() {
        // bot_count == limit must succeed; catches the >= mutation on the > guard.
        reset_state();
        let limit = BotProfile::default_profiles().len();
        let result = new_bot_game("Standard", limit, 0.0);
        assert!(
            !is_error_json(&result),
            "bot_count at limit should succeed: {result}"
        );
    }

    #[test]
    fn test_new_bot_game_exceeds_profile_limit_returns_error() {
        // bot_count > limit must error; catches the == mutation on the > guard.
        let limit = BotProfile::default_profiles().len();
        let result = new_bot_game("Standard", limit + 1, 0.0);
        assert!(
            is_error_json(&result),
            "bot_count over limit should error: {result}"
        );
    }

    // --- new_human_vs_bots_game ---

    #[test]
    fn test_new_human_vs_bots_game_valid_returns_state_json() {
        reset_state();
        let result = new_human_vs_bots_game("Standard", "Alice", 2, 0.0);
        assert!(!is_error_json(&result), "unexpected error: {result}");
        let v = parse(&result);
        assert_eq!(v["players"].as_array().unwrap().len(), 3);
        assert_eq!(v["players"][0]["name"], "Alice");
    }

    #[test]
    fn test_new_human_vs_bots_game_zero_bots_returns_error() {
        // Catches the != mutation on the bot_count == 0 guard.
        let result = new_human_vs_bots_game("Standard", "Alice", 0, 0.0);
        assert!(is_error_json(&result));
    }

    #[test]
    fn test_new_human_vs_bots_game_one_bot_succeeds() {
        // bot_count=1 must succeed; catches the != mutation (which would wrongly error).
        reset_state();
        let result = new_human_vs_bots_game("Standard", "Alice", 1, 0.0);
        assert!(!is_error_json(&result), "1 bot should succeed: {result}");
    }

    #[test]
    fn test_new_human_vs_bots_game_at_profile_limit_succeeds() {
        // bot_count == limit must succeed; catches the >= mutation on the > guard.
        reset_state();
        let limit = BotProfile::default_profiles().len();
        let result = new_human_vs_bots_game("Standard", "Alice", limit, 0.0);
        assert!(
            !is_error_json(&result),
            "bot_count at limit should succeed: {result}"
        );
    }

    #[test]
    fn test_new_human_vs_bots_game_exceeds_profile_limit_returns_error() {
        // bot_count > limit must error; catches the == mutation on the > guard.
        let limit = BotProfile::default_profiles().len();
        let result = new_human_vs_bots_game("Standard", "Alice", limit + 1, 0.0);
        assert!(
            is_error_json(&result),
            "bot_count over limit should error: {result}"
        );
    }

    // --- act ---

    #[test]
    fn test_act_bad_json_returns_error() {
        let result = act("not-json");
        assert!(is_error_json(&result));
    }

    #[test]
    fn test_act_no_game_in_progress_returns_error() {
        reset_state();
        let result = act("\"Draw\"");
        assert!(is_error_json(&result));
    }

    #[test]
    fn test_act_wrong_phase_returns_error() {
        reset_state();
        let _ = new_game("Standard", r#"["Alice","Bob"]"#, 0.0);
        // Game starts in WaitingForAsk; Draw is invalid in this phase.
        let result = act("\"Draw\"");
        assert!(
            is_error_json(&result),
            "Draw in WaitingForAsk should error: {result}"
        );
    }

    // --- get_state ---

    #[test]
    fn test_get_state_no_game_returns_error() {
        reset_state();
        let result = get_state();
        assert!(is_error_json(&result));
    }

    #[test]
    fn test_get_state_after_new_game_returns_state_json() {
        reset_state();
        let _ = new_game("Standard", r#"["Alice","Bob"]"#, 0.0);
        let result = get_state();
        assert!(!is_error_json(&result), "unexpected error: {result}");
        let v = parse(&result);
        assert_eq!(v["phase"], "WaitingForAsk");
    }

    // --- step_bot ---

    #[test]
    fn test_step_bot_no_game_returns_done() {
        reset_state();
        // Missing game is treated as "done" (no bot to step).
        assert_eq!(step_bot(), "{\"done\":true}");
    }

    #[test]
    fn test_step_bot_human_player_returns_done() {
        reset_state();
        let _ = new_human_vs_bots_game("Standard", "Alice", 2, 0.0);
        // Player 0 is human — no bot profile, so step_bot must return done:true.
        let result = step_bot();
        let v = parse(&result);
        assert_eq!(
            v["done"], true,
            "human turn must return done:true: {result}"
        );
    }

    #[test]
    fn test_step_bot_bot_game_returns_valid_response() {
        reset_state();
        let _ = new_bot_game("Standard", 2, 0.0);
        let result = step_bot();
        assert!(!is_error_json(&result), "step_bot must not error: {result}");
        let v = parse(&result);
        assert!(v.get("done").is_some(), "must have 'done' key: {result}");
    }

    /// Exercises the `state.phase == WaitingForDraw` branch in `step_bot`.
    ///
    /// With the `== → !=` mutation the bot would call `decide()` (returning an
    /// Ask action) instead of `Draw`, causing an `OutOfTurn` error from the
    /// game engine.  Asserting no error here catches that mutation.
    #[test]
    fn test_step_bot_draws_when_in_waiting_for_draw_phase() {
        reset_state();
        let _ = new_bot_game("Standard", 2, 0.0);
        for _ in 0..500 {
            let state = parse(&get_state());
            match state["phase"].as_str().unwrap_or("") {
                "GameOver" => return,
                "WaitingForDraw" => {
                    let result = step_bot();
                    assert!(
                        !is_error_json(&result),
                        "step_bot in WaitingForDraw must not error: {result}"
                    );
                    let v = parse(&result);
                    assert_eq!(v["done"], false, "expected done=false: {result}");
                    return;
                }
                _ => {
                    let _ = step_bot();
                }
            }
        }
    }

    // --- get_game_yaml ---

    #[cfg(feature = "history")]
    #[test]
    fn test_get_game_yaml_no_game_returns_error() {
        reset_state();
        let result = get_game_yaml();
        assert!(is_error_json(&result));
    }

    #[cfg(feature = "history")]
    #[test]
    fn test_get_game_yaml_with_active_game_returns_yaml() {
        reset_state();
        let _ = new_bot_game("Standard", 2, 0.0);
        let result = get_game_yaml();
        assert!(
            !is_error_json(&result),
            "get_game_yaml must not error: {result}"
        );
        assert!(!result.is_empty(), "yaml must not be empty");
    }

    // --- parse_game_collection ---

    #[test]
    fn test_parse_game_collection_invalid_input_returns_error() {
        let result = parse_game_collection("definitely not a game collection");
        assert!(is_error_json(&result));
    }

    #[cfg(feature = "history")]
    #[test]
    fn test_parse_game_collection_valid_yaml_returns_json() {
        let yaml = crate::history::GameCollection::new().to_yaml().unwrap();
        let result = parse_game_collection(&yaml);
        assert!(!is_error_json(&result), "valid yaml should parse: {result}");
        assert!(!result.is_empty(), "result must not be empty");
    }
}
