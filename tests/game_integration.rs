//! Integration test: drive a 2-player game of Go Fish to completion.
//!
//! The test uses a deterministic "round-robin" bot strategy:
//!
//! - When `WaitingForAsk`: the current player cycles through the ranks in
//!   their hand in order of descending `Pip::weight`.  A per-player counter
//!   advances on every ask so that every held rank is eventually requested,
//!   guaranteeing termination when a productive ask exists.
//! - When `WaitingForDraw`: the current player draws.
//!
//! Because the deck is shuffled randomly, the exact event sequence varies each
//! run.  What the test asserts is that:
//!
//! 1. The game always reaches `GameOver`.
//! 2. The sum of all books across players is ≤ 13 (max for a 52-card deck).
//! 3. Any reported winner index is valid.

use gfcore::prelude::{Game, GameEvent, GamePhase, GameVariant, Player, PlayerAction};

// -----------------------------------------------------------------------
// Bot state
// -----------------------------------------------------------------------

/// Per-game bot state: each player has a counter that cycles through ranks.
struct BotCounters {
    /// `counters[i]` is the current round-robin offset for player `i`.
    counters: Vec<usize>,
}

impl BotCounters {
    fn new(player_count: usize) -> Self {
        Self {
            counters: vec![0; player_count],
        }
    }

    /// Pick the next rank for `player_idx` by cycling through their hand's
    /// unique ranks sorted by descending weight.  Returns `None` if the hand
    /// is empty.
    fn choose_rank(
        &mut self,
        player_idx: usize,
        hand: &cardpack::prelude::BasicPile,
    ) -> Option<cardpack::prelude::Pip> {
        if hand.is_empty() {
            return None;
        }
        // Collect unique ranks, sorted descending by weight (stable order).
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

// -----------------------------------------------------------------------
// Single-step helper
// -----------------------------------------------------------------------

/// Advance one game step using the round-robin bot.
///
/// Returns `true` if the game is still running.
fn step(game: &mut Game, bots: &mut BotCounters) -> bool {
    if game.is_over() {
        return false;
    }
    let state = game.state().unwrap();
    match state.phase {
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
                .expect("round-robin ask must not error");
        }
        GamePhase::WaitingForDraw => {
            game.act(PlayerAction::Draw)
                .expect("draw must not error when WaitingForDraw");
        }
        GamePhase::GameOver => return false,
    }
    !game.is_over()
}

// -----------------------------------------------------------------------
// Game runner
// -----------------------------------------------------------------------

/// Play a full 2-player game to completion.  Returns the final [`GameEvent`].
///
/// Panics if the game does not terminate within a generous iteration budget.
fn play_to_completion() -> GameEvent {
    let players = vec![Player::new("Alice"), Player::new("Bob")];
    let mut game = Game::new(GameVariant::Standard, players).expect("valid 2-player Standard game");
    let mut bots = BotCounters::new(2);

    // Budget: 13 books × up to 1000 attempts per book = 13 000.
    // In practice 52-card games finish in well under 500 steps.
    for _ in 0..13_000 {
        if !step(&mut game, &mut bots) {
            break;
        }
    }

    assert!(game.is_over(), "game must terminate within 13000 steps");
    game.state()
        .unwrap()
        .last_event
        .expect("finished game must have at least one event")
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[test]
fn test_two_player_game_reaches_game_over() {
    let event = play_to_completion();
    assert!(
        matches!(event, GameEvent::GameOver { .. }),
        "final event must be GameOver, got {event:?}"
    );
}

#[test]
fn test_two_player_game_books_sum_at_most_thirteen() {
    // Run five independent games (different shuffles) to stress the engine.
    for _run in 0..5 {
        let players = vec![Player::new("Alice"), Player::new("Bob")];
        let mut game = Game::new(GameVariant::Standard, players).unwrap();
        let mut bots = BotCounters::new(2);

        for _ in 0..13_000 {
            if !step(&mut game, &mut bots) {
                break;
            }
        }
        assert!(game.is_over(), "game must terminate");

        let state = game.state().unwrap();
        let total_books: usize = state.players.iter().map(|p| p.books).sum();
        assert!(
            total_books <= 13,
            "cannot exceed 13 books in a 52-card game; got {total_books}"
        );
        assert!(total_books > 0, "at least one book must be completed");
    }
}

#[test]
fn test_two_player_game_winner_valid() {
    for _run in 0..5 {
        let players = vec![Player::new("Alice"), Player::new("Bob")];
        let mut game = Game::new(GameVariant::Standard, players).unwrap();
        let mut bots = BotCounters::new(2);

        for _ in 0..13_000 {
            if !step(&mut game, &mut bots) {
                break;
            }
        }
        assert!(game.is_over());

        let state = game.state().unwrap();
        assert_eq!(state.phase, GamePhase::GameOver);

        if let Some(winner) = state.winner {
            assert!(
                winner < state.players.len(),
                "winner index {winner} out of range"
            );
            let max_books = state.players.iter().map(|p| p.books).max().unwrap_or(0);
            assert_eq!(
                state.players[winner].books, max_books,
                "winner must have the most books"
            );
        }
    }
}

#[test]
fn test_game_already_over_returns_error() {
    use gfcore::prelude::GfError;

    let players = vec![Player::new("Alice"), Player::new("Bob")];
    let mut game = Game::new(GameVariant::Standard, players).unwrap();
    let mut bots = BotCounters::new(2);

    for _ in 0..13_000 {
        if !step(&mut game, &mut bots) {
            break;
        }
    }
    assert!(game.is_over(), "game must terminate");

    let result = game.act(PlayerAction::Draw);
    assert!(
        matches!(result, Err(GfError::GameAlreadyOver)),
        "expected GameAlreadyOver after game ended, got {result:?}"
    );
}

#[test]
fn test_state_snapshot_serializes_and_deserializes() {
    let players = vec![Player::new("Alice"), Player::new("Bob")];
    let game = Game::new(GameVariant::Standard, players).expect("valid game");
    let state = game.state().unwrap();
    let json = serde_json::to_string(&state).expect("state must serialize to JSON");
    let back: gfcore::prelude::GameState =
        serde_json::from_str(&json).expect("state must deserialize from JSON");
    assert_eq!(back.current_player, state.current_player);
    assert_eq!(back.draw_pile_size, state.draw_pile_size);
    assert_eq!(back.players.len(), state.players.len());
}
