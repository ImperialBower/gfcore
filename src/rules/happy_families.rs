//! Happy Families Go Fish variant — 44-card deck (11 ranks × 4 suits).
//!
//! This variant uses a subset of the standard French deck: ranks Ace through
//! Four (11 ranks) across all four suits, giving 44 cards and 11 families.
//! Four cards of the same rank complete a family (book).

use std::collections::HashMap;

use cardpack::prelude::{
    BasicCard, BasicPile, Color, DeckedBase, FrenchBasicCard, Pip, Standard52,
};

use super::GoFishRules;

// ---------------------------------------------------------------------------
// HappyFamilies (DeckedBase implementation)
// ---------------------------------------------------------------------------

/// A 44-card deck used by the Happy Families variant.
///
/// Consists of ranks Ace through Four across Spades, Hearts, Diamonds, and
/// Clubs — 11 ranks × 4 suits = 44 cards.
///
/// # Examples
///
/// ```
/// use cardpack::prelude::DeckedBase;
/// use gfcore::rules::HappyFamiliesRules;
///
/// // The deck has 44 cards.
/// use gfcore::rules::HappyFamilies;
/// assert_eq!(HappyFamilies::basic_pile().len(), 44);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HappyFamilies;

impl HappyFamilies {
    /// Total number of cards in this deck.
    pub const DECK_SIZE: usize = 44;

    /// The fixed 44-card deck: ranks A–4 across all four suits.
    pub const DECK: [BasicCard; Self::DECK_SIZE] = [
        // Spades: A K Q J T 9 8 7 6 5 4
        FrenchBasicCard::ACE_SPADES,
        FrenchBasicCard::KING_SPADES,
        FrenchBasicCard::QUEEN_SPADES,
        FrenchBasicCard::JACK_SPADES,
        FrenchBasicCard::TEN_SPADES,
        FrenchBasicCard::NINE_SPADES,
        FrenchBasicCard::EIGHT_SPADES,
        FrenchBasicCard::SEVEN_SPADES,
        FrenchBasicCard::SIX_SPADES,
        FrenchBasicCard::FIVE_SPADES,
        FrenchBasicCard::FOUR_SPADES,
        // Hearts: A K Q J T 9 8 7 6 5 4
        FrenchBasicCard::ACE_HEARTS,
        FrenchBasicCard::KING_HEARTS,
        FrenchBasicCard::QUEEN_HEARTS,
        FrenchBasicCard::JACK_HEARTS,
        FrenchBasicCard::TEN_HEARTS,
        FrenchBasicCard::NINE_HEARTS,
        FrenchBasicCard::EIGHT_HEARTS,
        FrenchBasicCard::SEVEN_HEARTS,
        FrenchBasicCard::SIX_HEARTS,
        FrenchBasicCard::FIVE_HEARTS,
        FrenchBasicCard::FOUR_HEARTS,
        // Diamonds: A K Q J T 9 8 7 6 5 4
        FrenchBasicCard::ACE_DIAMONDS,
        FrenchBasicCard::KING_DIAMONDS,
        FrenchBasicCard::QUEEN_DIAMONDS,
        FrenchBasicCard::JACK_DIAMONDS,
        FrenchBasicCard::TEN_DIAMONDS,
        FrenchBasicCard::NINE_DIAMONDS,
        FrenchBasicCard::EIGHT_DIAMONDS,
        FrenchBasicCard::SEVEN_DIAMONDS,
        FrenchBasicCard::SIX_DIAMONDS,
        FrenchBasicCard::FIVE_DIAMONDS,
        FrenchBasicCard::FOUR_DIAMONDS,
        // Clubs: A K Q J T 9 8 7 6 5 4
        FrenchBasicCard::ACE_CLUBS,
        FrenchBasicCard::KING_CLUBS,
        FrenchBasicCard::QUEEN_CLUBS,
        FrenchBasicCard::JACK_CLUBS,
        FrenchBasicCard::TEN_CLUBS,
        FrenchBasicCard::NINE_CLUBS,
        FrenchBasicCard::EIGHT_CLUBS,
        FrenchBasicCard::SEVEN_CLUBS,
        FrenchBasicCard::SIX_CLUBS,
        FrenchBasicCard::FIVE_CLUBS,
        FrenchBasicCard::FOUR_CLUBS,
    ];
}

impl DeckedBase for HappyFamilies {
    fn colors() -> HashMap<Pip, Color> {
        Standard52::colors()
    }

    fn base_vec() -> Vec<BasicCard> {
        Self::DECK.to_vec()
    }

    fn deck_name() -> String {
        "Happy Families".to_string()
    }

    fn fluent_deck_key() -> String {
        cardpack::prelude::FLUENT_KEY_BASE_NAME_FRENCH.to_string()
    }
}

// ---------------------------------------------------------------------------
// HappyFamiliesRules (GoFishRules implementation)
// ---------------------------------------------------------------------------

/// Go Fish rules for the Happy Families variant.
///
/// Uses a 44-card deck (11 ranks × 4 suits).  Four cards of the same rank
/// constitute a book.  Hand sizes are slightly smaller than Standard Go Fish
/// to account for the reduced deck.
///
/// | Players | Initial hand |
/// |---------|-------------|
/// | 2–4     | 6 cards     |
/// | 5–8     | 4 cards     |
///
/// # Examples
///
/// ```
/// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
///
/// let rules = HappyFamiliesRules;
/// assert_eq!(rules.name(), "Happy Families");
/// assert_eq!(rules.book_size(), 4);
/// assert_eq!(rules.min_players(), 2);
/// assert_eq!(rules.max_players(), 8);
/// assert_eq!(rules.initial_hand_size(2), 6);
/// assert_eq!(rules.initial_hand_size(5), 4);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct HappyFamiliesRules;

