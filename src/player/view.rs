//! External view of a player — used for bot decisions and wire serialization.

use cardpack::prelude::BasicPile;
use serde::{Deserialize, Serialize};

use crate::error::GfError;

use super::Player;

/// A single entry in the public ask log.
///
/// Records who asked for which rank (by the rank's index character), so that
/// bots can build inferences from the history of asks.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::AskEntry;
///
/// let entry = AskEntry { asker: 0, rank: "A".to_string() };
/// assert_eq!(entry.asker, 0);
/// assert_eq!(entry.rank, "A");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AskEntry {
    /// Index of the player who made the ask.
    pub asker: usize,
    /// The rank index character that was requested (e.g. `"A"` for Ace, `"7"` for Seven).
    pub rank: String,
}

/// An external view of a single player's public state.
///
/// Observers who are not the player themselves see only the hand size, not the
/// actual cards.  The observing player sees their own full hand via `hand: Some(...)`.
///
/// # Examples
///
/// ```
/// use cardpack::prelude::FrenchBasicCard;
/// use gfcore::prelude::{Player, PlayerView};
///
/// let mut alice = Player::new("Alice");
/// alice.receive_card(FrenchBasicCard::ACE_SPADES);
/// let players = vec![alice];
///
/// let views = PlayerView::from_perspective(&players, 0).unwrap();
/// assert_eq!(views.len(), 1);
/// assert!(views[0].hand.is_some()); // Alice sees her own hand
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerView {
    /// The position of this player in the game's player list.
    pub index: usize,
    /// The player's display name.
    pub name: String,
    /// Number of cards in the player's hand (always visible).
    pub hand_size: usize,
    /// The player's actual hand — `Some` only for the observing player themselves.
    pub hand: Option<BasicPile>,
    /// Number of completed books.
    pub books: usize,
    /// Rank index character of each completed book (e.g. `["A","7"]`).
    pub completed_book_ranks: Vec<String>,
}

impl PlayerView {
    /// Builds a view of every player from the perspective of `observer`.
    ///
    /// The observer sees their own full hand (`hand: Some(...)`).
    /// All other players have `hand: None` — only `hand_size` is visible.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::InvalidTarget`] if `observer >= players.len()` and
    /// `players` is non-empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::FrenchBasicCard;
    /// use gfcore::prelude::{Player, PlayerView};
    ///
    /// let mut alice = Player::new("Alice");
    /// alice.receive_card(FrenchBasicCard::ACE_SPADES);
    /// let mut bob = Player::new("Bob");
    /// bob.receive_card(FrenchBasicCard::KING_HEARTS);
    /// bob.receive_card(FrenchBasicCard::KING_SPADES);
    ///
    /// let players = vec![alice, bob];
    ///
    /// // Alice's perspective (observer = 0)
    /// let views = PlayerView::from_perspective(&players, 0)?;
    /// assert_eq!(views.len(), 2);
    /// assert!(views[0].hand.is_some());  // Alice sees her own hand
    /// assert!(views[1].hand.is_none());  // Alice cannot see Bob's hand
    /// assert_eq!(views[1].hand_size, 2); // But she knows Bob has 2 cards
    /// # Ok::<(), gfcore::prelude::GfError>(())
    /// ```
    pub fn from_perspective(
        players: &[Player],
        observer: usize,
    ) -> Result<Vec<PlayerView>, GfError> {
        if !players.is_empty() && observer >= players.len() {
            return Err(GfError::InvalidTarget);
        }
        Ok(players
            .iter()
            .enumerate()
            .map(|(index, player)| {
                let hand = if index == observer {
                    Some(player.hand().clone())
                } else {
                    None
                };
                PlayerView {
                    index,
                    name: player.name.clone(),
                    hand_size: player.hand_size(),
                    hand,
                    books: player.book_count(),
                    completed_book_ranks: player.book_ranks(),
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardpack::prelude::FrenchBasicCard;

    fn make_players() -> Vec<Player> {
        let mut alice = Player::new("Alice");
        alice.receive_card(FrenchBasicCard::ACE_SPADES);
        let mut bob = Player::new("Bob");
        bob.receive_card(FrenchBasicCard::KING_HEARTS);
        bob.receive_card(FrenchBasicCard::KING_SPADES);
        vec![alice, bob]
    }

    #[test]
    fn test_view_from_perspective_observer_sees_own_hand() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();

        assert_eq!(views.len(), 2);
        assert!(views[0].hand.is_some());
        assert_eq!(views[0].hand.as_ref().map(BasicPile::len), Some(1));
    }

    #[test]
    fn test_view_from_perspective_others_hand_is_none() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        assert!(views[1].hand.is_none());
    }

    #[test]
    fn test_view_hand_size_always_visible() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        assert_eq!(views[0].hand_size, 1);
        assert_eq!(views[1].hand_size, 2);
    }

    #[test]
    fn test_view_indices_correct() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        assert_eq!(views[0].index, 0);
        assert_eq!(views[1].index, 1);
    }

    #[test]
    fn test_view_names_correct() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        assert_eq!(views[0].name, "Alice");
        assert_eq!(views[1].name, "Bob");
    }

    #[test]
    fn test_view_book_count() {
        let mut players = make_players();
        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        players[0].add_book(book);
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        assert_eq!(views[0].books, 1);
        assert_eq!(views[1].books, 0);
        assert_eq!(views[0].completed_book_ranks, vec!["A".to_string()]);
        assert!(views[1].completed_book_ranks.is_empty());
    }

    #[test]
    fn test_view_bob_as_observer() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 1).unwrap();
        // Bob (observer=1) sees his own hand; Alice's is hidden
        assert!(views[0].hand.is_none());
        assert!(views[1].hand.is_some());
    }

    #[test]
    fn test_view_empty_players() {
        let views = PlayerView::from_perspective(&[], 0).unwrap();
        assert!(views.is_empty());
    }

    #[test]
    fn test_view_invalid_observer_returns_err() {
        let players = make_players();
        let result = PlayerView::from_perspective(&players, 99);
        assert_eq!(result, Err(GfError::InvalidTarget));
    }

    #[test]
    fn test_ask_entry_fields() {
        let entry = AskEntry {
            asker: 3,
            rank: "7".to_string(),
        };
        assert_eq!(entry.asker, 3);
        assert_eq!(entry.rank, "7");
    }

    #[test]
    fn test_ask_entry_partial_eq() {
        let a = AskEntry {
            asker: 0,
            rank: "A".to_string(),
        };
        let b = AskEntry {
            asker: 0,
            rank: "A".to_string(),
        };
        let c = AskEntry {
            asker: 1,
            rank: "K".to_string(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_view_serialization_round_trip() {
        let players = make_players();
        let views = PlayerView::from_perspective(&players, 0).unwrap();
        let json = serde_json::to_string(&views[0]).expect("serialize");
        let back: PlayerView = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, "Alice");
        assert_eq!(back.hand_size, 1);
    }

    #[test]
    fn test_player_view_partial_eq() {
        let players = make_players();
        let views1 = PlayerView::from_perspective(&players, 0).unwrap();
        let views2 = PlayerView::from_perspective(&players, 0).unwrap();
        assert_eq!(views1[0], views2[0]);
        assert_ne!(views1[0], views1[1]);
    }
}
