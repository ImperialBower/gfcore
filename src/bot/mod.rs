//! Bot AI module: strategies and profiles.
//!
//! This module provides the [`BotStrategy`] trait and two built-in
//! implementations ([`strategy::RandomStrategy`] and
//! [`strategy::BasicStrategy`]), as well as the [`BotProfile`] type that
//! couples a bot's display name with a chosen strategy.
//!
//! # Quick example
//!
//! ```
//! use gfcore::bot::{BotProfile, BotStrategy};
//! use gfcore::prelude::Player;
//!
//! // Create a bot player backed by the basic heuristic strategy.
//! let profile = BotProfile::basic("Harriet");
//! let bot_player = Player::new_bot("Harriet", profile);
//! assert!(bot_player.is_bot());
//! ```

pub mod profile;
pub mod strategy;

pub use profile::BotProfile;
pub use strategy::BotStrategy;
