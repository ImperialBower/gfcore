//! Uniformly random bot strategy.
//!
//! [`RandomStrategy`] asks for a random rank from the bot's hand and targets
//! a random other player.  It ignores the ask log entirely.

use cardpack::prelude::BasicPile;
use rand::Rng as _;

use crate::game::PlayerAction;
use crate::player::{AskEntry, PlayerView};

use super::BotStrategy;

/// A bot strategy that makes uniformly random valid decisions.
///
/// Both the rank and the target are chosen uniformly at random:
/// - **Rank**: a random card drawn from the hand; its rank is requested.
/// - **Target**: a random index that is not the bot's own index.
///
/// The strategy never inspects the ask log.
///
/// # Examples
///
/// ```
/// use cardpack::prelude::{BasicPile, FrenchBasicCard};
/// use gfcore::bot::strategy::{BotStrategy, RandomStrategy};
/// use gfcore::prelude::{Player, PlayerAction, PlayerView};
///
/// let strategy = RandomStrategy;
///
/// let mut alice = Player::new("Alice");
/// alice.receive_card(FrenchBasicCard::ACE_SPADES);
/// let mut bob = Player::new("Bob");
/// bob.receive_card(FrenchBasicCard::KING_HEARTS);
///
/// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
/// let hand = players[0].hand.clone().unwrap_or_default();
///
/// let action = strategy.decide(&hand, &players, &[]);
/// assert!(matches!(action, PlayerAction::Ask { target: 1, .. }));
/// ```
#[derive(Debug, Clone)]
pub struct RandomStrategy;

impl BotStrategy for RandomStrategy {
    /// Decides by picking a uniformly random rank from `hand` and a uniformly
    /// random non-self player from `players`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::bot::strategy::{BotStrategy, RandomStrategy};
    /// use gfcore::prelude::{Player, PlayerAction, PlayerView};
    ///
    /// let strategy = RandomStrategy;
    ///
    /// let mut alice = Player::new("Alice");
    /// alice.receive_card(FrenchBasicCard::ACE_SPADES);
    /// let mut bob = Player::new("Bob");
    /// bob.receive_card(FrenchBasicCard::KING_HEARTS);
    ///
    /// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
    /// let hand = players[0].hand.clone().unwrap_or_default();
    ///
    /// let action = strategy.decide(&hand, &players, &[]);
    /// assert!(matches!(action, PlayerAction::Ask { .. }));
    /// ```
    fn decide(
        &self,
        hand: &BasicPile,
        players: &[PlayerView],
        _ask_log: &[AskEntry],
    ) -> PlayerAction {
        // Precondition: hand must be non-empty. The game engine enforces this,
        // but guard defensively so external callers never trigger a panic.
        if hand.is_empty() {
            return PlayerAction::Draw;
        }

        let mut rng = rand::rng();

        // Identify the observer (the bot) — the one view that has `hand: Some`.
        let observer = players
            .iter()
            .position(|player_view| player_view.hand.is_some())
            .unwrap_or(0);

        // Pick a random card from hand and use its rank.
        // Safety: hand.v() is non-empty (guarded above) and the index is < len.
        let cards = hand.v();
        let card_index = rng.random_range(0..cards.len());
        #[allow(clippy::indexing_slicing)]
        let rank = cards[card_index].rank;

        // Build a list of valid (non-self) target indices.
        let targets: Vec<usize> = (0..players.len()).filter(|&idx| idx != observer).collect();

        let target = if targets.is_empty() {
            // Degenerate case: only one player (should not happen in a real game).
            (observer + 1) % players.len().max(2)
        } else {
            let pick = rng.random_range(0..targets.len());
            targets[pick]
        };

        PlayerAction::Ask { target, rank }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::FrenchBasicCard;

    fn two_player_views() -> (BasicPile, Vec<PlayerView>) {
        use crate::player::Player;

        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        alice.receive_card(FrenchBasicCard::ACE_HEARTS);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::KING_HEARTS);

        let views = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
        let hand = views[0].hand.clone().unwrap_or_default();
        (hand, views)
    }

    #[test]
    fn test_random_strategy_returns_ask() {
        let strategy = RandomStrategy;
        let (hand, players) = two_player_views();
        let action = strategy.decide(&hand, &players, &[]);
        assert!(matches!(action, PlayerAction::Ask { .. }));
    }

    #[test]
    fn test_random_strategy_target_is_not_self() {
        let strategy = RandomStrategy;
        let (hand, players) = two_player_views();
        // Run several times to rule out lucky one-off passes.
        for _ in 0..20 {
            if let PlayerAction::Ask { target, .. } = strategy.decide(&hand, &players, &[]) {
                assert_ne!(target, 0, "target must not be the bot's own index");
            }
        }
    }

    #[test]
    fn test_random_strategy_rank_in_hand() {
        let strategy = RandomStrategy;
        let (hand, players) = two_player_views();
        let held: std::collections::HashSet<_> = hand.v().iter().map(|card| card.rank).collect();
        for _ in 0..20 {
            if let PlayerAction::Ask { rank, .. } = strategy.decide(&hand, &players, &[]) {
                assert!(held.contains(&rank), "rank must come from hand");
            }
        }
    }
}
