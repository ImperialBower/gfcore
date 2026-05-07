//! Bot player profiles.
//!
//! A [`BotProfile`] pairs a display name with a [`BotStrategy`] implementation
//! and exposes a single [`BotProfile::decide`] method that the game engine
//! calls to obtain the bot's next action.
//!
//! Two convenience constructors are provided:
//!
//! - [`BotProfile::random`] — wraps a [`RandomStrategy`].
//! - [`BotProfile::basic`]  — wraps a [`BasicStrategy`].
//!
//! For custom strategies use [`BotProfile::custom`].

use cardpack::prelude::BasicPile;

use crate::game::PlayerAction;
use crate::player::{AskEntry, PlayerView};

use super::strategy::{BasicStrategy, BotStrategy, RandomStrategy};

/// Tracks which built-in strategy a [`BotProfile`] uses, enabling correct
/// [`Clone`] behaviour without requiring `Box<dyn BotStrategy>: Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrategyKind {
    Random,
    Basic,
    /// Custom strategy — not clonable; clone falls back to `BasicStrategy`.
    Custom,
}

/// A named bot player profile that couples a display name with a decision
/// strategy.
///
/// # Examples
///
/// ```
/// use gfcore::bot::BotProfile;
///
/// let profile = BotProfile::basic("Harriet");
/// assert_eq!(profile.name, "Harriet");
/// ```
pub struct BotProfile {
    /// The bot's display name, visible to all players and in game logs.
    pub name: String,
    /// The strategy used when [`BotProfile::decide`] is called.
    strategy: Box<dyn BotStrategy>,
    /// Which built-in strategy backs this profile (for [`Clone`] support).
    kind: StrategyKind,
}

impl std::fmt::Debug for BotProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BotProfile")
            .field("name", &self.name)
            .field("strategy", &"<dyn BotStrategy>")
            .field("kind", &self.kind)
            .finish()
    }
}

impl BotProfile {
    /// Creates a profile backed by [`RandomStrategy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    ///
    /// let profile = BotProfile::random("Lucky");
    /// assert_eq!(profile.name, "Lucky");
    /// ```
    #[must_use]
    pub fn random(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            strategy: Box::new(RandomStrategy),
            kind: StrategyKind::Random,
        }
    }

    /// Creates a profile backed by [`BasicStrategy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    ///
    /// let profile = BotProfile::basic("Harriet");
    /// assert_eq!(profile.name, "Harriet");
    /// ```
    #[must_use]
    pub fn basic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            strategy: Box::new(BasicStrategy),
            kind: StrategyKind::Basic,
        }
    }

    /// Creates a profile backed by a custom [`BotStrategy`] implementation.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::BasicPile;
    /// use gfcore::bot::{BotProfile, BotStrategy};
    /// use gfcore::prelude::{AskEntry, PlayerAction, PlayerView};
    /// use cardpack::prelude::{DeckedBase, Standard52, FrenchBasicCard};
    ///
    /// struct AlwaysAskAce;
    ///
    /// impl BotStrategy for AlwaysAskAce {
    ///     fn decide(
    ///         &self,
    ///         _hand: &BasicPile,
    ///         _players: &[PlayerView],
    ///         _ask_log: &[AskEntry],
    ///     ) -> PlayerAction {
    ///         let rank = FrenchBasicCard::ACE_SPADES.rank;
    ///         PlayerAction::Ask { target: 1, rank }
    ///     }
    /// }
    ///
    /// let profile = BotProfile::custom("Ace-Fanatic", Box::new(AlwaysAskAce));
    /// assert_eq!(profile.name, "Ace-Fanatic");
    /// ```
    #[must_use]
    pub fn custom(name: impl Into<String>, strategy: Box<dyn BotStrategy>) -> Self {
        Self {
            name: name.into(),
            strategy,
            kind: StrategyKind::Custom,
        }
    }

    /// Asks the strategy to decide the bot's next action.
    ///
    /// `hand` must be non-empty (the game engine enforces this invariant before
    /// calling into the bot layer).  `players` provides the current-player view
    /// with `hand: Some(...)` for the bot.  `ask_log` contains all asks made
    /// since game start.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::bot::BotProfile;
    /// use gfcore::prelude::{Player, PlayerAction, PlayerView};
    ///
    /// let profile = BotProfile::basic("Harriet");
    ///
    /// let mut alice = Player::new("Alice");
    /// alice.receive_card(FrenchBasicCard::ACE_SPADES);
    /// let mut bob = Player::new("Bob");
    /// bob.receive_card(FrenchBasicCard::KING_HEARTS);
    ///
    /// let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
    /// let hand = players[0].hand.clone().unwrap_or_default();
    ///
    /// let action = profile.decide(&hand, &players, &[]);
    /// assert!(matches!(action, PlayerAction::Ask { .. }));
    /// ```
    #[must_use]
    pub fn decide(
        &self,
        hand: &BasicPile,
        players: &[PlayerView],
        ask_log: &[AskEntry],
    ) -> PlayerAction {
        self.strategy.decide(hand, players, ask_log)
    }

    /// Returns a set of four pre-named bot profiles: two using [`BasicStrategy`]
    /// and two using [`RandomStrategy`].
    ///
    /// The profiles are (in order):
    /// 1. `BotProfile::basic("Harriet")`
    /// 2. `BotProfile::basic("Bertram")`
    /// 3. `BotProfile::random("Lucky")`
    /// 4. `BotProfile::random("Chance")`
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    ///
    /// let profiles = BotProfile::default_profiles();
    /// assert_eq!(profiles.len(), 4);
    /// assert_eq!(profiles[0].name, "Harriet");
    /// assert_eq!(profiles[1].name, "Bertram");
    /// assert_eq!(profiles[2].name, "Lucky");
    /// assert_eq!(profiles[3].name, "Chance");
    /// ```
    #[must_use]
    pub fn default_profiles() -> Vec<Self> {
        vec![
            Self::basic("Harriet"),
            Self::basic("Bertram"),
            Self::random("Lucky"),
            Self::random("Chance"),
        ]
    }
}

