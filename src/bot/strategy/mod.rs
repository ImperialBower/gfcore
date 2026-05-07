//! Bot decision-making strategies.
//!
//! The [`BotStrategy`] trait is the single extension point: implement it to
//! create custom AI behaviours.  Two built-in strategies are provided:
//!
//! - [`RandomStrategy`] — picks a uniformly random rank from hand and a
//!   uniformly random other player as the target.
//! - [`BasicStrategy`] — picks the rank held in the greatest quantity, and
//!   targets the most recent other player who has asked for any rank.

pub mod basic;
pub mod random;

pub use basic::BasicStrategy;
pub use random::RandomStrategy;

use cardpack::prelude::BasicPile;

use crate::game::PlayerAction;
use crate::player::{AskEntry, PlayerView};

/// A pluggable bot decision strategy.
///
/// Implement this trait to define how a bot chooses its next
/// [`PlayerAction::Ask`].  Both [`RandomStrategy`] and [`BasicStrategy`]
/// implement this trait and can be composed via [`crate::bot::BotProfile`].
///
/// # Examples
///
/// ```
/// use cardpack::prelude::{BasicPile, FrenchBasicCard};
/// use gfcore::bot::strategy::{BotStrategy, RandomStrategy};
/// use gfcore::prelude::{AskEntry, Player, PlayerAction, PlayerView};
///
/// let strategy = RandomStrategy;
///
/// // Build a minimal hand and two-player view.
/// let mut hand = BasicPile::default();
/// hand.push(FrenchBasicCard::ACE_SPADES);
///
/// let mut alice = Player::new("Alice");
/// alice.receive_card(FrenchBasicCard::ACE_SPADES);
/// let mut bob = Player::new("Bob");
/// bob.receive_card(FrenchBasicCard::KING_HEARTS);
///
/// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
/// let action = strategy.decide(&hand, &players, &[]);
/// assert!(matches!(action, PlayerAction::Ask { .. }));
/// ```
pub trait BotStrategy: Send + Sync {
    /// Decides the next action for a bot.
    ///
    /// The bot receives its own hand (guaranteed non-empty when called from the
    /// game engine), the full player-view list with `hand: Some(...)` for the
    /// bot itself and `hand: None` for all others, and the full ask log.
    ///
    /// Returns a [`PlayerAction::Ask`] with a valid rank from `hand` and a
    /// valid non-self target index.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::bot::strategy::{BotStrategy, BasicStrategy};
    /// use gfcore::prelude::{AskEntry, Player, PlayerAction, PlayerView};
    ///
    /// let strategy = BasicStrategy;
    ///
    /// let mut hand = BasicPile::default();
    /// hand.push(FrenchBasicCard::ACE_SPADES);
    /// hand.push(FrenchBasicCard::ACE_HEARTS);
    ///
    /// let mut alice = Player::new("Alice");
    /// alice.receive_card(FrenchBasicCard::ACE_SPADES);
    /// alice.receive_card(FrenchBasicCard::ACE_HEARTS);
    /// let mut bob = Player::new("Bob");
    /// bob.receive_card(FrenchBasicCard::KING_HEARTS);
    ///
    /// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
    /// let action = strategy.decide(&hand, &players, &[]);
    /// assert!(matches!(action, PlayerAction::Ask { .. }));
    /// ```
    fn decide(
        &self,
        hand: &BasicPile,
        players: &[PlayerView],
        ask_log: &[AskEntry],
    ) -> PlayerAction;
}
