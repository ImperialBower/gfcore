//! Go Fish rule variants and configuration.
//!
//! This module defines the [`GoFishRules`] trait that every variant must implement,
//! the [`GameVariant`] enum that selects between built-in and custom variants, and
//! the three built-in rule structs: [`StandardRules`], [`HappyFamiliesRules`], and
//! [`QuartetRules`].
//!
//! # Quick example
//!
//! ```
//! use gfcore::rules::{GameVariant, GoFishRules};
//!
//! let variant = GameVariant::Standard;
//! let rules = variant.rules();
//! assert_eq!(rules.name(), "Standard Go Fish");
//! assert_eq!(rules.book_size(), 4);
//! ```

pub(crate) mod happy_families;
pub(crate) mod quartet;
pub(crate) mod standard;

pub use happy_families::{HappyFamilies, HappyFamiliesRules};
pub use quartet::{Quartet, QuartetRules};
pub use standard::StandardRules;

use cardpack::prelude::{BasicPile, Pip};

/// The contract every Go Fish variant must satisfy.
///
/// All methods must be object-safe so that `&dyn GoFishRules` is a valid trait
/// object.  No generic methods, no `Self` in return position, no `Sized` bound.
///
/// # Examples
///
/// ```
/// use gfcore::rules::{GoFishRules, StandardRules};
///
/// let rules: &dyn GoFishRules = &StandardRules;
/// assert_eq!(rules.name(), "Standard Go Fish");
/// ```
pub trait GoFishRules: Send + Sync {
    /// Returns the human-readable name of the variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.name(), "Standard Go Fish");
    /// ```
    fn name(&self) -> &'static str;

    /// Returns a freshly shuffled draw pile for this variant.
    ///
    /// # Panics
    ///
    /// May panic on `wasm32-unknown-unknown` targets if the `wasm` crate feature
    /// is not enabled, because the underlying RNG requires `getrandom/wasm_js`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let pile = StandardRules.deck();
    /// assert_eq!(pile.len(), 52);
    /// ```
    fn deck(&self) -> BasicPile;

    /// Returns the number of cards that form a complete book.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.book_size(), 4);
    /// ```
    fn book_size(&self) -> usize;

    /// Returns the number of cards dealt to each player at the start of the game.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.initial_hand_size(2), 7);
    /// assert_eq!(StandardRules.initial_hand_size(6), 5);
    /// ```
    fn initial_hand_size(&self, player_count: usize) -> usize;

    /// Returns the minimum number of players required by this variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.min_players(), 2);
    /// ```
    fn min_players(&self) -> usize;

    /// Returns the maximum number of players allowed by this variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.max_players(), 8);
    /// ```
    fn max_players(&self) -> usize;

    /// Returns `true` if the player's hand contains at least one card of the given rank.
    ///
    /// A player may only ask for a rank they already hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, DeckedBase, Standard52};
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let pile = Standard52::basic_pile();
    /// let ace_rank = pile.v()[0].rank;
    /// assert!(StandardRules.is_valid_ask(&pile, &ace_rank));
    ///
    /// let empty = BasicPile::default();
    /// assert!(!StandardRules.is_valid_ask(&empty, &ace_rank));
    /// ```
    fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool;

    /// Returns `true` if this pile of cards constitutes a complete book.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let four_aces = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]);
    /// assert!(StandardRules.is_book(&four_aces));
    ///
    /// let three_aces = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    /// ]);
    /// assert!(!StandardRules.is_book(&three_aces));
    /// ```
    fn is_book(&self, cards: &BasicPile) -> bool;
}

// ---------------------------------------------------------------------------
// GameVariant
// ---------------------------------------------------------------------------

/// Selects which Go Fish variant to play.
///
/// All three built-in variants are fully implemented.  Pass a `Custom` variant
/// to supply your own [`GoFishRules`] implementation.
///
/// | Variant         | Deck  | Families | Hand (2–4p / 5–8p) |
/// |-----------------|-------|----------|--------------------|
/// | Standard        | 52    | 13       | 7 / 5              |
/// | `HappyFamilies` | 44    | 11       | 6 / 4              |
/// | Quartet         | 32    | 8        | 8 / 6              |
///
/// # Examples
///
/// ```
/// use gfcore::rules::{GameVariant, GoFishRules};
///
/// let variant = GameVariant::Standard;
/// assert_eq!(variant.rules().name(), "Standard Go Fish");
///
/// let hf = GameVariant::HappyFamilies;
/// assert_eq!(hf.rules().name(), "Happy Families");
///
/// let qt = GameVariant::Quartet;
/// assert_eq!(qt.rules().name(), "Quartet");
/// ```
pub enum GameVariant {
    /// Standard Go Fish on a 52-card French deck.
    Standard,
    /// Happy Families variant — 44-card deck, 11 families.
    HappyFamilies,
    /// Quartet variant — 32-card deck, 8 families.
    Quartet,
    /// A fully custom variant supplied by the caller.
    Custom(Box<dyn GoFishRules + Send + Sync>),
}

