//! Integration test: play a full game, build a [`GameRecord`], and assert
//! round-trip equality via YAML and JSON serialization.
//!
//! The bot strategy is the same round-robin rank selector used in
//! `game_integration.rs` — deterministic enough for testing, random enough
//! (due to deck shuffle) to cover realistic event sequences.
//!
//! This module is compiled only when the `history` feature is enabled (the
//! default).

#![cfg(feature = "history")]

use gfcore::bot::BotProfile;
use gfcore::history::{GameCollection, GameRecord, TurnRecord};
use gfcore::prelude::{Game, GameEvent, GamePhase, GameVariant, Player, PlayerAction};

// ---------------------------------------------------------------------------
// Bot helper — identical to game_integration.rs
// ---------------------------------------------------------------------------

struct BotCounters {
    counters: Vec<usize>,
}

impl BotCounters {
    fn new(player_count: usize) -> Self {
        Self {
            counters: vec![0; player_count],
        }
    }

    fn choose_rank(
        &mut self,
        player_idx: usize,
        hand: &cardpack::prelude::BasicPile,
    ) -> Option<cardpack::prelude::Pip> {
        if hand.is_empty() {
            return None;
        }
        let mut seen = std::collections::HashSet::new();
        let mut ranks: Vec<cardpack::prelude::Pip> = Vec::new();
        for card in hand {
            if seen.insert(card.rank) {
                ranks.push(card.rank);
            }
        }
        ranks.sort_by(|a, b| b.weight.cmp(&a.weight));

        if player_idx >= self.counters.len() {
            self.counters.resize(player_idx + 1, 0);
        }
        let idx = self.counters[player_idx] % ranks.len();
        self.counters[player_idx] += 1;
        Some(ranks[idx])
    }
}

// ---------------------------------------------------------------------------
// Game runner with history recording
// ---------------------------------------------------------------------------

/// Play a full 2-player Standard game to completion, recording every event.
///
/// Returns a fully populated [`GameRecord`].
fn play_and_record() -> GameRecord {
    let player_names = vec!["Alice".to_string(), "Bob".to_string()];
    let players = vec![Player::new("Alice"), Player::new("Bob")];
    let mut game = Game::new(GameVariant::Standard, players).expect("valid 2-player game");
    let mut bots = BotCounters::new(2);

    let mut record = GameRecord::new("Standard", player_names);

    // Track the current turn being built.
    let mut current_turn_player = game.current_player();
    let mut current_turn_events: Vec<GameEvent> = Vec::new();

    // Budget: same as game_integration.rs.
    for _ in 0..13_000 {
        if game.is_over() {
            break;
        }

        let state = game.state().expect("state must be available");
        let cp_before = game.current_player();

        let event = match state.phase {
            GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                let cp = state.current_player;
                let hand = state
                    .players
                    .iter()
                    .find(|v| v.index == cp)
                    .and_then(|v| v.hand.as_ref())
                    .expect("current player must see their own hand");

                let rank = bots
                    .choose_rank(cp, hand)
                    .expect("hand must be non-empty when WaitingForAsk");
                let target = state
                    .players
                    .iter()
                    .find(|v| v.index != cp)
                    .expect("at least one other player")
                    .index;

                game.act(PlayerAction::Ask { target, rank })
                    .expect("round-robin ask must not error")
            }
            GamePhase::WaitingForDraw => game.act(PlayerAction::Draw).expect("draw must not error"),
            GamePhase::GameOver => break,
        };

        let is_game_over = matches!(event, GameEvent::GameOver { .. });
        current_turn_events.push(event.clone());

        let cp_after = game.current_player();
        let phase_after = game.phase().clone();

        // Determine whether the current turn is finished.
        // A turn ends when:
        //  - The game is over, OR
        //  - The current player has changed (advance_turn was called), AND
        //    the new phase is WaitingForAsk (not mid-draw of same player).
        let turn_ended = is_game_over
            || (cp_after != cp_before
                && matches!(phase_after, GamePhase::WaitingForAsk | GamePhase::GameOver));

        if turn_ended {
            // Snapshot book counts from the final state.
            let final_state = game.state().expect("state after turn");
            let books_after_turn: Vec<usize> =
                final_state.players.iter().map(|p| p.books).collect();

            record.turns.push(TurnRecord {
                player: current_turn_player,
                events: std::mem::take(&mut current_turn_events),
                books_after_turn,
                actions: None,
            });

            // Start fresh for the next turn.
            current_turn_player = cp_after;
        }

        if is_game_over {
            record.winner = if let GameEvent::GameOver { winner } = event {
                winner
            } else {
                None
            };
            break;
        }
    }

    // If the game ended but there are leftover events (shouldn't happen in
    // normal flow, but guard defensively), flush them.
    if !current_turn_events.is_empty() {
        let final_state = game.state().expect("state after flush");
        let books_after_turn: Vec<usize> = final_state.players.iter().map(|p| p.books).collect();
        record.turns.push(TurnRecord {
            player: current_turn_player,
            events: current_turn_events,
            books_after_turn,
            actions: None,
        });
    }

    assert!(game.is_over(), "game must reach GameOver within budget");
    record
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_game_record_yaml_round_trip() {
    let record = play_and_record();

    // Sanity: record has turns and a variant name.
    assert!(
        !record.turns.is_empty(),
        "played game must have at least one turn"
    );
    assert_eq!(record.variant, "Standard");

    // Serialize → deserialize → assert equality.
    let yaml = record.to_yaml().expect("to_yaml must succeed");
    assert!(!yaml.is_empty());

    let parsed = GameRecord::from_yaml(&yaml).expect("from_yaml must succeed");
    assert_eq!(record, parsed, "YAML round-trip must preserve all fields");
}

