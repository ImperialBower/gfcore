//! Error types for the `gfcore` Go Fish game engine.
//!
//! All fallible operations in `gfcore` return `Result<T, GfError>`.
//! Match on `GfError` variants to handle specific failure modes in your
//! application.

use std::fmt;

/// The unified error type for all `gfcore` operations.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::GfError;
///
/// let err = GfError::OutOfTurn;
/// assert_eq!(err.to_string(), "action called when it is not this player's turn");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GfError {
    /// The requesting player's hand contains no card of the rank they asked for.
    ///
    /// In standard Go Fish rules a player may only ask for a rank they already hold.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::InvalidAsk;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "asker does not hold any card of the requested rank",
    /// );
    /// ```
    InvalidAsk,

    /// The target of a request is invalid: either the player targeted themselves,
    /// or the target index is out of range for the current player list.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::InvalidTarget;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "invalid target: player targeted themselves or index is out of range",
    /// );
    /// ```
    InvalidTarget,

    /// An action was attempted when it is not the calling player's turn.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::OutOfTurn;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "action called when it is not this player's turn",
    /// );
    /// ```
    OutOfTurn,

    /// The game was started with fewer players than the variant requires.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::NotEnoughPlayers;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "game started with fewer players than the variant minimum",
    /// );
    /// ```
    NotEnoughPlayers,

    /// The game was started with more players than the variant allows.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::TooManyPlayers;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "game started with more players than the variant maximum",
    /// );
    /// ```
    TooManyPlayers,

    /// An action was attempted after the game has already reached the `GameOver` state.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::GameAlreadyOver;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "action called after the game has already ended",
    /// );
    /// ```
    GameAlreadyOver,

    /// Reserved for callers or future engine versions that need to signal a
    /// draw on an empty pile as a hard error.
    ///
    /// The current engine does not emit this variant; when the draw pile is
    /// exhausted it advances the turn and emits [`crate::game::GameEvent::Drew`] with
    /// `matched: false` instead of returning an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::EmptyDrawPile;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "draw attempted on an empty draw pile",
    /// );
    /// ```
    EmptyDrawPile,

    /// A parse or deserialization error, e.g. when loading game history from JSON or YAML.
    ///
    /// The inner `String` contains the human-readable error message from the
    /// underlying deserializer.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::ParseError("unexpected token at line 3".to_string());
    /// assert_eq!(
    ///     err.to_string(),
    ///     "parse error: unexpected token at line 3",
    /// );
    /// ```
    ParseError(String),

    /// A filesystem operation failed during save.
    ///
    /// The inner `String` contains the OS error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::IoError("permission denied".to_string());
    /// assert_eq!(err.to_string(), "io error: permission denied");
    /// ```
    IoError(String),

    /// `replay()` was called on a record where at least one turn has no stored actions.
    ///
    /// This error is expected when replaying records produced before action
    /// recording was added (e.g., WASM games or old tests).
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::GfError;
    ///
    /// let err = GfError::NoReplayData;
    /// assert_eq!(
    ///     err.to_string(),
    ///     "no replay data: record is missing actions or initial deck state",
    /// );
    /// ```
    NoReplayData,
}

impl fmt::Display for GfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAsk => f.write_str("asker does not hold any card of the requested rank"),
            Self::InvalidTarget => {
                f.write_str("invalid target: player targeted themselves or index is out of range")
            }
            Self::OutOfTurn => f.write_str("action called when it is not this player's turn"),
            Self::NotEnoughPlayers => {
                f.write_str("game started with fewer players than the variant minimum")
            }
            Self::TooManyPlayers => {
                f.write_str("game started with more players than the variant maximum")
            }
            Self::GameAlreadyOver => f.write_str("action called after the game has already ended"),
            Self::EmptyDrawPile => f.write_str("draw attempted on an empty draw pile"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::IoError(msg) => write!(f, "io error: {msg}"),
            Self::NoReplayData => {
                f.write_str("no replay data: record is missing actions or initial deck state")
            }
        }
    }
}

impl std::error::Error for GfError {}

/// Convert a [`serde_json::Error`] into a [`GfError::ParseError`].
///
/// This allows using `?` when deserializing JSON game history.
///
/// `serde_json` is an opt-in (format) dependency, so this impl — and its doc
/// test — are only compiled when the `history` feature is enabled. (The `wasm`
/// bridge also pulls `serde_json`, but calls it directly rather than through
/// this conversion.)
///
/// # Examples
///
/// ```
/// use gfcore::prelude::GfError;
///
/// let json_err: Result<serde_json::Value, _> = serde_json::from_str("{bad json");
/// let gf_err = GfError::from(json_err.unwrap_err());
/// assert!(matches!(gf_err, GfError::ParseError(_)));
/// ```
#[cfg(feature = "history")]
impl From<serde_json::Error> for GfError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

/// Convert a [`serde_norway::Error`] into a [`GfError::ParseError`].
///
/// This allows using `?` when deserializing YAML game history (requires the
/// `history` feature).
///
/// This impl — and its doc test — are only compiled when the `history` feature is
/// enabled (which is the default).
///
/// # Examples
///
/// ```
/// use gfcore::prelude::GfError;
///
/// let yaml_err: Result<serde_norway::Value, _> = serde_norway::from_str(":\n  bad: [yaml");
/// let gf_err = GfError::from(yaml_err.unwrap_err());
/// assert!(matches!(gf_err, GfError::ParseError(_)));
/// ```
#[cfg(feature = "history")]
impl From<serde_norway::Error> for GfError {
    fn from(err: serde_norway::Error) -> Self {
        Self::ParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_inequality() {
        assert_ne!(
            GfError::ParseError("one".into()),
            GfError::ParseError("two".into())
        );
    }

    #[test]
    fn test_error_source_is_none() {
        use std::error::Error;
        assert!(GfError::InvalidAsk.source().is_none());
        assert!(GfError::ParseError("x".into()).source().is_none());
    }

    #[test]
    fn test_display_messages_are_non_empty() {
        let variants = [
            GfError::InvalidAsk,
            GfError::InvalidTarget,
            GfError::OutOfTurn,
            GfError::NotEnoughPlayers,
            GfError::TooManyPlayers,
            GfError::GameAlreadyOver,
            GfError::EmptyDrawPile,
            GfError::ParseError("bad".into()),
            GfError::IoError("disk full".into()),
            GfError::NoReplayData,
        ];
        for v in &variants {
            assert!(!v.to_string().is_empty(), "{v:?} Display must not be empty");
        }
    }

    #[test]
    fn test_io_error_display() {
        let err = GfError::IoError("permission denied".to_string());
        assert_eq!(err.to_string(), "io error: permission denied");
    }

    #[test]
    fn test_no_replay_data_display() {
        let err = GfError::NoReplayData;
        assert_eq!(
            err.to_string(),
            "no replay data: record is missing actions or initial deck state",
        );
    }
}
