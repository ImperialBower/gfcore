//! Quartet Go Fish variant — 32-card deck (8 ranks × 4 suits).
//!
//! This variant uses the high-pip subset of the standard French deck: ranks
//! Ace through Seven (8 ranks) across all four suits, giving 32 cards and
//! 8 families.  Four cards of the same rank complete a family (book).

use std::collections::HashMap;

use cardpack::prelude::{
    BasicCard, BasicPile, Color, DeckedBase, FrenchBasicCard, Pip, Standard52,
};

use super::GoFishRules;

// ---------------------------------------------------------------------------
// Quartet (DeckedBase implementation)
// ---------------------------------------------------------------------------

/// A 32-card deck used by the Quartet variant.
///
/// Consists of ranks Ace through Seven across Spades, Hearts, Diamonds, and
/// Clubs — 8 ranks × 4 suits = 32 cards.
///
/// # Examples
///
/// ```
/// use cardpack::prelude::DeckedBase;
/// use gfcore::rules::Quartet;
///
/// assert_eq!(Quartet::basic_pile().len(), 32);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Quartet;

impl Quartet {
    /// Total number of cards in this deck.
    pub const DECK_SIZE: usize = 32;

    /// The fixed 32-card deck: ranks A–7 across all four suits.
    pub const DECK: [BasicCard; Self::DECK_SIZE] = [
        // Spades: A K Q J T 9 8 7
        FrenchBasicCard::ACE_SPADES,
        FrenchBasicCard::KING_SPADES,
        FrenchBasicCard::QUEEN_SPADES,
        FrenchBasicCard::JACK_SPADES,
        FrenchBasicCard::TEN_SPADES,
        FrenchBasicCard::NINE_SPADES,
        FrenchBasicCard::EIGHT_SPADES,
        FrenchBasicCard::SEVEN_SPADES,
        // Hearts: A K Q J T 9 8 7
        FrenchBasicCard::ACE_HEARTS,
        FrenchBasicCard::KING_HEARTS,
        FrenchBasicCard::QUEEN_HEARTS,
        FrenchBasicCard::JACK_HEARTS,
        FrenchBasicCard::TEN_HEARTS,
        FrenchBasicCard::NINE_HEARTS,
        FrenchBasicCard::EIGHT_HEARTS,
        FrenchBasicCard::SEVEN_HEARTS,
        // Diamonds: A K Q J T 9 8 7
        FrenchBasicCard::ACE_DIAMONDS,
        FrenchBasicCard::KING_DIAMONDS,
        FrenchBasicCard::QUEEN_DIAMONDS,
        FrenchBasicCard::JACK_DIAMONDS,
        FrenchBasicCard::TEN_DIAMONDS,
        FrenchBasicCard::NINE_DIAMONDS,
        FrenchBasicCard::EIGHT_DIAMONDS,
        FrenchBasicCard::SEVEN_DIAMONDS,
        // Clubs: A K Q J T 9 8 7
        FrenchBasicCard::ACE_CLUBS,
        FrenchBasicCard::KING_CLUBS,
        FrenchBasicCard::QUEEN_CLUBS,
        FrenchBasicCard::JACK_CLUBS,
        FrenchBasicCard::TEN_CLUBS,
        FrenchBasicCard::NINE_CLUBS,
        FrenchBasicCard::EIGHT_CLUBS,
        FrenchBasicCard::SEVEN_CLUBS,
    ];
}

impl DeckedBase for Quartet {
    fn colors() -> HashMap<Pip, Color> {
        Standard52::colors()
    }

    fn base_vec() -> Vec<BasicCard> {
        Self::DECK.to_vec()
    }

    fn deck_name() -> String {
        "Quartet".to_string()
    }

    fn fluent_deck_key() -> String {
        cardpack::prelude::FLUENT_KEY_BASE_NAME_FRENCH.to_string()
    }
}

// ---------------------------------------------------------------------------
// QuartetRules (GoFishRules implementation)
// ---------------------------------------------------------------------------

/// Go Fish rules for the Quartet variant.
///
/// Uses a 32-card deck (8 ranks × 4 suits).  Four cards of the same rank
/// constitute a book.  Initial hand sizes are larger than Standard Go Fish
/// because the smaller deck means quicker exhaustion.
///
/// | Players | Initial hand |
/// |---------|-------------|
/// | 2–4     | 8 cards     |
/// | 5–8     | 6 cards     |
///
/// # Examples
///
/// ```
/// use gfcore::rules::{GoFishRules, QuartetRules};
///
/// let rules = QuartetRules;
/// assert_eq!(rules.name(), "Quartet");
/// assert_eq!(rules.book_size(), 4);
/// assert_eq!(rules.min_players(), 2);
/// assert_eq!(rules.max_players(), 8);
/// assert_eq!(rules.initial_hand_size(2), 8);
/// assert_eq!(rules.initial_hand_size(5), 6);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct QuartetRules;

