//! Basic heuristic bot strategy.
//!
//! [`BasicStrategy`] makes simple but informed decisions:
//!
//! - **Rank**: asks for the rank held in the greatest quantity in its own hand.
//!   Ties are broken by rank index string in descending lexicographic order for
//!   a stable, deterministic result.
//! - **Target**: scans the ask log from the end and picks the most recent
//!   player (other than itself) who made an ask.  Falls back to a random
//!   non-self player when the ask log provides no usable entry.

use cardpack::prelude::BasicPile;
use rand::Rng as _;

use crate::game::PlayerAction;
use crate::player::{AskEntry, PlayerView};

use super::BotStrategy;

/// A heuristic bot strategy that leverages hand composition and the ask log.
///
/// See the [module documentation](self) for the full decision algorithm.
///
/// # Examples
///
/// ```
/// use cardpack::prelude::{BasicPile, FrenchBasicCard};
/// use gfcore::bot::strategy::{BasicStrategy, BotStrategy};
/// use gfcore::prelude::{Player, PlayerAction, PlayerView};
///
/// let strategy = BasicStrategy;
///
/// let mut alice = Player::new("Alice");
/// alice.receive_card(FrenchBasicCard::ACE_SPADES);
/// alice.receive_card(FrenchBasicCard::ACE_HEARTS);
/// let mut bob = Player::new("Bob");
/// bob.receive_card(FrenchBasicCard::KING_HEARTS);
///
/// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
/// let hand = players[0].hand.clone().unwrap_or_default();
///
/// let action = strategy.decide(&hand, &players, &[]);
/// // Alice holds two Aces — BasicStrategy should ask for Aces.
/// if let PlayerAction::Ask { rank, .. } = action {
///     assert_eq!(rank.index, 'A');
/// } else {
///     panic!("expected Ask action");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BasicStrategy;

impl BotStrategy for BasicStrategy {
    /// Decides using a hand-count heuristic for rank selection and the ask log
    /// for target selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::bot::strategy::{BasicStrategy, BotStrategy};
    /// use gfcore::prelude::{AskEntry, Player, PlayerAction, PlayerView};
    ///
    /// let strategy = BasicStrategy;
    ///
    /// let mut alice = Player::new("Alice");
    /// alice.receive_card(FrenchBasicCard::ACE_SPADES);
    /// alice.receive_card(FrenchBasicCard::ACE_HEARTS);
    /// let mut bob = Player::new("Bob");
    /// bob.receive_card(FrenchBasicCard::KING_HEARTS);
    ///
    /// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
    /// let hand = players[0].hand.clone().unwrap_or_default();
    ///
    /// // Without a log, falls back to a random non-self target.
    /// let action = strategy.decide(&hand, &players, &[]);
    /// assert!(matches!(action, PlayerAction::Ask { .. }));
    /// ```
    fn decide(
        &self,
        hand: &BasicPile,
        players: &[PlayerView],
        ask_log: &[AskEntry],
    ) -> PlayerAction {
        // Precondition: hand must be non-empty. The game engine enforces this,
        // but guard defensively so external callers never trigger a panic.
        if hand.is_empty() {
            return PlayerAction::Draw;
        }

        // Identify the bot's own player index.
        let observer = players
            .iter()
            .position(|player_view| player_view.hand.is_some())
            .unwrap_or(0);

        // --- Rank selection: highest count in hand, tie-broken by index desc ---
        let rank = {
            // Count occurrences of each unique rank.
            let mut rank_counts: Vec<(cardpack::prelude::Pip, usize)> = {
                let mut counts: std::collections::HashMap<cardpack::prelude::Pip, usize> =
                    std::collections::HashMap::new();
                for card in hand.v() {
                    *counts.entry(card.rank).or_insert(0) += 1;
                }
                counts.into_iter().collect()
            };

            // Sort by count descending, then by rank index string descending for
            // stable tie-breaking without needing a numeric weight.
            rank_counts.sort_by(|(rank_a, count_a), (rank_b, count_b)| {
                count_b
                    .cmp(count_a)
                    .then_with(|| rank_b.index.cmp(&rank_a.index))
            });

            // hand is non-empty (guarded above) so rank_counts is non-empty.
            match rank_counts.first() {
                Some((pip, _)) => *pip,
                // Unreachable: rank_counts is non-empty when hand is non-empty.
                None => return PlayerAction::Draw,
            }
        };

        // --- Target selection: most recent other-player entry in ask log ---
        let target = ask_log
            .iter()
            .rev()
            .find(|entry| entry.asker != observer)
            .map_or_else(
                || {
                    // Fall back: pick a random non-self player.
                    let mut rng = rand::rng();
                    let targets: Vec<usize> =
                        (0..players.len()).filter(|&idx| idx != observer).collect();
                    if targets.is_empty() {
                        (observer + 1) % players.len().max(2)
                    } else {
                        let pick = rng.random_range(0..targets.len());
                        targets[pick]
                    }
                },
                |entry| entry.asker,
            );

        PlayerAction::Ask { target, rank }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::FrenchBasicCard;

    fn two_player_views_alice_two_aces() -> (BasicPile, Vec<PlayerView>) {
        use crate::player::Player;

        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        alice.receive_card(FrenchBasicCard::ACE_HEARTS);
        alice.receive_card(FrenchBasicCard::KING_SPADES);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::QUEEN_HEARTS);

        let views = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
        let hand = views[0].hand.clone().unwrap_or_default();
        (hand, views)
    }

