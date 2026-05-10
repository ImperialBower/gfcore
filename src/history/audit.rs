//! Structural audit of [`GameRecord`] and [`GameCollection`].

use serde::{Deserialize, Serialize};

use super::record::{GameCollection, GameRecord};

/// The result of a structural audit of a single [`GameRecord`].
///
/// Produced by [`GameRecord::audit`] and collected by [`GameCollection::audit_all`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditResult {
    /// The game ID from the audited record.
    pub game_id: String,
    /// `true` iff no violations were found.
    pub is_consistent: bool,
    /// Book counts per player as of the last recorded turn (empty if no turns).
    pub final_books: Vec<usize>,
    /// Human-readable violation descriptions. Empty when `is_consistent`.
    pub violations: Vec<String>,
}

impl GameRecord {
    /// Validates structural invariants of this record.
    ///
    /// Infallible — violations accumulate in [`AuditResult::violations`]
    /// rather than returning an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::GameRecord;
    ///
    /// let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
    /// let result = record.audit();
    /// assert!(result.is_consistent);
    /// assert!(result.violations.is_empty());
    /// ```
    #[must_use]
    pub fn audit(&self) -> AuditResult {
        let mut violations: Vec<String> = Vec::new();
        let player_count = self.players.len();

        // Check 1: at least 2 players.
        if player_count < 2 {
            violations.push(format!(
                "player count is {player_count}; must be at least 2"
            ));
        }

        for (i, turn) in self.turns.iter().enumerate() {
            // Check 2: turn player index in range.
            if turn.player >= player_count {
                violations.push(format!(
                    "turn {i}: player index {} out of range (players: {player_count})",
                    turn.player
                ));
            }

            // Check 3: books_after_turn length matches player count.
            if turn.books_after_turn.len() != player_count {
                violations.push(format!(
                    "turn {i}: books_after_turn length {} != player count {player_count}",
                    turn.books_after_turn.len()
                ));
            }

            // Check 4: events must not be empty.
            if turn.events.is_empty() {
                violations.push(format!("turn {i}: events list is empty"));
            }
        }

        // Check 5: book counts non-decreasing per player.
        for (i, turn) in self.turns.iter().enumerate().skip(1) {
            let prev = &self.turns[i - 1].books_after_turn;
            let curr = &turn.books_after_turn;
            if prev.len() == player_count && curr.len() == player_count {
                for p in 0..player_count {
                    if curr[p] < prev[p] {
                        violations.push(format!(
                            "player {p} book count decreased from {} to {} at turn {i}",
                            prev[p], curr[p]
                        ));
                    }
                }
            }
        }

        // Check 6: total books <= 13 (Standard52 deck maximum).
        if let Some(last) = self.turns.last() {
            let total: usize = last.books_after_turn.iter().sum();
            if total > 13 {
                violations.push(format!(
                    "total books {total} exceeds maximum of 13 (Standard52 deck yields at most 13 books)"
                ));
            }
        }

        // Check 7: winner consistent with final book counts.
        if let Some(last) = self.turns.last() {
            let books = &last.books_after_turn;
            if books.len() == player_count && player_count >= 2 {
                let max_books = *books.iter().max().unwrap_or(&0);
                let leaders: Vec<usize> = (0..player_count)
                    .filter(|&p| books[p] == max_books)
                    .collect();
                let unique_leader = leaders.len() == 1;

                match self.winner {
                    Some(w) => {
                        if w >= player_count {
                            violations.push(format!(
                                "winner index {w} is out of range (players: {player_count})"
                            ));
                        } else if !unique_leader {
                            violations.push(format!(
                                "winner declared as player {w} but final books are tied (books: {books:?})"
                            ));
                        } else if leaders[0] != w {
                            violations.push(format!(
                                "winner declared as player {w} but player {} has more books (books: {books:?})",
                                leaders[0]
                            ));
                        }
                    }
                    None => {
                        if unique_leader && max_books > 0 {
                            violations.push(format!(
                                "winner is None but player {} has unique max book count of \
                                 {max_books} (books: {books:?})",
                                leaders[0]
                            ));
                        }
                    }
                }
            }
        }

        let final_books = self
            .turns
            .last()
            .map(|t| t.books_after_turn.clone())
            .unwrap_or_default();

        AuditResult {
            game_id: self.id.clone(),
            is_consistent: violations.is_empty(),
            final_books,
            violations,
        }
    }
}