#[test]
fn test_game_record_json_round_trip() {
    let record = play_and_record();

    let json = record.to_json().expect("to_json must succeed");
    assert!(!json.is_empty());

    let parsed = GameRecord::from_json(&json).expect("from_json must succeed");
    assert_eq!(record, parsed, "JSON round-trip must preserve all fields");
}

#[test]
fn test_game_collection_yaml_round_trip() {
    let record = play_and_record();

    let mut col = GameCollection::new();
    col.push(record.clone());
    assert_eq!(col.len(), 1);

    let yaml = col.to_yaml().expect("collection to_yaml must succeed");
    let parsed_col = GameCollection::from_yaml(&yaml).expect("collection from_yaml must succeed");
    assert_eq!(col, parsed_col, "collection YAML round-trip must be equal");

    // The individual record inside the collection must also match.
    assert_eq!(parsed_col[0], record);
}

#[test]
fn test_game_collection_json_round_trip() {
    let record = play_and_record();

    let mut col = GameCollection::new();
    col.push(record.clone());

    let json = col.to_json().expect("collection to_json must succeed");
    let parsed_col = GameCollection::from_json(&json).expect("collection from_json must succeed");
    assert_eq!(col, parsed_col, "collection JSON round-trip must be equal");
}

#[test]
fn test_game_record_has_game_over_winner() {
    // The winner field must be consistent: if set, index must be in range.
    let record = play_and_record();
    if let Some(winner) = record.winner {
        assert!(
            winner < record.players.len(),
            "winner index {winner} must be within players range"
        );
    }
}

#[test]
fn test_turn_records_cover_all_players() {
    // In a 2-player game both players must appear as turn owners at least once.
    let record = play_and_record();
    let player_count = record.players.len();
    for expected_player in 0..player_count {
        assert!(
            record.turns.iter().any(|t| t.player == expected_player),
            "player {expected_player} must have taken at least one turn"
        );
    }
}

#[test]
fn test_each_turn_has_at_least_one_event() {
    let record = play_and_record();
    for (i, turn) in record.turns.iter().enumerate() {
        assert!(
            !turn.events.is_empty(),
            "turn {i} (player {}) must have at least one event",
            turn.player
        );
    }
}

