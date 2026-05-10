//! Integration test: 100-game bot marathon with all 4 standard bot profiles.
#![allow(non_snake_case)]
//!
//! Plays 100 games of Standard Go Fish with 4 bot players (2 `BasicStrategy`,
//! 2 `RandomStrategy`). After every game the test:
//!   - asserts book conservation (total books ≤ 13 for a Standard52 deck)
//!   - validates the winner field is consistent with player count
//!   - serializes the `GameRecord` to YAML, deserializes it, and asserts
//!     round-trip equality
//!
//! The built-in game-over check and deadlock detection in [`Game::act`] are
//! exercised automatically on every game.
//!
//! On any error the full collection is serialized to a YAML file and the path
//! is included in the panic message for offline debugging.
//!
//! Run with:
//! ```text
//! cargo test --test bot_marathon -- --include-ignored --nocapture
//! ```

#![cfg(feature = "history")]

use gfcore::bot::BotProfile;
use gfcore::history::{GameCollection, GameRecord};
use gfcore::prelude::{Game, GamePhase, GameVariant, Player, PlayerAction};

const NUM_GAMES: usize = 100;
const PROGRESS_INTERVAL: usize = 10;
const MAX_STEPS_PER_GAME: usize = 13_000;
const MAX_BOOKS_STANDARD: usize = 13;

/// Serializes the full game collection to YAML, writes it to a file, and
/// panics with a summary message.
///
/// The output path is controlled by the `MARATHON_DUMP_PATH` environment
/// variable (default: `marathon_failure.yaml`), letting CI point an artifact
/// upload step at the right location.
///
/// Returns `!` so it can be used in `unwrap_or_else` closures.
fn dump_and_panic(game_num: usize, context: &str, msg: String, collection: &GameCollection) -> ! {
    let yaml = collection
        .to_yaml()
        .unwrap_or_else(|e| format!("(YAML serialization also failed: {e})"));
    let path =
        std::env::var("MARATHON_DUMP_PATH").unwrap_or_else(|_| "marathon_failure.yaml".to_string());
    let _ = std::fs::write(&path, &yaml);
    panic!(
        "bot_marathon FAILED at game {game_num} [{context}]: {msg}\n\
         (YAML written to {path} — download the CI artifact if the log is truncated)"
    );
}

/// Plays one Standard Go Fish game to completion using the given bot profiles.
///
/// Each player is assigned to `profiles[player_index % profiles.len()]`.
/// Returns the completed `GameRecord` from `Game::record()`, or calls
/// [`dump_and_panic`] if the game errors or fails to terminate.
fn play_one_game(
    game_num: usize,
    profiles: &[BotProfile],
    collection: &GameCollection,
) -> GameRecord {
    let players: Vec<Player> = profiles
        .iter()
        .map(|p| Player::new(p.name.clone()))
        .collect();

    let mut game = match Game::new(GameVariant::Standard, players) {
        Ok(g) => g,
        Err(e) => dump_and_panic(game_num, "Game::new", e.to_string(), collection),
    };

    for _ in 0..MAX_STEPS_PER_GAME {
        if game.is_over() {
            break;
        }

        let state = match game.state() {
            Ok(s) => s,
            Err(e) => dump_and_panic(game_num, "state", e.to_string(), collection),
        };

        let action = match state.phase {
            GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                let cp = state.current_player;
                let hand = state
                    .players
                    .iter()
                    .find(|v| v.index == cp)
                    .and_then(|v| v.hand.as_ref())
                    .cloned()
                    .unwrap_or_default();
                profiles[cp % profiles.len()].decide(&hand, &state.players, &state.ask_log)
            }
            GamePhase::WaitingForDraw => PlayerAction::Draw,
            GamePhase::GameOver => break,
        };

        if let Err(e) = game.act(action) {
            dump_and_panic(game_num, "act", e.to_string(), collection);
        }
    }

    if !game.is_over() {
        dump_and_panic(
            game_num,
            "termination",
            format!("game did not terminate within {MAX_STEPS_PER_GAME} steps"),
            collection,
        );
    }

    game.record()
}

/// Validates the most recently pushed game in `collection`.
///
/// Checks:
/// - book conservation (total books ≤ `MAX_BOOKS_STANDARD`)
/// - winner index in range
/// - YAML round-trip equality
///
/// On any failure calls [`dump_and_panic`] with the full collection.
fn validate_last_game(game_num: usize, collection: &GameCollection) {
    let record = match collection.iter().last() {
        Some(r) => r,
        None => dump_and_panic(
            game_num,
            "validate",
            "collection is empty".to_string(),
            collection,
        ),
    };

    // Book conservation: a Standard52 deck yields at most 13 books.
    let total_books: usize = record
        .turns
        .last()
        .map(|t| t.books_after_turn.iter().sum())
        .unwrap_or(0);

    if total_books > MAX_BOOKS_STANDARD {
        dump_and_panic(
            game_num,
            "book_conservation",
            format!("total books {total_books} exceeds maximum of {MAX_BOOKS_STANDARD}"),
            collection,
        );
    }

    // Winner index must be a valid player index when set.
    if let Some(winner) = record.winner {
        if winner >= record.players.len() {
            dump_and_panic(
                game_num,
                "winner_range",
                format!(
                    "winner index {winner} out of range (players: {})",
                    record.players.len()
                ),
                collection,
            );
        }
    }

    // Structural audit: book conservation, winner consistency, turn integrity.
    let audit = record.audit();
    if !audit.is_consistent {
        dump_and_panic(
            game_num,
            "audit",
            format!("audit violations: {:?}", audit.violations),
            collection,
        );
    }

    // YAML round-trip must preserve all fields exactly.
    let yaml = record
        .to_yaml()
        .unwrap_or_else(|e| dump_and_panic(game_num, "to_yaml", e.to_string(), collection));
    let parsed = GameRecord::from_yaml(&yaml)
        .unwrap_or_else(|e| dump_and_panic(game_num, "from_yaml", e.to_string(), collection));
    if record != &parsed {
        dump_and_panic(
            game_num,
            "round_trip",
            "YAML round-trip produced unequal record".to_string(),
            collection,
        );
    }
}

/// Plays 100 games of Standard Go Fish with all 4 default bot profiles,
/// validating book conservation and YAML round-trip after every game.
///
/// Run with:
///
/// ```text
/// cargo test --test bot_marathon -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "marathon: 100 games with 4 bot profiles; use --include-ignored"]
fn bot_marathon__100_games_without_error() {
    let profiles = BotProfile::default_profiles();
    let mut collection = GameCollection::new();

    for game_num in 1..=NUM_GAMES {
        let record = play_one_game(game_num, &profiles, &collection);
        collection.push(record);
        validate_last_game(game_num, &collection);

        if game_num % PROGRESS_INTERVAL == 0 {
            println!("bot_marathon: {game_num}/{NUM_GAMES} games complete");
        }
    }

    println!("bot_marathon: complete — {NUM_GAMES} games played without error");
}