impl GameCollection {
    /// Audits every game in this collection and returns one [`AuditResult`] per game.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::history::{GameCollection, GameRecord};
    ///
    /// let mut col = GameCollection::new();
    /// col.push(GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]));
    /// let results = col.audit_all();
    /// assert_eq!(results.len(), 1);
    /// assert!(results[0].is_consistent);
    /// ```
    pub fn audit_all(&self) -> Vec<AuditResult> {
        self.games.iter().map(GameRecord::audit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameEvent;
    use crate::history::record::{GameCollection, TurnRecord};

    fn make_clean_record(player_count: usize, turn_count: usize) -> GameRecord {
        let players: Vec<String> = (0..player_count).map(|i| format!("P{i}")).collect();
        let mut r = GameRecord::new("Standard", players);
        for t in 0..turn_count {
            r.turns.push(TurnRecord {
                player: t % player_count,
                events: vec![GameEvent::Drew {
                    player: t % player_count,
                    matched: false,
                }],
                books_after_turn: vec![0; player_count],
                actions: None,
            });
        }
        r
    }

    #[test]
    fn test_audit_clean_record_is_consistent() {
        let record = make_clean_record(2, 3);
        let result = record.audit();
        assert!(result.is_consistent);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_audit_empty_record_no_turns_is_consistent() {
        let record = GameRecord::new("Standard", vec!["Alice".to_string(), "Bob".to_string()]);
        let result = record.audit();
        assert!(result.is_consistent);
        assert!(result.violations.is_empty());
        assert!(result.final_books.is_empty());
    }

    #[test]
    fn test_audit_single_player_is_violation() {
        let record = GameRecord::new("Standard", vec!["Solo".to_string()]);
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("player")));
    }

    #[test]
    fn test_audit_turn_player_out_of_range() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 5,
            events: vec![GameEvent::Drew {
                player: 5,
                matched: false,
            }],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_books_after_turn_wrong_length() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![0], // should be length 2
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_empty_events_in_turn() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![],
            books_after_turn: vec![0, 0],
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("turn 0")));
    }

    #[test]
    fn test_audit_book_counts_decreasing_is_violation() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![2, 0],
            actions: None,
        });
        record.turns.push(TurnRecord {
            player: 1,
            events: vec![GameEvent::Drew {
                player: 1,
                matched: false,
            }],
            books_after_turn: vec![1, 0], // player 0 decreased
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("decreas")));
    }

    #[test]
    fn test_audit_total_books_exceeds_13() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![10, 5], // 15 > 13
            actions: None,
        });
        let result = record.audit();
        assert!(!result.is_consistent);
        // The violation message includes the actual total (15).
        assert!(result.violations.iter().any(|v| v.contains("15")));
    }

    #[test]
    fn test_audit_winner_some_but_another_has_more_books() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![3, 5],
            actions: None,
        });
        record.winner = Some(0); // player 1 has more books
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_some_but_tied() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![5, 5],
            actions: None,
        });
        record.winner = Some(0); // tied — no unique winner
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_none_but_clear_leader() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![5, 3],
            actions: None,
        });
        record.winner = None; // player 0 has unique max — must be declared winner
        let result = record.audit();
        assert!(!result.is_consistent);
        assert!(result.violations.iter().any(|v| v.contains("winner")));
    }

    #[test]
    fn test_audit_winner_correct_unique_max() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![5, 3],
            actions: None,
        });
        record.winner = Some(0);
        let result = record.audit();
        assert!(result.is_consistent, "violations: {:?}", result.violations);
    }

    #[test]
    fn test_audit_winner_none_correct_when_tied() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![4, 4],
            actions: None,
        });
        record.winner = None;
        let result = record.audit();
        assert!(result.is_consistent, "violations: {:?}", result.violations);
    }

    #[test]
    fn test_audit_final_books_populated_from_last_turn() {
        let mut record = make_clean_record(2, 0);
        record.turns.push(TurnRecord {
            player: 0,
            events: vec![GameEvent::Drew {
                player: 0,
                matched: false,
            }],
            books_after_turn: vec![2, 3],
            actions: None,
        });
        record.winner = Some(1);
        let result = record.audit();
        assert_eq!(result.final_books, vec![2, 3]);
    }

    #[test]
    fn test_audit_all_consistent_collection() {
        let mut col = GameCollection::new();
        col.push(make_clean_record(2, 3));
        col.push(make_clean_record(3, 4));
        let results = col.audit_all();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_consistent));
    }
}