impl std::fmt::Debug for GameVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "GameVariant::Standard"),
            Self::HappyFamilies => write!(f, "GameVariant::HappyFamilies"),
            Self::Quartet => write!(f, "GameVariant::Quartet"),
            Self::Custom(_) => write!(f, "GameVariant::Custom(<dyn GoFishRules>)"),
        }
    }
}

impl GameVariant {
    /// Returns a reference to the [`GoFishRules`] implementation for this
    /// variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GameVariant, GoFishRules};
    ///
    /// let rules = GameVariant::Standard.rules();
    /// assert_eq!(rules.name(), "Standard Go Fish");
    /// assert_eq!(rules.book_size(), 4);
    ///
    /// assert_eq!(GameVariant::HappyFamilies.rules().book_size(), 4);
    /// assert_eq!(GameVariant::Quartet.rules().book_size(), 4);
    /// ```
    #[must_use]
    pub fn rules(&self) -> &dyn GoFishRules {
        match self {
            Self::Standard => &StandardRules,
            Self::HappyFamilies => &HappyFamiliesRules,
            Self::Quartet => &QuartetRules,
            Self::Custom(rules) => rules.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::{BasicPile, FrenchBasicCard};

    #[test]
    fn test_game_variant_standard_rules() {
        let variant = GameVariant::Standard;
        let rules = variant.rules();
        assert_eq!(rules.name(), "Standard Go Fish");
        assert_eq!(rules.book_size(), 4);
        assert_eq!(rules.min_players(), 2);
        assert_eq!(rules.max_players(), 8);
    }

    #[test]
    fn test_game_variant_happy_families_rules() {
        let variant = GameVariant::HappyFamilies;
        let rules = variant.rules();
        assert_eq!(rules.name(), "Happy Families");
        assert_eq!(rules.book_size(), 4);
        assert_eq!(rules.min_players(), 2);
        assert_eq!(rules.max_players(), 8);
        assert_eq!(rules.initial_hand_size(4), 6);
        assert_eq!(rules.initial_hand_size(5), 4);
    }

    #[test]
    fn test_game_variant_quartet_rules() {
        let variant = GameVariant::Quartet;
        let rules = variant.rules();
        assert_eq!(rules.name(), "Quartet");
        assert_eq!(rules.book_size(), 4);
        assert_eq!(rules.min_players(), 2);
        assert_eq!(rules.max_players(), 8);
        assert_eq!(rules.initial_hand_size(4), 8);
        assert_eq!(rules.initial_hand_size(5), 6);
    }

    #[test]
    fn test_game_variant_custom() {
        struct MyRules;
        impl GoFishRules for MyRules {
            fn name(&self) -> &'static str {
                "My Custom Rules"
            }
            fn deck(&self) -> BasicPile {
                BasicPile::default()
            }
            fn book_size(&self) -> usize {
                3
            }
            fn initial_hand_size(&self, _player_count: usize) -> usize {
                5
            }
            fn min_players(&self) -> usize {
                2
            }
            fn max_players(&self) -> usize {
                6
            }
            fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
                hand.iter().any(|c| &c.rank == rank)
            }
            fn is_book(&self, cards: &BasicPile) -> bool {
                cards.len() == 3
            }
        }

        let variant = GameVariant::Custom(Box::new(MyRules));
        let rules = variant.rules();
        assert_eq!(rules.name(), "My Custom Rules");
        assert_eq!(rules.book_size(), 3);
    }

    #[test]
    fn test_go_fish_rules_trait_object_is_valid() {
        // Compile-time proof: &dyn GoFishRules must be a valid trait object.
        let rules: &dyn GoFishRules = &StandardRules;
        assert_eq!(rules.name(), "Standard Go Fish");
    }

    #[test]
    fn test_standard_rules_is_book_four_aces() {
        let four_aces = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        assert!(StandardRules.is_book(&four_aces));
    }

    #[test]
    fn test_game_variant_debug_standard() {
        assert_eq!(
            format!("{:?}", GameVariant::Standard),
            "GameVariant::Standard"
        );
    }

    #[test]
    fn test_game_variant_debug_happy_families() {
        assert_eq!(
            format!("{:?}", GameVariant::HappyFamilies),
            "GameVariant::HappyFamilies"
        );
    }

    #[test]
    fn test_game_variant_debug_quartet() {
        assert_eq!(
            format!("{:?}", GameVariant::Quartet),
            "GameVariant::Quartet"
        );
    }
}