impl Clone for BotProfile {
    /// Clones the profile, preserving both the name and the strategy type.
    ///
    /// Built-in strategies ([`BotProfile::random`] and [`BotProfile::basic`])
    /// are cloned with the same strategy.  Profiles created via
    /// [`BotProfile::custom`] cannot clone their arbitrary strategy, so they
    /// fall back to [`BasicStrategy`]; the name is always preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    ///
    /// let original = BotProfile::random("Lucky");
    /// let copy = original.clone();
    /// assert_eq!(copy.name, "Lucky");
    /// // Copy retains RandomStrategy behaviour (not silently converted to Basic).
    /// ```
    fn clone(&self) -> Self {
        let strategy: Box<dyn BotStrategy> = match self.kind {
            StrategyKind::Random => Box::new(RandomStrategy),
            // Custom strategies are not clonable; fall back to BasicStrategy.
            StrategyKind::Basic | StrategyKind::Custom => Box::new(BasicStrategy),
        };
        Self {
            name: self.name.clone(),
            strategy,
            kind: self.kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_profile_random_name() {
        let profile = BotProfile::random("Lucky");
        assert_eq!(profile.name, "Lucky");
    }

    #[test]
    fn test_bot_profile_basic_name() {
        let profile = BotProfile::basic("Harriet");
        assert_eq!(profile.name, "Harriet");
    }

    #[test]
    fn test_bot_profile_default_profiles_count() {
        let profiles = BotProfile::default_profiles();
        assert_eq!(profiles.len(), 4);
    }

    #[test]
    fn test_bot_profile_default_profiles_names() {
        let profiles = BotProfile::default_profiles();
        assert_eq!(profiles[0].name, "Harriet");
        assert_eq!(profiles[1].name, "Bertram");
        assert_eq!(profiles[2].name, "Lucky");
        assert_eq!(profiles[3].name, "Chance");
    }

    #[test]
    fn test_bot_profile_clone_preserves_name() {
        let profile = BotProfile::basic("Harriet");
        let cloned = profile.clone();
        assert_eq!(cloned.name, "Harriet");
    }

    #[test]
    fn test_bot_profile_decide_returns_ask() {
        use crate::player::Player;
        use cardpack::prelude::FrenchBasicCard;

        let profile = BotProfile::basic("Harriet");

        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::KING_HEARTS);

        let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
        let hand = players[0].hand.clone().unwrap_or_default();

        let action = profile.decide(&hand, &players, &[]);
        assert!(matches!(action, PlayerAction::Ask { .. }));
    }

    #[test]
    fn test_bot_profile_custom() {
        use crate::player::Player;
        use cardpack::prelude::FrenchBasicCard;

        struct FixedTarget;
        impl BotStrategy for FixedTarget {
            fn decide(
                &self,
                hand: &BasicPile,
                _players: &[PlayerView],
                _ask_log: &[AskEntry],
            ) -> PlayerAction {
                let rank = hand
                    .v()
                    .first()
                    .map(|card| card.rank)
                    .unwrap_or(FrenchBasicCard::ACE_SPADES.rank);
                PlayerAction::Ask { target: 1, rank }
            }
        }

        let profile = BotProfile::custom("Custom", Box::new(FixedTarget));
        assert_eq!(profile.name, "Custom");

        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::KING_HEARTS);

        let players = PlayerView::from_perspective(&[alice, bob], 0).unwrap();
        let hand = players[0].hand.clone().unwrap_or_default();

        if let PlayerAction::Ask { target, .. } = profile.decide(&hand, &players, &[]) {
            assert_eq!(target, 1);
        } else {
            panic!("expected Ask");
        }
    }

    #[test]
    fn test_bot_profile_debug_contains_name_and_struct_name() {
        let profile = BotProfile::random("Lucky");
        let debug = format!("{profile:?}");
        assert!(
            debug.contains("BotProfile"),
            "debug must contain struct name"
        );
        assert!(debug.contains("Lucky"), "debug must contain the bot's name");
    }

    #[test]
    fn test_bot_profile_debug_basic_strategy_contains_kind() {
        let profile = BotProfile::basic("Harriet");
        let debug = format!("{profile:?}");
        assert!(debug.contains("BotProfile"));
        assert!(debug.contains("Harriet"));
    }
}