impl GoFishRules for HappyFamiliesRules {
    /// Returns the variant name.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// assert_eq!(HappyFamiliesRules.name(), "Happy Families");
    /// ```
    fn name(&self) -> &'static str {
        "Happy Families"
    }

    /// Returns a freshly shuffled 44-card draw pile.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// let pile = HappyFamiliesRules.deck();
    /// assert_eq!(pile.len(), 44);
    /// ```
    fn deck(&self) -> BasicPile {
        let mut pile = HappyFamilies::basic_pile();
        pile.shuffle();
        pile
    }

    /// Returns 4: four matching cards form a book.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// assert_eq!(HappyFamiliesRules.book_size(), 4);
    /// ```
    fn book_size(&self) -> usize {
        4
    }

    /// Returns the initial hand size: 6 cards for 2–4 players, 4 for 5–8.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// assert_eq!(HappyFamiliesRules.initial_hand_size(4), 6);
    /// assert_eq!(HappyFamiliesRules.initial_hand_size(5), 4);
    /// ```
    fn initial_hand_size(&self, player_count: usize) -> usize {
        if player_count <= 4 { 6 } else { 4 }
    }

    /// Returns 2: Happy Families requires at least 2 players.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// assert_eq!(HappyFamiliesRules.min_players(), 2);
    /// ```
    fn min_players(&self) -> usize {
        2
    }

    /// Returns 8: Happy Families supports up to 8 players.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// assert_eq!(HappyFamiliesRules.max_players(), 8);
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
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// let mut hand = BasicPile::default();
    /// hand.push(FrenchBasicCard::ACE_SPADES);
    /// let rank = FrenchBasicCard::ACE_SPADES.rank;
    /// assert!(HappyFamiliesRules.is_valid_ask(&hand, &rank));
    ///
    /// let empty = BasicPile::default();
    /// assert!(!HappyFamiliesRules.is_valid_ask(&empty, &rank));
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
    /// use gfcore::rules::{GoFishRules, HappyFamiliesRules};
    ///
    /// let book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]);
    /// assert!(HappyFamiliesRules.is_book(&book));
    ///
    /// let not_book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    /// ]);
    /// assert!(!HappyFamiliesRules.is_book(&not_book));
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
    fn test_deck_has_44_cards() {
        assert_eq!(HappyFamilies::basic_pile().len(), 44);
    }

    #[test]
    fn test_deck_size_constant() {
        assert_eq!(HappyFamilies::DECK_SIZE, 44);
    }

    #[test]
    fn test_rules_name() {
        assert_eq!(HappyFamiliesRules.name(), "Happy Families");
    }

    #[test]
    fn test_deck_returns_44_cards() {
        assert_eq!(HappyFamiliesRules.deck().len(), 44);
    }

    #[test]
    fn test_book_size() {
        assert_eq!(HappyFamiliesRules.book_size(), 4);
    }

    #[test]
    fn test_initial_hand_size_small_game() {
        assert_eq!(HappyFamiliesRules.initial_hand_size(2), 6);
        assert_eq!(HappyFamiliesRules.initial_hand_size(4), 6);
    }

    #[test]
    fn test_initial_hand_size_large_game() {
        assert_eq!(HappyFamiliesRules.initial_hand_size(5), 4);
        assert_eq!(HappyFamiliesRules.initial_hand_size(8), 4);
    }

    #[test]
    fn test_player_limits() {
        assert_eq!(HappyFamiliesRules.min_players(), 2);
        assert_eq!(HappyFamiliesRules.max_players(), 8);
    }

    #[test]
    fn test_is_book_four_same_rank() {
        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        assert!(HappyFamiliesRules.is_book(&book));
    }

    #[test]
    fn test_is_book_rejects_three_cards() {
        let three = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
        ]);
        assert!(!HappyFamiliesRules.is_book(&three));
    }

    #[test]
    fn test_is_book_rejects_mixed_ranks() {
        let mixed = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::KING_CLUBS,
        ]);
        assert!(!HappyFamiliesRules.is_book(&mixed));
    }

    #[test]
    fn test_is_valid_ask_with_rank_in_hand() {
        let mut hand = BasicPile::default();
        hand.push(FrenchBasicCard::ACE_SPADES);
        let rank = FrenchBasicCard::ACE_SPADES.rank;
        assert!(HappyFamiliesRules.is_valid_ask(&hand, &rank));
    }

    #[test]
    fn test_is_valid_ask_empty_hand() {
        let empty = BasicPile::default();
        let rank = FrenchBasicCard::ACE_SPADES.rank;
        assert!(!HappyFamiliesRules.is_valid_ask(&empty, &rank));
    }

    #[test]
    fn test_deck_name_is_happy_families() {
        assert_eq!(HappyFamilies::deck_name(), "Happy Families");
    }

    #[test]
    fn test_fluent_deck_key_is_non_empty() {
        assert_eq!(
            HappyFamilies::fluent_deck_key(),
            cardpack::prelude::FLUENT_KEY_BASE_NAME_FRENCH
        );
    }

    #[test]
    fn test_colors_is_non_empty() {
        assert!(!HappyFamilies::colors().is_empty());
    }
}