/// Play a full game using only `Game::act()` + `Game::record()` — no manual
/// event accumulation — and verify the auto-recorded `GameRecord` is valid.
#[test]
fn test_game_record_auto_records_turns() {
    let players = vec![Player::new("Alice"), Player::new("Bob")];
    let mut game = Game::new(GameVariant::Standard, players).expect("valid 2-player game");
    let mut bots = BotCounters::new(2);

    for _ in 0..13_000 {
        if game.is_over() {
            break;
        }
        let state = game.state().expect("state available");
        let cp = state.current_player;

        match state.phase {
            GamePhase::WaitingForAsk | GamePhase::BookCompleted => {
                let hand = state
                    .players
                    .iter()
                    .find(|v| v.index == cp)
                    .and_then(|v| v.hand.as_ref())
                    .expect("current player must see hand");
                let rank = bots.choose_rank(cp, hand).expect("non-empty hand");
                let target = state
                    .players
                    .iter()
                    .find(|v| v.index != cp)
                    .expect("at least one other player")
                    .index;
                game.act(PlayerAction::Ask { target, rank })
                    .expect("ask must not error");
            }
            GamePhase::WaitingForDraw => {
                game.act(PlayerAction::Draw).expect("draw must not error");
            }
            GamePhase::GameOver => break,
        }
    }

    assert!(game.is_over(), "game must finish within budget");

    let record = game.record();

    assert_eq!(record.variant, "Standard Go Fish");
    assert_eq!(record.players, vec!["Alice", "Bob"]);
    assert!(
        !record.turns.is_empty(),
        "auto-recorded game must have turns"
    );

    for (i, turn) in record.turns.iter().enumerate() {
        assert!(
            !turn.events.is_empty(),
            "auto-recorded turn {i} must have at least one event"
        );
        assert!(turn.player < 2, "turn {i} player index must be in range");
    }

    // Both players must appear as turn owners.
    for expected_player in 0..2 {
        assert!(
            record.turns.iter().any(|t| t.player == expected_player),
            "player {expected_player} must own at least one turn in auto-record"
        );
    }

    // YAML round-trip must preserve the record.
    let yaml = record.to_yaml().expect("to_yaml must succeed");
    let parsed = GameRecord::from_yaml(&yaml).expect("from_yaml must succeed");
    assert_eq!(
        record.turns.len(),
        parsed.turns.len(),
        "YAML round-trip must preserve turn count"
    );
    assert_eq!(
        record.winner, parsed.winner,
        "YAML round-trip must preserve winner"
    );
}

#[test]
fn test_game_collection_multiple_records() {
    // Push two independently played games into a collection and round-trip.
    let r1 = play_and_record();
    let r2 = play_and_record();

    let mut col = GameCollection::new();
    col.push(r1.clone());
    col.push(r2.clone());
    assert_eq!(col.len(), 2);

    let yaml = col.to_yaml().expect("serialize");
    let back = GameCollection::from_yaml(&yaml).expect("deserialize");
    assert_eq!(col, back);
    assert_eq!(back[0], r1);
    assert_eq!(back[1], r2);
}

// ---------------------------------------------------------------------------
// Helper: play using Game::act() so actions are auto-recorded
// ---------------------------------------------------------------------------

/// Plays a Standard Go Fish game to completion using two `BotProfile` bots.
///
/// Returns `game.record()`, which has `TurnRecord::actions` populated for
/// every turn (enabling replay) and `initial_draw_pile` set.
fn play_game_with_bot_profiles() -> GameRecord {
    let profiles = [BotProfile::basic("Alice"), BotProfile::basic("Bob")];
    let players = vec![
        Player::new("Alice".to_string()),
        Player::new("Bob".to_string()),
    ];
    let mut game = Game::new(GameVariant::Standard, players).expect("valid 2-player game");

    for _ in 0..13_000 {
        if game.is_over() {
            break;
        }
        let state = game.state().expect("state available");
        let cp = state.current_player;
        let action = match state.phase {
            GamePhase::WaitingForDraw => PlayerAction::Draw,
            GamePhase::GameOver => break,
            _ => {
                let hand = state
                    .players
                    .iter()
                    .find(|v| v.index == cp)
                    .and_then(|v| v.hand.as_ref())
                    .cloned()
                    .unwrap_or_default();
                profiles[cp % profiles.len()].decide(&hand, &state.players, &state.ask_log)
            }
        };
        game.act(action).expect("bot action must not error");
    }

    assert!(game.is_over(), "game must finish within budget");
    game.record()
}

// ---------------------------------------------------------------------------
// Audit integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_audit_all_on_played_collection() {
    let mut col = GameCollection::new();
    for _ in 0..5 {
        col.push(play_game_with_bot_profiles());
    }

    let results = col.audit_all();
    assert_eq!(results.len(), 5, "must have one audit result per game");

    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_consistent,
            "game {i} audit failed with violations: {:?}",
            result.violations
        );
    }
}

// ---------------------------------------------------------------------------
// Replay integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_replay_all_on_played_collection() {
    let mut col = GameCollection::new();
    for _ in 0..5 {
        col.push(play_game_with_bot_profiles());
    }

    let results = col.replay_all();
    assert_eq!(results.len(), 5, "must have one replay result per game");

    for (i, result) in results.iter().enumerate() {
        let result = result
            .as_ref()
            .unwrap_or_else(|e| panic!("game {i} replay returned Err: {e}"));
        assert!(
            result.is_consistent,
            "game {i} replay diverged at turn {:?}",
            result.mismatch_at_turn
        );
    }
}
