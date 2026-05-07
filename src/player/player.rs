//! The [`Player`] struct and its hand-management methods.

use cardpack::prelude::{BasicCard, BasicPile, Pip};

use crate::bot::BotProfile;

/// A Go Fish player, holding a hand of cards and a collection of completed books.
///
/// The `hand` and `books` fields are private; interact with them exclusively
/// through the provided methods.  Set `profile` to `Some(BotProfile)` for a
/// bot player and `None` for a human.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::Player;
///
/// let mut player = Player::new("Alice");
/// assert_eq!(player.name, "Alice");
/// assert!(player.hand_is_empty());
/// assert!(!player.is_bot());
/// ```
#[derive(Debug, Clone)]
pub struct Player {
    /// The player's display name.
    pub name: String,
    /// Private hand — access through methods.
    hand: BasicPile,
    /// Completed books, each stored as a [`BasicPile`] of `book_size` cards.
    books: Vec<BasicPile>,
    /// `None` for a human player; `Some` for a bot.
    ///
    /// TODO(Step 6): replace with the full `BotProfile` implementation.
    pub profile: Option<BotProfile>,
}

impl Player {
    /// Creates a new human player with an empty hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::Player;
    ///
    /// let player = Player::new("Bob");
    /// assert_eq!(player.name, "Bob");
    /// assert!(player.hand_is_empty());
    /// assert!(!player.is_bot());
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hand: BasicPile::default(),
            books: Vec::new(),
            profile: None,
        }
    }

    /// Creates a new bot player with the given [`BotProfile`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    /// use gfcore::prelude::Player;
    ///
    /// let profile = BotProfile::basic("DeepFish");
    /// let player = Player::new_bot("Bot-1", profile);
    /// assert!(player.is_bot());
    /// ```
    #[must_use]
    pub fn new_bot(name: impl Into<String>, profile: BotProfile) -> Self {
        Self {
            name: name.into(),
            hand: BasicPile::default(),
            books: Vec::new(),
            profile: Some(profile),
        }
    }

    /// Adds a card to this player's hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Carol");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// assert_eq!(player.hand_size(), 1);
    /// ```
    pub fn receive_card(&mut self, card: BasicCard) {
        self.hand.push(card);
    }

    /// Removes and returns all cards of the given rank from this player's hand.
    ///
    /// Returns an empty [`BasicPile`] if the hand holds no card of that rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Dave");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// player.receive_card(FrenchBasicCard::ACE_HEARTS);
    /// player.receive_card(FrenchBasicCard::KING_SPADES);
    ///
    /// let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
    /// let given = player.give_cards_of_rank(&ace_rank);
    /// assert_eq!(given.len(), 2);
    /// assert_eq!(player.hand_size(), 1); // only King remains
    /// ```
    pub fn give_cards_of_rank(&mut self, rank: &Pip) -> BasicPile {
        let hand = std::mem::take(&mut self.hand);
        let (matching, remaining): (Vec<_>, Vec<_>) =
            hand.into_iter().partition(|card| &card.rank == rank);
        self.hand = remaining.into_iter().collect();
        matching.into_iter().collect()
    }

    /// Returns `true` if the hand contains at least one card of the given rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Eve");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    ///
    /// let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
    /// let king_rank = FrenchBasicCard::KING_SPADES.rank;
    /// assert!(player.has_rank(&ace_rank));
    /// assert!(!player.has_rank(&king_rank));
    /// ```
    #[must_use]
    pub fn has_rank(&self, rank: &Pip) -> bool {
        self.hand.iter().any(|card| &card.rank == rank)
    }

    /// Returns a copy of all cards of the given rank without removing them.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Frank");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// player.receive_card(FrenchBasicCard::ACE_HEARTS);
    ///
    /// let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
    /// let peek = player.cards_of_rank(&ace_rank);
    /// assert_eq!(peek.len(), 2);
    /// assert_eq!(player.hand_size(), 2); // hand unchanged
    /// ```
    #[must_use]
    pub fn cards_of_rank(&self, rank: &Pip) -> BasicPile {
        self.hand
            .iter()
            .filter(|card| &card.rank == rank)
            .copied()
            .collect()
    }

    /// Records a completed book, moving it from active play into the player's
    /// score pile.
    ///
    /// The caller is responsible for ensuring that `book` is a valid book
    /// according to the active [`crate::rules::GoFishRules`].
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Grace");
    /// let book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]);
    /// player.add_book(book);
    /// assert_eq!(player.book_count(), 1);
    /// ```
    pub fn add_book(&mut self, book: BasicPile) {
        self.books.push(book);
    }

    /// Returns the number of completed books this player holds.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Henry");
    /// assert_eq!(player.book_count(), 0);
    /// player.add_book(BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]));
    /// assert_eq!(player.book_count(), 1);
    /// ```
    #[must_use]
    pub fn book_count(&self) -> usize {
        self.books.len()
    }

    /// Returns the rank index string of each completed book, in collection order.
    ///
    /// Each entry is the `index` of the first card in the book pile
    /// (e.g., `"A"` for Aces, `"7"` for Sevens).
    /// The returned `Vec` is always the same length as `book_count()`.
    /// An empty-pile book (which should not occur in normal play) produces `""`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{BasicPile, FrenchBasicCard};
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Alice");
    /// let book = BasicPile::from(vec![
    ///     FrenchBasicCard::ACE_SPADES,
    ///     FrenchBasicCard::ACE_HEARTS,
    ///     FrenchBasicCard::ACE_DIAMONDS,
    ///     FrenchBasicCard::ACE_CLUBS,
    /// ]);
    /// player.add_book(book);
    /// assert_eq!(player.book_ranks(), vec!["A".to_string()]);
    /// ```
    #[must_use]
    pub fn book_ranks(&self) -> Vec<String> {
        self.books
            .iter()
            .map(|pile| {
                debug_assert!(!pile.is_empty(), "book pile must not be empty");
                pile.v()
                    .first()
                    .map_or_else(String::new, |card| card.rank.index.to_string())
            })
            .collect()
    }

    /// Returns the number of cards currently in this player's hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Iris");
    /// assert_eq!(player.hand_size(), 0);
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// assert_eq!(player.hand_size(), 1);
    /// ```
    #[must_use]
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }

    /// Returns `true` if this player's hand is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Jack");
    /// assert!(player.hand_is_empty());
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// assert!(!player.hand_is_empty());
    /// ```
    #[must_use]
    pub fn hand_is_empty(&self) -> bool {
        self.hand.is_empty()
    }

    /// Returns the unique ranks currently held in this player's hand.
    ///
    /// The returned list contains each rank at most once and is in the order
    /// first encountered while iterating the hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Kate");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    /// player.receive_card(FrenchBasicCard::ACE_HEARTS);
    /// player.receive_card(FrenchBasicCard::KING_SPADES);
    ///
    /// let ranks = player.held_ranks();
    /// assert_eq!(ranks.len(), 2); // Ace and King — no duplicates
    /// ```
    #[must_use]
    pub fn held_ranks(&self) -> Vec<Pip> {
        let mut seen = std::collections::HashSet::new();
        let mut ranks = Vec::new();
        for card in &self.hand {
            if seen.insert(card.rank) {
                ranks.push(card.rank);
            }
        }
        ranks
    }

    /// Returns `true` if this player is controlled by a bot.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::bot::BotProfile;
    /// use gfcore::prelude::Player;
    ///
    /// let human = Player::new("Leo");
    /// let bot = Player::new_bot("R2D2", BotProfile::random("R2D2"));
    ///
    /// assert!(!human.is_bot());
    /// assert!(bot.is_bot());
    /// ```
    #[must_use]
    pub fn is_bot(&self) -> bool {
        self.profile.is_some()
    }

    /// Returns a read-only reference to the player's hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::Player;
    ///
    /// let mut player = Player::new("Mia");
    /// player.receive_card(FrenchBasicCard::ACE_SPADES);
    ///
    /// assert_eq!(player.hand().len(), 1);
    /// ```
    #[must_use]
    pub fn hand(&self) -> &BasicPile {
        &self.hand
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::FrenchBasicCard;

    #[test]
    fn test_player_new_defaults() {
        let player = Player::new("Alice");
        assert_eq!(player.name, "Alice");
        assert!(player.hand_is_empty());
        assert_eq!(player.hand_size(), 0);
        assert_eq!(player.book_count(), 0);
        assert!(!player.is_bot());
        assert!(player.profile.is_none());
    }

    #[test]
    fn test_player_new_bot() {
        let profile = BotProfile::basic("HAL");
        let player = Player::new_bot("Bot", profile);
        assert!(player.is_bot());
        assert!(player.profile.is_some());
    }

    #[test]
    fn test_player_receive_card_increments_hand_size() {
        let mut player = Player::new("Bob");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        assert_eq!(player.hand_size(), 1);
        player.receive_card(FrenchBasicCard::KING_HEARTS);
        assert_eq!(player.hand_size(), 2);
    }

    #[test]
    fn test_player_give_cards_of_rank_removes_correct_cards() {
        let mut player = Player::new("Carol");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        player.receive_card(FrenchBasicCard::ACE_HEARTS);
        player.receive_card(FrenchBasicCard::KING_SPADES);

        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        let given = player.give_cards_of_rank(&ace_rank);

        assert_eq!(given.len(), 2);
        assert_eq!(player.hand_size(), 1);
        assert!(!player.has_rank(&ace_rank));
    }

    #[test]
    fn test_player_give_cards_of_rank_missing_returns_empty() {
        let mut player = Player::new("Dave");
        player.receive_card(FrenchBasicCard::ACE_SPADES);

        let king_rank = FrenchBasicCard::KING_SPADES.rank;
        let given = player.give_cards_of_rank(&king_rank);

        assert!(given.is_empty());
        assert_eq!(player.hand_size(), 1);
    }

    #[test]
    fn test_player_has_rank() {
        let mut player = Player::new("Eve");
        player.receive_card(FrenchBasicCard::ACE_SPADES);

        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        let king_rank = FrenchBasicCard::KING_SPADES.rank;

        assert!(player.has_rank(&ace_rank));
        assert!(!player.has_rank(&king_rank));
    }

    #[test]
    fn test_player_cards_of_rank_does_not_remove() {
        let mut player = Player::new("Frank");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        player.receive_card(FrenchBasicCard::ACE_HEARTS);

        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        let peek = player.cards_of_rank(&ace_rank);

        assert_eq!(peek.len(), 2);
        assert_eq!(player.hand_size(), 2);
    }

    #[test]
    fn test_player_add_book_increments_count() {
        let mut player = Player::new("Grace");
        assert_eq!(player.book_count(), 0);

        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        player.add_book(book);
        assert_eq!(player.book_count(), 1);
    }

    #[test]
    fn test_player_held_ranks_unique() {
        let mut player = Player::new("Henry");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        player.receive_card(FrenchBasicCard::ACE_HEARTS);
        player.receive_card(FrenchBasicCard::KING_SPADES);

        let ranks = player.held_ranks();
        assert_eq!(ranks.len(), 2);
    }

    #[test]
    fn test_player_held_ranks_empty_hand() {
        let player = Player::new("Iris");
        assert!(player.held_ranks().is_empty());
    }

    #[test]
    fn test_player_hand_ref_is_correct() {
        let mut player = Player::new("Jack");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        assert_eq!(player.hand().len(), 1);
    }

    #[test]
    fn test_player_hand_is_empty_after_give_all() {
        let mut player = Player::new("Kate");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        player.give_cards_of_rank(&ace_rank);
        assert!(player.hand_is_empty());
    }

    #[test]
    fn test_player_book_ranks_empty() {
        let player = Player::new("Alice");
        assert!(player.book_ranks().is_empty());
    }

    #[test]
    fn test_player_book_ranks_one_book() {
        let mut player = Player::new("Alice");
        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        player.add_book(book);
        assert_eq!(player.book_ranks(), vec!["A".to_string()]);
    }

    #[test]
    fn test_player_book_ranks_multiple() {
        let mut player = Player::new("Bob");
        let ace_book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        let king_book = BasicPile::from(vec![
            FrenchBasicCard::KING_SPADES,
            FrenchBasicCard::KING_HEARTS,
            FrenchBasicCard::KING_DIAMONDS,
            FrenchBasicCard::KING_CLUBS,
        ]);
        player.add_book(ace_book);
        player.add_book(king_book);
        assert_eq!(player.book_ranks(), vec!["A".to_string(), "K".to_string()]);
    }
}
