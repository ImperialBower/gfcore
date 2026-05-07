//! Player actions that drive the Go Fish state machine.
//!
//! Every move a player can make is represented as a [`PlayerAction`] variant.
//! Pass one of these to [`crate::game::Game::act`] to advance the game.
//!
//! # Examples
//!
//! ```
//! use cardpack::prelude::{DeckedBase, Standard52};
//! use gfcore::prelude::{PlayerAction};
//!
//! let pile = Standard52::basic_pile();
//! let ace_rank = pile.v()[0].rank;
//!
//! let ask = PlayerAction::Ask { target: 1, rank: ace_rank };
//! let draw = PlayerAction::Draw;
//!
//! assert!(matches!(ask, PlayerAction::Ask { .. }));
//! assert!(matches!(draw, PlayerAction::Draw));
//! ```

use cardpack::prelude::Pip;
use serde::{Deserialize, Serialize};

/// A move that the current player may submit to [`crate::game::Game::act`].
///
/// The game phase determines which variant is valid:
/// - [`PlayerAction::Ask`] is valid only when the phase is
///   [`crate::game::GamePhase::WaitingForAsk`].
/// - [`PlayerAction::Draw`] is valid only when the phase is
///   [`crate::game::GamePhase::WaitingForDraw`].
///
/// # Examples
///
/// ```
/// use cardpack::prelude::{DeckedBase, Standard52};
/// use gfcore::prelude::PlayerAction;
///
/// let pile = Standard52::basic_pile();
/// let rank = pile.v()[0].rank;
///
/// let ask = PlayerAction::Ask { target: 2, rank };
/// assert!(matches!(ask, PlayerAction::Ask { target: 2, .. }));
///
/// let draw = PlayerAction::Draw;
/// assert_eq!(draw, PlayerAction::Draw);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Ask `target` for all cards of `rank`.
    ///
    /// The current player must already hold at least one card of `rank`,
    /// and `target` must not be the current player.
    ///
    /// # Examples
    ///
    /// ```
    /// use cardpack::prelude::{DeckedBase, Standard52};
    /// use gfcore::prelude::PlayerAction;
    ///
    /// let rank = Standard52::basic_pile().v()[0].rank;
    /// let action = PlayerAction::Ask { target: 1, rank };
    /// assert!(matches!(action, PlayerAction::Ask { target: 1, .. }));
    /// ```
    Ask {
        /// Index of the player being asked.
        target: usize,
        /// The rank being requested.
        rank: Pip,
    },

    /// Draw the top card from the draw pile after a "Go Fish" response.
    ///
    /// This action is only valid when the phase is
    /// [`crate::game::GamePhase::WaitingForDraw`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::PlayerAction;
    ///
    /// let action = PlayerAction::Draw;
    /// assert_eq!(action, PlayerAction::Draw);
    /// ```
    Draw,
}
