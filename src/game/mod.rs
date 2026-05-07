//! Core game state machine.
//!
//! This module provides the [`Game`] engine, the [`GamePhase`] enum,
//! the [`GameState`] snapshot type, and the [`GameEvent`] enum that describes
//! the outcome of every player action.
//!
//! For player moves see [`action::PlayerAction`].
//!
//! # Quick example
//!
//! ```
//! use gfcore::prelude::{Game, GamePhase, GameVariant, Player};
//!
//! let players = vec![Player::new("Alice"), Player::new("Bob")];
//! let game = Game::new(GameVariant::Standard, players).unwrap();
//! assert_eq!(*game.phase(), GamePhase::WaitingForAsk);
//! assert_eq!(game.current_player(), 0);
//! assert!(!game.is_over());
//! ```

pub mod action;
pub(crate) mod state;

pub use action::PlayerAction;
pub use state::{Game, GameEvent, GamePhase, GameState};