impl GoFishRules for QuartetRules {
    /// Returns the variant name.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// assert_eq!(QuartetRules.name(), "Quartet");
    /// ```
    fn name(&self) -> &'static str {
        "Quartet"
    }

    /// Returns a freshly shuffled 32-card draw pile.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// let pile = QuartetRules.deck();
    /// assert_eq!(pile.len(), 32);
    /// ```
    fn deck(&self) -> BasicPile {
        let mut pile = Quartet::basic_pile();
        pile.shuffle();
        pile
    }

    /// Returns 4: four matching cards form a book.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// assert_eq!(QuartetRules.book_size(), 4);
    /// ```
    fn book_size(&self) -> usize {
        4
    }

    /// Returns the initial hand size: 8 cards for 2–4 players, 6 for 5–8.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// assert_eq!(QuartetRules.initial_hand_size(4), 8);
    /// assert_eq!(QuartetRules.initial_hand_size(5), 6);
    /// ```
    fn initial_hand_size(&self, player_count: usize) -> usize {
        if player_count <= 4 { 8 } else { 6 }
    }

    /// Returns 2: Quartet requires at least 2 players.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// assert_eq!(QuartetRules.min_players(), 2);
    /// ```
    fn min_players(&self) -> usize {
        2
    }

    /// Returns 8: Quartet supports up to 8 players.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// assert_eq!(QuartetRules.max_players(), 8);
    /// ```
    fn max_players(&self) -> usize {
        8
    }

    /// Returns `true` if `hand` contains at least one card of the requested rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// let mut hand = BasicPile::default();
    /// hand.push(FrenchBasicCard::ACE_SPADES);
    /// let rank = FrenchBasicCard::ACE_SPADES.rank;
    /// assert!(QuartetRules.is_valid_ask(&hand, &rank));
    ///
    /// let empty = BasicPile::default();
    /// assert!(!QuartetRules.is_valid_ask(&empty, &rank));
    /// ```
    fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
        hand.iter().any(|card| &card.rank == rank)
    }

    /// Returns `true` if `cards` is a complete book: exactly 4 cards of the same rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::rules::{GoFishRules, QuartetRules};
    ///
    /// let book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]);
    /// assert!(QuartetRules.is_book(&book));
    ///
    /// let not_book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    /// ]);
    /// assert!(!QuartetRules.is_book(&not_book));
    /// ```
    fn is_book(&self, cards: &BasicPile) -> bool {
        if cards.len() != self.book_size() {
            return false;
        }
        #[allow(clippy::indexing_slicing)]
        let first_rank = cards.v()[0].rank;
        cards.iter().all(|card| card.rank == first_rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deck_has_32_cards() {
        assert_eq!(Quartet::basic_pile().len(), 32);
    }

    #[test]
    fn test_deck_size_constant() {
        assert_eq!(Quartet::DECK_SIZE, 32);
    }

    #[test]
    fn test_rules_name() {
        assert_eq!(QuartetRules.name(), "Quartet");
    }

    #[test]
    fn test_deck_returns_32_cards() {
        assert_eq!(QuartetRules.deck().len(), 32);
    }

    #[test]
    fn test_book_size() {
        assert_eq!(QuartetRules.book_size(), 4);
    }

    #[test]
    fn test_initial_hand_size_small_game() {
        assert_eq!(QuartetRules.initial_hand_size(2), 8);
        assert_eq!(QuartetRules.initial_hand_size(4), 8);
    }

    #[test]
    fn test_initial_hand_size_large_game() {
        assert_eq!(QuartetRules.initial_hand_size(5), 6);
        assert_eq!(QuartetRules.initial_hand_size(8), 6);
    }

    #[test]
    fn test_player_limits() {
        assert_eq!(QuartetRules.min_players(), 2);
        assert_eq!(QuartetRules.max_players(), 8);
    }

    #[test]
    fn test_is_book_four_same_rank() {
        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        assert!(QuartetRules.is_book(&book));
    }

    #[test]
    fn test_is_book_rejects_three_cards() {
        let three = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
        ]);
        assert!(!QuartetRules.is_book(&three));
    }

    #[test]
    fn test_is_book_rejects_mixed_ranks() {
        let mixed = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::KING_CLUBS,
        ]);
        assert!(!QuartetRules.is_book(&mixed));
    }

    #[test]
    fn test_is_valid_ask_with_rank_in_hand() {
        let mut hand = BasicPile::default();
        hand.push(FrenchBasicCard::SEVEN_SPADES);
        let rank = FrenchBasicCard::SEVEN_SPADES.rank;
        assert!(QuartetRules.is_valid_ask(&hand, &rank));
    }

    #[test]
    fn test_is_valid_ask_empty_hand() {
        let empty = BasicPile::default();
        let rank = FrenchBasicCard::SEVEN_SPADES.rank;
        assert!(!QuartetRules.is_valid_ask(&empty, &rank));
    }

    #[test]
    fn test_deck_contains_no_low_pips() {
        let pile = Quartet::basic_pile();
        // Six through Two should not appear in the Quartet deck.
        let has_six = pile
            .iter()
            .any(|c| c.rank == FrenchBasicCard::SIX_SPADES.rank);
        assert!(!has_six, "Quartet deck must not contain sixes");
    }

    #[test]
    fn test_deck_contains_sevens() {
        let pile = Quartet::basic_pile();
        let seven_count = pile
            .iter()
            .filter(|c| c.rank == FrenchBasicCard::SEVEN_SPADES.rank)
            .count();
        assert_eq!(seven_count, 4, "Quartet deck must contain four sevens");
    }
}
