//! Standard Go Fish rules on a 52-card French deck.

use cardpack::prelude::{BasicPile, DeckedBase, Pip, Standard52};

/// Standard Go Fish rules.
///
/// Plays on a single shuffled 52-card French deck.  Four cards of the same
/// rank form a book.  Hand sizes and player limits follow the most widely
/// published rule set.
///
/// # Examples
///
/// ```
/// use gfcore::rules::{GoFishRules, StandardRules};
///
/// let rules = StandardRules;
/// assert_eq!(rules.name(), "Standard Go Fish");
/// assert_eq!(rules.book_size(), 4);
/// assert_eq!(rules.min_players(), 2);
/// assert_eq!(rules.max_players(), 8);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardRules;

impl GoFishRules for StandardRules {
    /// Returns the variant name.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.name(), "Standard Go Fish");
    /// ```
    fn name(&self) -> &'static str {
        "Standard Go Fish"
    }

    /// Returns a freshly shuffled 52-card draw pile.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let pile = StandardRules.deck();
    /// assert_eq!(pile.len(), 52);
    /// ```
    fn deck(&self) -> BasicPile {
        let mut pile = Standard52::basic_pile();
        pile.shuffle();
        pile
    }

    /// Returns the number of cards required to complete a book.
    ///
    /// In Standard Go Fish a book is all four cards of a single rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.book_size(), 4);
    /// ```
    fn book_size(&self) -> usize {
        4
    }

    /// Returns the number of cards dealt to each player at the start of the game.
    ///
    /// Seven cards for four or fewer players; five cards for five or more.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.initial_hand_size(2), 7);
    /// assert_eq!(StandardRules.initial_hand_size(4), 7);
    /// assert_eq!(StandardRules.initial_hand_size(5), 5);
    /// assert_eq!(StandardRules.initial_hand_size(8), 5);
    /// ```
    fn initial_hand_size(&self, player_count: usize) -> usize {
        if player_count <= 4 { 7 } else { 5 }
    }

    /// Returns the minimum number of players required.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.min_players(), 2);
    /// ```
    fn min_players(&self) -> usize {
        2
    }

    /// Returns the maximum number of players allowed.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// assert_eq!(StandardRules.max_players(), 8);
    /// ```
    fn max_players(&self) -> usize {
        8
    }

    /// Returns `true` if the player's hand contains at least one card of the given rank.
    ///
    /// A player may only ask for a rank they already hold in their hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, DeckedBase, Pip, Standard52};
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let pile = Standard52::basic_pile();
    /// // The first card is the Ace of Spades; its rank Pip is the Ace.
    /// let ace_rank: Pip = pile.v()[0].rank;
    /// assert!(StandardRules.is_valid_ask(&pile, &ace_rank));
    ///
    /// // Build an empty hand and verify that any ask is invalid.
    /// let empty = BasicPile::default();
    /// assert!(!StandardRules.is_valid_ask(&empty, &ace_rank));
    /// ```
    fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
        hand.iter().any(|card| &card.rank == rank)
    }

    /// Returns `true` if the pile constitutes a complete book.
    ///
    /// A book is exactly four cards that all share the same rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, DeckedBase, Standard52};
    /// use gfcore::rules::{GoFishRules, StandardRules};
    ///
    /// let all_aces: BasicPile = Standard52::basic_pile()
    ///     .iter()
    ///     .filter(|c| c.rank == Standard52::basic_pile().v()[0].rank)
    ///     .copied()
    ///     .collect();
    ///
    /// assert_eq!(all_aces.len(), 4);
    /// assert!(StandardRules.is_book(&all_aces));
    ///
    /// // Three cards is not a book.
    /// let partial: BasicPile = Standard52::basic_pile()
    ///     .iter()
    ///     .filter(|c| c.rank == Standard52::basic_pile().v()[0].rank)
    ///     .take(3)
    ///     .copied()
    ///     .collect();
    /// assert!(!StandardRules.is_book(&partial));
    /// ```
    fn is_book(&self, cards: &BasicPile) -> bool {
        if cards.len() != self.book_size() {
            return false;
        }
        // SAFETY: len == book_size() >= 1, so index 0 is valid
        #[allow(clippy::indexing_slicing)]
        let first_rank = cards.v()[0].rank;
        cards.v().iter().all(|card| card.rank == first_rank)
    }
}

