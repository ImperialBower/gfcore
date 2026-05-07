//! `gfcore` — Go Fish card game engine.
//!
//! Built on [`cardpack`](https://crates.io/crates/cardpack) v0.7 card primitives.
//! Supports multiple Go Fish variants, bot AI players, and YAML/JSON game history.
//!
//! # Quick Start
//!
//! ```
//! use gfcore::prelude::*;
//!
//! // One human player, one bot backed by the basic heuristic strategy.
//! let players = vec![
//!     Player::new("Alice"),
//!     Player::new_bot("HAL", BotProfile::basic("HAL")),
//! ];
//!
//! let mut game = Game::new(GameVariant::Standard, players).unwrap();
//! assert!(!game.is_over());
//! assert_eq!(game.current_player(), 0);
//!
//! // Inspect the initial state — always starts in WaitingForAsk.
//! let state = game.state().unwrap();
//! assert_eq!(state.phase, GamePhase::WaitingForAsk);
//! assert_eq!(state.players.len(), 2);
//! assert!(state.ask_log.is_empty());
//! ```
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---------|---------|---------|
//! | `history` | ✓ | [`history::GameRecord`], YAML/JSON round-trips |
//! | `wasm` | — | `#[wasm_bindgen]` exports for browser play |

#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

pub mod bot;
pub mod error;
pub mod game;
pub mod player;
pub mod rules;

#[cfg(feature = "history")]
pub mod history;

pub mod prelude;

#[cfg(feature = "wasm")]
mod wasm_api;

#[cfg(feature = "wasm")]
pub use wasm_api::*;