    #[test]
    fn test_basic_strategy_picks_most_common_rank() {
        let strategy = BasicStrategy;
        let (hand, players) = two_player_views_alice_two_aces();

        let action = strategy.decide(&hand, &players, &[]);
        if let PlayerAction::Ask { rank, .. } = action {
            // Alice holds 2 Aces and 1 King; should ask for Aces.
            assert_eq!(rank.index, 'A');
        } else {
            panic!("expected Ask action");
        }
    }

    #[test]
    fn test_basic_strategy_targets_last_other_asker() {
        let strategy = BasicStrategy;
        let (hand, players) = two_player_views_alice_two_aces();

        // Bob (index 1) asked last.
        let log = vec![
            AskEntry {
                asker: 0,
                rank: "K".to_string(),
            },
            AskEntry {
                asker: 1,
                rank: "Q".to_string(),
            },
        ];

        let action = strategy.decide(&hand, &players, &log);
        if let PlayerAction::Ask { target, .. } = action {
            assert_eq!(target, 1, "should target the most recent other asker");
        } else {
            panic!("expected Ask action");
        }
    }

    #[test]
    fn test_basic_strategy_fallback_when_log_empty() {
        let strategy = BasicStrategy;
        let (hand, players) = two_player_views_alice_two_aces();

        let action = strategy.decide(&hand, &players, &[]);
        if let PlayerAction::Ask { target, .. } = action {
            assert_ne!(target, 0, "target must not be the bot itself");
        } else {
            panic!("expected Ask action");
        }
    }

    #[test]
    fn test_basic_strategy_skips_self_in_log() {
        use crate::player::Player;

        // Three-player game: Alice(0), Bob(1), Carol(2); Alice is the bot.
        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::KING_HEARTS);
        let mut carol = Player::new("Carol");
        carol.receive_card(FrenchBasicCard::QUEEN_HEARTS);

        let views = PlayerView::from_perspective(&[alice, bob, carol], 0).unwrap();
        let hand = views[0].hand.clone().unwrap_or_default();

        // Log: Alice asked last (most recent), then Carol asked before that.
        let log = vec![
            AskEntry {
                asker: 2,
                rank: "Q".to_string(),
            }, // Carol
            AskEntry {
                asker: 0,
                rank: "A".to_string(),
            }, // Alice (self — skip)
        ];

        let action = BasicStrategy.decide(&hand, &views, &log);
        if let PlayerAction::Ask { target, .. } = action {
            // Most recent non-self asker is Carol (index 2).
            assert_eq!(target, 2);
        } else {
            panic!("expected Ask action");
        }
    }
}