use super::GoFishRules;

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::{BasicPile, DeckedBase, FrenchBasicCard, Standard52};

    #[test]
    fn test_standard_rules_name() {
        assert_eq!(StandardRules.name(), "Standard Go Fish");
    }

    #[test]
    fn test_standard_rules_deck_size() {
        let deck = StandardRules.deck();
        assert_eq!(deck.len(), 52);
    }

    #[test]
    fn test_standard_rules_deck_is_shuffled() {
        let rules = StandardRules;
        let deck_a = rules.deck();
        let deck_b = rules.deck();
        assert_eq!(deck_a.len(), 52);
        assert_ne!(
            deck_a.v().iter().map(|c| c.index()).collect::<Vec<_>>(),
            deck_b.v().iter().map(|c| c.index()).collect::<Vec<_>>(),
            "two independently shuffled decks should almost never be identical"
        );
    }

    #[test]
    fn test_standard_rules_book_size() {
        assert_eq!(StandardRules.book_size(), 4);
    }

    #[test]
    fn test_standard_rules_initial_hand_size_small_game() {
        assert_eq!(StandardRules.initial_hand_size(2), 7);
        assert_eq!(StandardRules.initial_hand_size(3), 7);
        assert_eq!(StandardRules.initial_hand_size(4), 7);
    }

    #[test]
    fn test_standard_rules_initial_hand_size_large_game() {
        assert_eq!(StandardRules.initial_hand_size(5), 5);
        assert_eq!(StandardRules.initial_hand_size(8), 5);
    }

    #[test]
    fn test_standard_rules_min_players() {
        assert_eq!(StandardRules.min_players(), 2);
    }

    #[test]
    fn test_standard_rules_max_players() {
        assert_eq!(StandardRules.max_players(), 8);
    }

    #[test]
    fn test_standard_rules_is_valid_ask_true() {
        let pile = Standard52::basic_pile();
        let ace_rank = pile.v()[0].rank;
        assert!(StandardRules.is_valid_ask(&pile, &ace_rank));
    }

    #[test]
    fn test_standard_rules_is_valid_ask_false_empty_hand() {
        let empty = BasicPile::default();
        let pile = Standard52::basic_pile();
        let ace_rank = pile.v()[0].rank;
        assert!(!StandardRules.is_valid_ask(&empty, &ace_rank));
    }

    #[test]
    fn test_standard_rules_is_book_true() {
        // Collect all four aces from a Standard52 deck.
        let pile = Standard52::basic_pile();
        let ace_rank = pile.v()[0].rank;
        let four_aces: BasicPile = pile
            .iter()
            .filter(|c| c.rank == ace_rank)
            .copied()
            .collect();
        assert_eq!(four_aces.len(), 4);
        assert!(StandardRules.is_book(&four_aces));
    }

    #[test]
    fn test_standard_rules_is_book_false_wrong_count() {
        let pile = Standard52::basic_pile();
        let ace_rank = pile.v()[0].rank;
        let three_aces: BasicPile = pile
            .iter()
            .filter(|c| c.rank == ace_rank)
            .take(3)
            .copied()
            .collect();
        assert!(!StandardRules.is_book(&three_aces));
    }

    #[test]
    fn test_standard_rules_is_book_false_mixed_ranks() {
        // Mix of Ace and King — not a valid book.
        let mixed = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::KING_CLUBS,
        ]);
        assert!(!StandardRules.is_book(&mixed));
    }

    #[test]
    fn test_standard_rules_is_book_false_empty() {
        let empty = BasicPile::default();
        assert!(!StandardRules.is_book(&empty));
    }

    #[test]
    fn test_standard_rules_is_valid_ask_partial_match() {
        // Hand has one king and three twos — asking for King should be valid.
        let hand = BasicPile::from(vec![
            FrenchBasicCard::KING_SPADES,
            FrenchBasicCard::DEUCE_HEARTS,
            FrenchBasicCard::DEUCE_CLUBS,
            FrenchBasicCard::DEUCE_DIAMONDS,
        ]);
        let king_rank = FrenchBasicCard::KING_SPADES.rank;
        let two_rank = FrenchBasicCard::DEUCE_HEARTS.rank;
        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;

        assert!(StandardRules.is_valid_ask(&hand, &king_rank));
        assert!(StandardRules.is_valid_ask(&hand, &two_rank));
        assert!(!StandardRules.is_valid_ask(&hand, &ace_rank));
    }

    #[test]
    fn test_standard_rules_trait_object_compiles() {
        // Verify that &dyn GoFishRules is a valid trait object.
        let rules: &dyn GoFishRules = &StandardRules;
        assert_eq!(rules.name(), "Standard Go Fish");
        assert_eq!(rules.book_size(), 4);
    }
}
