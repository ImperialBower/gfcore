//! Engine-replay verification for [`GameRecord`] and [`GameCollection`].
//!
//! Provides [`ReplayResult`] and the [`GameRecord::replay`] and
//! [`GameCollection::replay_all`] methods.  Both re-run stored
//! [`crate::history::TurnRecord`] actions through a fresh
//! [`crate::game::Game`] engine instance and compare the replayed turn state
//! to the stored data.

use crate::error::GfError;
use crate::game::Game;
use crate::player::Player;
use crate::rules::GameVariant;

use super::record::{GameCollection, GameRecord};

/// Maps a stored variant-name string back to a [`GameVariant`].
///
/// The stored string is produced by `variant.rules().name()` at game creation
/// and saved into [`GameRecord::variant`].
fn parse_variant(name: &str) -> Result<GameVariant, GfError> {
    match name {
        "Standard Go Fish" => Ok(GameVariant::Standard),
        "Happy Families" => Ok(GameVariant::HappyFamilies),
        "Quartet" => Ok(GameVariant::Quartet),
        other => Err(GfError::ParseError(format!(
            "unknown game variant: {other:?}"
        ))),
    }
}

/// The result of replaying a [`GameRecord`] through a fresh engine instance.
///
/// Produced by [`GameRecord::replay`] and collected by
/// [`GameCollection::replay_all`].
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
    /// - [`GfError::NoReplayData`] — at least one turn has `actions: None`, or
    ///   the record has no `initial_draw_pile` (recorded without the replay path).
    /// - [`GfError::ParseError`] — the variant name is not recognised.
    /// - Any engine error propagated from [`crate::game::Game::act`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// // A record with no turns is trivially consistent.
    /// let record = GameRecord::new(
    ///     "Standard Go Fish",
    ///     vec!["Alice".to_string(), "Bob".to_string()],
    /// );
    /// let result = record.replay().expect("empty record replay must succeed");
    /// assert!(result.is_consistent);
    /// ```
    pub fn replay(&self) -> Result<ReplayResult, GfError> {
        // A record with no turns is trivially consistent with no engine needed.
        if self.turns.is_empty() {
            return Ok(ReplayResult {
                game_id: self.id.clone(),
                is_consistent: true,
                final_books: vec![],
                mismatch_at_turn: None,
            });
        }

        // Collect all action lists upfront; returns Err if any turn has actions: None.
        let all_actions = self
            .turns
            .iter()
            .map(|turn| turn.actions.clone().ok_or(GfError::NoReplayData))
            .collect::<Result<Vec<_>, _>>()?;

        let draw_pile = self
            .initial_draw_pile
            .clone()
            .ok_or(GfError::NoReplayData)?;

        let variant = parse_variant(&self.variant)?;
        let players: Vec<Player> = self
            .players
            .iter()
            .map(|name| Player::new(name.clone()))
            .collect();
        let mut game = Game::new_with_deck(variant, players, draw_pile)?;

        // Replay all actions. Engine errors (e.g. InvalidAsk from data corruption) propagate.
        for (_, actions) in self.turns.iter().zip(all_actions) {
            for action in actions {
                game.act(action)?;
            }
        }

        // Single snapshot after replay is complete — avoids O(n²) cloning.
        let final_record = game.record();

        // Compare per-turn book counts; report the first divergence.
        for (i, (stored, replayed)) in self.turns.iter().zip(final_record.turns.iter()).enumerate()
        {
            if replayed.books_after_turn != stored.books_after_turn {
                return Ok(ReplayResult {
                    game_id: self.id.clone(),
                    is_consistent: false,
                    final_books: replayed.books_after_turn.clone(),
                    mismatch_at_turn: Some(i),
                });
            }
        }

        // Structural mismatch: engine produced a different number of turns.
        if final_record.turns.len() != self.turns.len() {
            let mismatch_at_turn = self.turns.len().min(final_record.turns.len());
            let final_books = final_record
                .turns
                .last()
                .map(|t| t.books_after_turn.clone())
                .unwrap_or_default();
            return Ok(ReplayResult {
                game_id: self.id.clone(),
                is_consistent: false,
                final_books,
                mismatch_at_turn: Some(mismatch_at_turn),
            });
        }

        let is_consistent = final_record.winner == self.winner;
        let final_books = final_record
            .turns
            .last()
            .map(|t| t.books_after_turn.clone())
            .unwrap_or_default();

        Ok(ReplayResult {
            game_id: self.id.clone(),
            is_consistent,
            final_books,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::BotProfile;
    use crate::error::GfError;
    use crate::game::{Game, GamePhase, PlayerAction};
    use crate::history::record::{GameCollection, GameRecord, TurnRecord};
    use crate::player::Player;
    use crate::rules::GameVariant;

    /// Plays a two-player bot game of Standard Go Fish to completion.
    fn play_to_completion() -> GameRecord {
        let profiles = [BotProfile::basic("Alice"), BotProfile::basic("Bob")];
        let players = vec![
            Player::new("Alice".to_string()),
            Player::new("Bob".to_string()),
        ];
        let mut game = Game::new(GameVariant::Standard, players).unwrap();

        while !game.is_over() {
            let state = game.state().unwrap();
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
            game.act(action).unwrap();
        }

        game.record()
    }

    #[test]
    fn test_replay_no_actions_returns_error() {
        let mut record = GameRecord::new(
            "Standard Go Fish",
            vec!["Alice".to_string(), "Bob".to_string()],
        );
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let result = record.replay();
        assert!(
            matches!(result, Err(GfError::NoReplayData)),
            "expected Err(NoReplayData), got {result:?}"
        );
    }

    #[test]
    fn test_replay_consistent_game() {
        let record = play_to_completion();

        assert!(
            record.turns.iter().all(|t| t.actions.is_some()),
            "all turns must have stored actions"
        );
        assert!(
            record.initial_draw_pile.is_some(),
            "record must have initial_draw_pile for replay"
        );

        let result = record
            .replay()
            .expect("replay of a completed game must succeed");
        assert!(
            result.is_consistent,
            "replay must be consistent; mismatch at turn: {:?}",
            result.mismatch_at_turn
        );
        assert!(result.mismatch_at_turn.is_none());
    }

    #[test]
    fn test_replay_all_returns_vec() {
        let record = play_to_completion();
        let mut col = GameCollection::new();
        col.push(record);

        let results = col.replay_all();
        assert_eq!(results.len(), 1);
        let result = results[0].as_ref().expect("replay must succeed");
        assert!(
            result.is_consistent,
            "replayed game must be consistent; mismatch at turn: {:?}",
            result.mismatch_at_turn
        );
    }
}
