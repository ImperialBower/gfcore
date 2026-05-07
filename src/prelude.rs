//! Convenient re-exports for common `gfcore` types.
//!
//! Import everything you need in one line:
//!
//! ```rust
//! use gfcore::prelude::*;
//! ```

pub use crate::bot::strategy::{BasicStrategy, RandomStrategy};
pub use crate::bot::{BotProfile, BotStrategy};
pub use crate::error::GfError;
pub use crate::game::{Game, GameEvent, GamePhase, GameState, PlayerAction};
pub use crate::player::{AskEntry, Player, PlayerView};
pub use crate::rules::{
    GameVariant, GoFishRules, HappyFamilies, HappyFamiliesRules, Quartet, QuartetRules,
    StandardRules,
};

#[cfg(feature = "history")]
pub use crate::history::{GameCollection, GameRecord, TurnRecord};
