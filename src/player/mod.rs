//! Player types and hand management.
//!
//! This module provides [`Player`] for tracking a player's hand, books, and
//! profile, and [`PlayerView`] / [`AskEntry`] for the serialisable,
//! visibility-filtered snapshot used by bot decision-making and wire protocols.
//!
//! # Quick example
//!
//! ```
//! use cardpack::prelude::FrenchBasicCard;
//! use gfcore::prelude::{Player, PlayerView};
//!
//! let mut alice = Player::new("Alice");
//! alice.receive_card(FrenchBasicCard::ACE_SPADES);
//!
//! let players = vec![alice];
//! let views = PlayerView::from_perspective(&players, 0).unwrap();
//! assert_eq!(views[0].hand_size, 1);
//! ```

#[allow(clippy::module_inception)]
mod player;
mod view;

pub use player::Player;
pub use view::{AskEntry, PlayerView};
