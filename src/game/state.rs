//! Go Fish game state machine.
//!
//! The core of `gfcore`: a [`Game`] struct drives all state transitions for a
//! game of Go Fish.  Callers advance the game by calling [`Game::act`] with a
//! [`crate::game::PlayerAction`], and inspect the current situation through
//! [`Game::state`] which returns a serialisable [`GameState`] snapshot.
//!
//! # Phases
//!
//! The game cycles through these phases:
//!
//! 1. [`GamePhase::WaitingForAsk`] — the current player must submit
//!    `PlayerAction::Ask { target, rank }`.
//! 2. [`GamePhase::WaitingForDraw`] — "Go Fish" was called; the current player
//!    must submit `PlayerAction::Draw`.
//! 3. [`GamePhase::BookCompleted`] — a book was just completed; the same player
//!    asks again next.  This phase is transient; the engine returns to
//!    `WaitingForAsk` immediately.
//! 4. [`GamePhase::GameOver`] — terminal state; no further actions are accepted.

use cardpack::prelude::{BasicPile, Pip};
use serde::{Deserialize, Serialize};

use crate::error::GfError;
use crate::game::action::PlayerAction;
use crate::player::{AskEntry, Player, PlayerView};
use crate::rules::{GameVariant, GoFishRules};

#[cfg(feature = "history")]
use crate::history::{GameRecord, TurnRecord};

// ---------------------------------------------------------------------------
// GamePhase
// ---------------------------------------------------------------------------

/// The current phase of the Go Fish state machine.
///
/// The phase determines which [`PlayerAction`] the current player may submit.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::{GamePhase, GameVariant, Game, Player};
///
/// let players = vec![Player::new("Alice"), Player::new("Bob")];
/// let game = Game::new(GameVariant::Standard, players).unwrap();
/// assert_eq!(*game.phase(), GamePhase::WaitingForAsk);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GamePhase {
    /// The current player must submit `PlayerAction::Ask { target, rank }`.
    WaitingForAsk,
    /// "Go Fish" was called — the current player must submit `PlayerAction::Draw`.
    WaitingForDraw,
    /// A book was just completed; the same player asks again next.
    BookCompleted,
    /// Terminal state — no further actions are accepted.
    GameOver,
}

// ---------------------------------------------------------------------------
// GameEvent
// ---------------------------------------------------------------------------

/// An event emitted by the game engine as a result of a player action.
///
/// [`Game::act`] returns the most recently emitted event for that action.
/// Observers can use a sequence of events to reconstruct the full game
/// history; the `history` feature enables structured turn-by-turn recording.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::{Game, GameEvent, GameVariant, Player, PlayerAction};
/// use cardpack::prelude::{DeckedBase, Standard52};
///
/// // Build a minimal game; the exact events depend on the shuffled deck.
/// let players = vec![Player::new("Alice"), Player::new("Bob")];
/// let mut game = Game::new(GameVariant::Standard, players).unwrap();
/// // The game starts in WaitingForAsk — we can inspect state safely.
/// let state = game.state().unwrap();
/// assert!(state.last_event.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    /// The current player asked another player for cards of a rank.
    ///
    /// Note: [`Game::act`] returns the *final* event for each action (either
    /// [`GameEvent::GoFish`] or [`GameEvent::Gave`]), so this variant does not
    /// appear in the return value of `act()`.  It is present for constructing
    /// [`crate::history::TurnRecord`]s manually or via future event-log APIs.
    Asked {
        /// Index of the asking player.
        asker: usize,
        /// Index of the player being asked.
        target: usize,
        /// String representation of the rank's index character.
        rank: String,
    },
    /// The target player gave cards to the asker.
    Gave {
        /// Index of the player giving the cards.
        from: usize,
        /// Index of the player receiving the cards.
        to: usize,
        /// String representation of the rank's index character.
        rank: String,
        /// Number of cards transferred.
        count: usize,
    },
    /// The current player was told to "Go Fish" (target had no matching cards).
    GoFish {
        /// Index of the player who must draw.
        player: usize,
    },
    /// The current player drew a card from the draw pile.
    Drew {
        /// Index of the player who drew.
        player: usize,
        /// `true` if the drawn card matches the rank the player asked for.
        matched: bool,
    },
    /// A player completed a book.
    Book {
        /// Index of the player who completed the book.
        player: usize,
        /// String representation of the rank's index character.
        rank: String,
    },
    /// The game has ended.
    GameOver {
        /// Index of the winning player, or `None` if tied.
        winner: Option<usize>,
    },
}

// ---------------------------------------------------------------------------
// GameState (serialisable snapshot)
// ---------------------------------------------------------------------------

/// A serialisable snapshot of the game for callers, bots, and wire protocols.
///
/// Build one by calling [`Game::state`].  The `ask_log` field records every
/// ask that has been made since game start — bots can use it to infer what
/// ranks other players hold.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::{Game, GamePhase, GameVariant, Player};
///
/// let players = vec![Player::new("Alice"), Player::new("Bob")];
/// let game = Game::new(GameVariant::Standard, players).unwrap();
/// let state = game.state().unwrap();
/// assert_eq!(state.current_player, 0);
/// assert_eq!(state.phase, GamePhase::WaitingForAsk);
/// assert!(state.last_event.is_none());
/// assert!(state.ask_log.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The current phase of the game.
    pub phase: GamePhase,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// Public view of every player from the perspective of the current player.
    pub players: Vec<PlayerView>,
    /// Number of cards remaining in the draw pile.
    pub draw_pile_size: usize,
    /// The most recent event emitted by the engine, if any.
    pub last_event: Option<GameEvent>,
    /// The index of the winning player (set once the game reaches `GameOver`).
    pub winner: Option<usize>,
    /// Chronological log of every ask made in the game, oldest first.
    pub ask_log: Vec<AskEntry>,
}

// ---------------------------------------------------------------------------
// Game (engine)
// ---------------------------------------------------------------------------

/// The Go Fish game engine.
///
/// Create a game with [`Game::new`], then call [`Game::act`] repeatedly to
/// advance through actions.  Inspect the public state with [`Game::state`].
///
/// # Examples
///
/// ```
/// use gfcore::prelude::{Game, GameVariant, Player};
///
/// let players = vec![Player::new("Alice"), Player::new("Bob")];
/// let game = Game::new(GameVariant::Standard, players).unwrap();
/// assert_eq!(game.current_player(), 0);
/// assert!(!game.is_over());
/// ```
pub struct Game {
    /// The variant rules in use.
    variant: GameVariant,
    /// All players in turn order.
    players: Vec<Player>,
    /// The remaining draw pile.
    draw_pile: BasicPile,
    /// Index of the player whose turn it is.
    current_player: usize,
    /// Current phase of the state machine.
    phase: GamePhase,
    /// The rank the current player asked for before going to `WaitingForDraw`.
    last_asked_rank: Option<Pip>,
    /// The most recent event emitted.
    last_event: Option<GameEvent>,
    /// The winner once the game is over.
    winner: Option<usize>,
    /// Chronological log of every ask made since game start.
    ask_log: Vec<AskEntry>,
    /// The player whose turn is currently being recorded.
    #[cfg(feature = "history")]
    pending_turn_player: usize,
    /// Events accumulated for the current turn, flushed when the turn ends.
    #[cfg(feature = "history")]
    pending_turn_events: Vec<GameEvent>,
    /// Actions submitted during the current turn, flushed into [`TurnRecord::actions`].
    #[cfg(feature = "history")]
    pending_turn_actions: Vec<PlayerAction>,
    /// Completed turn records plus metadata; grows as turns are flushed.
    #[cfg(feature = "history")]
    history: GameRecord,
}

impl Game {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Creates a new game and deals the initial hands.
    ///
    /// Returns an error if the player count is outside the variant's allowed
    /// range.  After construction the game is in
    /// [`GamePhase::WaitingForAsk`] with player 0 to act first.
    ///
    /// # Errors
    ///
    /// - [`GfError::NotEnoughPlayers`] — fewer players than `rules.min_players()`.
    /// - [`GfError::TooManyPlayers`] — more players than `rules.max_players()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players);
    /// assert!(game.is_ok());
    /// ```
    pub fn new(variant: GameVariant, players: Vec<Player>) -> Result<Self, GfError> {
        let draw_pile = variant.rules().deck();
        Self::setup(variant, players, draw_pile)
    }

    /// Creates a game using the provided pre-built draw pile instead of dealing from a
    /// freshly shuffled deck.  Used by [`crate::history::replay`] to reconstruct the
    /// exact initial state recorded in a [`crate::history::GameRecord`].
    #[cfg(feature = "history")]
    pub(crate) fn new_with_deck(
        variant: GameVariant,
        players: Vec<Player>,
        draw_pile: cardpack::prelude::BasicPile,
    ) -> Result<Self, GfError> {
        Self::setup(variant, players, draw_pile)
    }

    fn setup(
        variant: GameVariant,
        mut players: Vec<Player>,
        mut draw_pile: cardpack::prelude::BasicPile,
    ) -> Result<Self, GfError> {
        let rules = variant.rules();
        let player_count = players.len();

        if player_count < rules.min_players() {
            return Err(GfError::NotEnoughPlayers);
        }
        if player_count > rules.max_players() {
            return Err(GfError::TooManyPlayers);
        }

        // Build the initial GameRecord before dealing (captures initial_draw_pile).
        #[cfg(feature = "history")]
        let history = {
            let player_names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();
            let mut record = GameRecord::new(variant.rules().name(), player_names);
            record.initial_draw_pile = Some(draw_pile.clone());
            record
        };

        let hand_size = rules.initial_hand_size(player_count);

        // Deal cards in round-robin order.
        for _ in 0..hand_size {
            for player in &mut players {
                if let Some(card) = draw_pile.draw_first() {
                    player.receive_card(card);
                }
            }
        }

        // Check for immediate books in initial hands.
        for player in &mut players {
            Self::collect_books_for_player(player, rules);
        }

        let mut game = Self {
            variant,
            players,
            draw_pile,
            current_player: 0,
            phase: GamePhase::WaitingForAsk,
            last_asked_rank: None,
            last_event: None,
            winner: None,
            ask_log: Vec::new(),
            #[cfg(feature = "history")]
            pending_turn_player: 0,
            #[cfg(feature = "history")]
            pending_turn_events: Vec::new(),
            #[cfg(feature = "history")]
            pending_turn_actions: Vec::new(),
            #[cfg(feature = "history")]
            history,
        };

        // Handle the edge case where player 0's initial hand is empty after
        // book collection (e.g., all dealt cards formed books immediately).
        // Ensures the invariant "current player has cards in WaitingForAsk"
        // holds from the very first action.
        game.ensure_current_player_can_ask();

        Ok(game)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Submits a player action to advance the game state.
    ///
    /// Returns the most recently emitted [`GameEvent`].  If the action causes
    /// multiple events (e.g., a `Gave` followed by a `Book`), only the final
    /// event is returned; callers interested in all intermediate events should
    /// observe state transitions between calls.
    ///
    /// # Game-over conditions (extension beyond strict rules)
    ///
    /// The standard Go Fish end condition is "all hands empty".  This engine
    /// also ends the game when the draw pile is empty **and** no productive ask
    /// is possible anywhere — i.e., no rank held by any player is simultaneously
    /// held by a different player.  This deadlock-detection extension prevents
    /// infinite loops in edge cases where isolated single-card ranks in separate
    /// hands would otherwise never resolve into books.
    ///
    /// # Errors
    ///
    /// - [`GfError::GameAlreadyOver`] — called after the game has ended.
    /// - [`GfError::OutOfTurn`] — wrong action for the current phase (e.g.,
    ///   `Ask` when waiting for `Draw`).
    /// - [`GfError::InvalidTarget`] — the target is the current player, or the
    ///   target index is out of bounds.
    /// - [`GfError::InvalidAsk`] — the current player does not hold any card of
    ///   the requested rank.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GameEvent, GamePhase, GameVariant, Player, PlayerAction};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let mut game = Game::new(GameVariant::Standard, players).unwrap();
    ///
    /// // Calling Draw when WaitingForAsk returns an error.
    /// let err = game.act(PlayerAction::Draw);
    /// assert!(err.is_err());
    /// ```
    pub fn act(&mut self, action: PlayerAction) -> Result<GameEvent, GfError> {
        if self.phase == GamePhase::GameOver {
            return Err(GfError::GameAlreadyOver);
        }

        match action {
            PlayerAction::Ask { target, rank } => self.handle_ask(target, rank),
            PlayerAction::Draw => self.handle_draw(),
        }
    }

    /// Returns a serialisable snapshot of the current game state.
    ///
    /// The player views are built from the perspective of the current player:
    /// they see their own full hand; all other players' hands are hidden.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::InvalidTarget`] if the internal `current_player` index
    /// is somehow out of range for the players list (should not occur in a
    /// correctly constructed game, but propagated for safety).
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GamePhase, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players).unwrap();
    /// let state = game.state().unwrap();
    /// assert_eq!(state.phase, GamePhase::WaitingForAsk);
    /// assert_eq!(state.current_player, 0);
    /// assert_eq!(state.players.len(), 2);
    /// ```
    pub fn state(&self) -> Result<GameState, GfError> {
        let players = PlayerView::from_perspective(&self.players, self.current_player)?;

        Ok(GameState {
            phase: self.phase.clone(),
            current_player: self.current_player,
            players,
            draw_pile_size: self.draw_pile.len(),
            last_event: self.last_event.clone(),
            winner: self.winner,
            ask_log: self.ask_log.clone(),
        })
    }

    /// Like [`Self::state`] but always reveals `observer`'s hand regardless of whose
    /// turn it is.  Used by the WASM layer so the human's hand stays visible
    /// throughout bot turns.
    ///
    /// # Errors
    ///
    /// Returns [`GfError::InvalidTarget`] if `observer` is out of range for the current player list.
    pub fn state_as_observer(&self, observer: usize) -> Result<GameState, GfError> {
        let players = PlayerView::from_perspective(&self.players, observer)?;

        Ok(GameState {
            phase: self.phase.clone(),
            current_player: self.current_player,
            players,
            draw_pile_size: self.draw_pile.len(),
            last_event: self.last_event.clone(),
            winner: self.winner,
            ask_log: self.ask_log.clone(),
        })
    }

    /// Returns the index of the player whose turn it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players).unwrap();
    /// assert_eq!(game.current_player(), 0);
    /// ```
    #[must_use]
    pub fn current_player(&self) -> usize {
        self.current_player
    }

    /// Returns a reference to the current game phase.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GamePhase, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players).unwrap();
    /// assert_eq!(*game.phase(), GamePhase::WaitingForAsk);
    /// ```
    #[must_use]
    pub fn phase(&self) -> &GamePhase {
        &self.phase
    }

    /// Returns `true` if the game has reached the [`GamePhase::GameOver`] state.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players).unwrap();
    /// assert!(!game.is_over());
    /// ```
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.phase == GamePhase::GameOver
    }

    /// Returns a snapshot of the game history accumulated so far.
    ///
    /// The record grows automatically as [`Game::act`] is called.  Call this
    /// after [`Game::is_over`] returns `true` to get the complete record.
    ///
    /// # Examples
    ///
    /// ```
    /// use gfcore::prelude::{Game, GameVariant, Player};
    ///
    /// let players = vec![Player::new("Alice"), Player::new("Bob")];
    /// let game = Game::new(GameVariant::Standard, players).unwrap();
    /// let record = game.record();
    /// assert_eq!(record.variant, "Standard Go Fish");
    /// assert_eq!(record.players, vec!["Alice", "Bob"]);
    /// assert!(record.turns.is_empty());
    /// ```
    #[cfg(feature = "history")]
    pub fn record(&self) -> GameRecord {
        let mut record = self.history.clone();
        if !self.pending_turn_events.is_empty() {
            let books: Vec<usize> = self.players.iter().map(Player::book_count).collect();
            record.turns.push(TurnRecord {
                player: self.pending_turn_player,
                events: self.pending_turn_events.clone(),
                books_after_turn: books,
                actions: if self.pending_turn_actions.is_empty() {
                    None
                } else {
                    Some(self.pending_turn_actions.clone())
                },
            });
        }
        record
    }

    // -----------------------------------------------------------------------
    // Private helpers — action handlers
    // -----------------------------------------------------------------------

    /// Handles a `PlayerAction::Ask`.
    fn handle_ask(&mut self, target: usize, rank: Pip) -> Result<GameEvent, GfError> {
        if self.phase != GamePhase::WaitingForAsk && self.phase != GamePhase::BookCompleted {
            return Err(GfError::OutOfTurn);
        }

        let cp = self.current_player;

        // Validate target.
        if target == cp || target >= self.players.len() {
            return Err(GfError::InvalidTarget);
        }

        // The asker must satisfy the variant's ask rule.
        if !self
            .variant
            .rules()
            .is_valid_ask(self.players[cp].hand(), &rank)
        {
            return Err(GfError::InvalidAsk);
        }

        // Record the validated action for replay.
        #[cfg(feature = "history")]
        self.pending_turn_actions
            .push(PlayerAction::Ask { target, rank });

        // Record the ask in the public log before branching on Go-Fish/give.
        self.ask_log.push(AskEntry {
            asker: cp,
            rank: rank.index.to_string(),
        });

        // Emit Asked event.
        let asked_event = GameEvent::Asked {
            asker: cp,
            target,
            rank: rank.index.to_string(),
        };
        self.last_event = Some(asked_event.clone());
        #[cfg(feature = "history")]
        self.push_event(&asked_event);

        // Check whether target has matching cards.
        let transferred = self.players[target].give_cards_of_rank(&rank);

        if transferred.is_empty() {
            // Go Fish.
            let event = GameEvent::GoFish { player: cp };
            self.last_event = Some(event.clone());
            #[cfg(feature = "history")]
            self.push_event(&event);
            self.phase = GamePhase::WaitingForDraw;
            self.last_asked_rank = Some(rank);
            return Ok(event);
        }

        // Transfer cards.
        let count = transferred.len();
        for card in transferred {
            self.players[cp].receive_card(card);
        }

        let gave_event = GameEvent::Gave {
            from: target,
            to: cp,
            rank: rank.index.to_string(),
            count,
        };
        self.last_event = Some(gave_event.clone());
        #[cfg(feature = "history")]
        self.push_event(&gave_event);

        // Check for a completed book.
        let rules = self.variant.rules();
        let last_event = if Self::check_and_collect_book(&mut self.players, cp, &rank, rules) {
            let book_event = GameEvent::Book {
                player: cp,
                rank: rank.index.to_string(),
            };
            self.last_event = Some(book_event.clone());
            #[cfg(feature = "history")]
            self.push_event(&book_event);
            book_event
        } else {
            gave_event
        };

        // Transition to BookCompleted if a book was just made; otherwise stay
        // on WaitingForAsk (same player keeps turn in both cases).
        self.phase = match &last_event {
            GameEvent::Book { .. } => GamePhase::BookCompleted,
            _ => GamePhase::WaitingForAsk,
        };

        // If the current player's hand is now empty (all cards went into
        // a book), they cannot ask again — replenish or skip them.
        self.ensure_current_player_can_ask();

        // Win-condition check (may overwrite phase to GameOver).
        if let Some(over_event) = self.check_win_condition() {
            return Ok(over_event);
        }

        Ok(last_event)
    }

    /// Handles a `PlayerAction::Draw`.
    fn handle_draw(&mut self) -> Result<GameEvent, GfError> {
        if self.phase != GamePhase::WaitingForDraw {
            return Err(GfError::OutOfTurn);
        }

        // Record the validated action for replay.
        #[cfg(feature = "history")]
        self.pending_turn_actions.push(PlayerAction::Draw);

        // Save and clear last_asked_rank atomically so early-exit paths never
        // leave a stale value while the match check below still uses it.
        let asked_rank = self.last_asked_rank.take();

        let cp = self.current_player;

        if self.draw_pile.is_empty() {
            // No card to draw — advance turn and return to WaitingForAsk.
            let drew_event = GameEvent::Drew {
                player: cp,
                matched: false,
            };
            self.last_event = Some(drew_event.clone());
            #[cfg(feature = "history")]
            self.push_event(&drew_event);
            self.advance_turn();
            self.phase = GamePhase::WaitingForAsk;
            self.ensure_current_player_can_ask();

            if let Some(over_event) = self.check_win_condition() {
                return Ok(over_event);
            }
            return Ok(drew_event);
        }

        // Draw the top card.
        let Some(card) = self.draw_pile.draw_first() else {
            // draw_pile was not empty (we checked above), but draw_first
            // returned None — treat defensively as if pile empty.
            let drew_event = GameEvent::Drew {
                player: cp,
                matched: false,
            };
            self.last_event = Some(drew_event.clone());
            #[cfg(feature = "history")]
            self.push_event(&drew_event);
            self.advance_turn();
            self.phase = GamePhase::WaitingForAsk;
            self.ensure_current_player_can_ask();
            if let Some(over_event) = self.check_win_condition() {
                return Ok(over_event);
            }
            return Ok(drew_event);
        };

        let drawn_rank = card.rank;
        let matched = asked_rank == Some(drawn_rank);

        self.players[cp].receive_card(card);

        let drew_event = GameEvent::Drew {
            player: cp,
            matched,
        };
        self.last_event = Some(drew_event.clone());
        #[cfg(feature = "history")]
        self.push_event(&drew_event);

        // Always check the drawn card's rank for book completion — a
        // non-matching draw can still complete a book of its own rank
        // (player held three of X, asked for Y, drew the fourth X).
        let rules = self.variant.rules();
        let book_formed = Self::check_and_collect_book(&mut self.players, cp, &drawn_rank, rules);

        let last_event = if book_formed {
            let book_event = GameEvent::Book {
                player: cp,
                rank: drawn_rank.index.to_string(),
            };
            self.last_event = Some(book_event.clone());
            #[cfg(feature = "history")]
            self.push_event(&book_event);
            if matched {
                // Drew the asked rank and completed a book — same player acts again.
                self.phase = GamePhase::BookCompleted;
            } else {
                // Drew a non-asked rank that completed a book — turn still ends.
                self.advance_turn();
                self.phase = GamePhase::WaitingForAsk;
            }
            book_event
        } else if matched {
            // Drew the asked rank but no book yet — same player keeps turn.
            self.phase = GamePhase::WaitingForAsk;
            drew_event
        } else {
            // Didn't match — next player's turn.
            self.advance_turn();
            self.phase = GamePhase::WaitingForAsk;
            drew_event
        };

        // Ensure the current player (possibly a new one after advance_turn,
        // or the same one who just completed a book) has cards to act with.
        self.ensure_current_player_can_ask();

        if let Some(over_event) = self.check_win_condition() {
            return Ok(over_event);
        }

        Ok(last_event)
    }

    // -----------------------------------------------------------------------
    // Private helpers — turn management
    // -----------------------------------------------------------------------

    /// Replenishes `player_index`'s hand by drawing cards from the pile until
    /// they have at least one card that is NOT immediately turned into a book,
    /// or the draw pile runs out.
    ///
    /// Returns `true` if the player ends up with at least one card in hand.
    #[mutants::skip] // loop bound (+1) and fallthrough are unreachable; draw_pile.is_empty() always fires first
    fn replenish_until_has_cards(&mut self, player_index: usize) -> bool {
        if player_index >= self.players.len() {
            return false;
        }
        // Draw cards until the player holds at least one or pile is exhausted.
        // Bound the loop to the pile size to prevent any infinite loop.
        let max_draws = self.draw_pile.len() + 1;
        for _ in 0..max_draws {
            if !self.players[player_index].hand_is_empty() {
                return true;
            }
            if self.draw_pile.is_empty() {
                return false;
            }
            if let Some(card) = self.draw_pile.draw_first() {
                let r = card.rank;
                self.players[player_index].receive_card(card);
                // If the drawn card completes a book, it is removed from the hand.
                let rules = self.variant.rules();
                Self::check_and_collect_book(&mut self.players, player_index, &r, rules);
            }
        }
        !self.players[player_index].hand_is_empty()
    }

    /// Advances `current_player` to the next player who can act.
    ///
    /// A player can act if they have cards in hand.  If their hand is empty
    /// and the draw pile is not empty, cards are dealt to them until they hold
    /// at least one non-book card.  Players whose hands remain empty after
    /// exhausting the pile are skipped.
    ///
    /// If every player is unable to act and the pile is empty,
    /// `current_player` is still advanced (the win-condition check that
    /// follows will transition to `GameOver`).
    fn advance_turn(&mut self) {
        // Flush the outgoing player's turn before changing current_player.
        #[cfg(feature = "history")]
        self.flush_turn();

        let count = self.players.len();
        if count == 0 {
            return;
        }

        let start = self.current_player;
        let mut next = (start + 1) % count;

        for _ in 0..count {
            // If this player has cards they can act.
            if !self.players[next].hand_is_empty() {
                self.current_player = next;
                #[cfg(feature = "history")]
                {
                    self.pending_turn_player = next;
                }
                return;
            }
            // Try to replenish from the pile.
            if self.replenish_until_has_cards(next) {
                self.current_player = next;
                #[cfg(feature = "history")]
                {
                    self.pending_turn_player = next;
                }
                return;
            }
            // This player is out and so is the pile — skip them.
            next = (next + 1) % count;
        }

        // Everyone is out and the pile is empty.  Set current_player to the
        // slot after the one that just acted; check_win_condition will end
        // the game momentarily.
        self.current_player = next;
        #[cfg(feature = "history")]
        {
            self.pending_turn_player = next;
        }
    }

    /// Ensures that `current_player` has cards to act with while phase is
    /// `WaitingForAsk`.  If their hand became empty (e.g., after completing a
    /// book that used all remaining cards) and the pile is also empty, the
    /// engine advances the turn.  If the pile is not empty it replenishes
    /// their hand.
    ///
    /// Must be called after any state change that could leave the current
    /// player's hand empty while the phase is `WaitingForAsk`.
    fn ensure_current_player_can_ask(&mut self) {
        if self.phase != GamePhase::WaitingForAsk && self.phase != GamePhase::BookCompleted {
            return;
        }
        let cp = self.current_player;
        if self.players[cp].hand_is_empty() {
            // Try to replenish.
            if !self.replenish_until_has_cards(cp) {
                // Pile is empty and hand is empty — advance to someone who can
                // act (or everyone is out, ending the game).
                self.advance_turn();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers — history recording
    // -----------------------------------------------------------------------

    /// Appends `event` to the pending turn's event list.
    #[cfg(feature = "history")]
    fn push_event(&mut self, event: &GameEvent) {
        self.pending_turn_events.push(event.clone());
    }

    /// Flushes the pending turn into `history` and resets the accumulator.
    ///
    /// No-op if no events have been accumulated since the last flush, so it is
    /// safe to call unconditionally at the start of every `advance_turn()`.
    #[cfg(feature = "history")]
    fn flush_turn(&mut self) {
        if self.pending_turn_events.is_empty() {
            return;
        }
        let books: Vec<usize> = self.players.iter().map(Player::book_count).collect();
        self.history.turns.push(TurnRecord {
            player: self.pending_turn_player,
            events: std::mem::take(&mut self.pending_turn_events),
            books_after_turn: books,
            actions: if self.pending_turn_actions.is_empty() {
                None
            } else {
                Some(std::mem::take(&mut self.pending_turn_actions))
            },
        });
    }

    // -----------------------------------------------------------------------
    // Private helpers — book collection
    // -----------------------------------------------------------------------

    /// Checks whether `player` now holds a complete book of `rank` according to
    /// the variant's `is_book` rule and, if so, removes the cards from the hand
    /// and records a book.  Returns `true` if a book was collected.
    ///
    /// Takes `players` as an explicit slice rather than `&mut self` so that
    /// callers can hold a shared borrow of `self.variant` (for `rules`) at the
    /// same time as a mutable borrow of `self.players` — the two fields do not
    /// alias.
    fn check_and_collect_book(
        players: &mut [Player],
        player: usize,
        rank: &Pip,
        rules: &dyn GoFishRules,
    ) -> bool {
        if player >= players.len() {
            return false;
        }
        let cards_of_rank = players[player].cards_of_rank(rank);
        if rules.is_book(&cards_of_rank) {
            let book = players[player].give_cards_of_rank(rank);
            players[player].add_book(book);
            return true;
        }
        false
    }

    /// Drains all completed books from `player`'s hand on game start.
    fn collect_books_for_player(player: &mut Player, rules: &dyn GoFishRules) {
        let ranks: Vec<Pip> = player.held_ranks();
        for rank in ranks {
            let cards = player.cards_of_rank(&rank);
            if rules.is_book(&cards) {
                let book = player.give_cards_of_rank(&rank);
                player.add_book(book);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private helpers — win condition
    // -----------------------------------------------------------------------

    /// Returns `true` if no productive ask is possible anywhere in the game.
    ///
    /// A productive ask requires that some player `A` holds rank `R` and some
    /// other player `B ≠ A` also holds at least one card of rank `R`.  When no
    /// such pair exists (and the pile is empty), no book can ever be completed
    /// by asking — the game is deadlocked.
    fn no_productive_ask_exists(&self) -> bool {
        // For each player who has cards, check whether any of their ranks
        // appears in another player's hand.
        for (i, player) in self.players.iter().enumerate() {
            if player.hand_is_empty() {
                continue;
            }
            for rank in player.held_ranks() {
                // Does any OTHER player have this rank?
                let other_has = self
                    .players
                    .iter()
                    .enumerate()
                    .any(|(j, other)| j != i && other.has_rank(&rank));
                if other_has {
                    return false; // a productive ask exists
                }
            }
        }
        true // no productive ask found
    }

    /// Checks whether the game-over condition is met and, if so, transitions
    /// to [`GamePhase::GameOver`].  Returns the `GameOver` event if the game
    /// just ended, or `None` if play should continue.
    ///
    /// Game over when the draw pile is empty **and** one of the following:
    /// - Every player has an empty hand (all books are formed), OR
    /// - No productive ask is possible: no rank held by any player is also
    ///   held by a different player, making further book formation impossible.
    fn check_win_condition(&mut self) -> Option<GameEvent> {
        if !self.draw_pile.is_empty() {
            return None;
        }

        // Primary condition: all hands empty.
        let all_hands_empty = self.players.iter().all(Player::hand_is_empty);

        // Secondary condition: no productive ask is possible anywhere.
        // Checked only when the pile is empty (expensive for large player counts).
        let deadlocked = !all_hands_empty && self.no_productive_ask_exists();

        if !all_hands_empty && !deadlocked {
            return None;
        }

        // Find the player(s) with the highest book count.
        let max_books = self
            .players
            .iter()
            .map(Player::book_count)
            .max()
            .unwrap_or(0);
        let winners: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.book_count() == max_books)
            .map(|(i, _)| i)
            .collect();

        let winner = if winners.len() == 1 {
            winners.first().copied()
        } else {
            None
        };

        self.winner = winner;
        self.phase = GamePhase::GameOver;

        let event = GameEvent::GameOver { winner };
        self.last_event = Some(event.clone());

        #[cfg(feature = "history")]
        {
            // Include GameOver in the current turn only if there are already
            // events for this turn (i.e., the game ended mid-turn without an
            // advance_turn flush).  If the turn was already flushed by
            // advance_turn, we don't append a synthetic GameOver-only turn.
            if !self.pending_turn_events.is_empty() {
                self.pending_turn_events.push(event.clone());
            }
            self.flush_turn();
            self.history.winner = winner;
        }

        Some(event)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn two_player_game() -> Game {
        let players = vec![Player::new("Alice"), Player::new("Bob")];
        Game::new(GameVariant::Standard, players).unwrap()
    }

    #[test]
    fn test_game_new_valid_player_count() {
        let game = two_player_game();
        assert_eq!(game.current_player(), 0);
        assert_eq!(*game.phase(), GamePhase::WaitingForAsk);
        assert!(!game.is_over());
    }

    #[test]
    fn test_game_new_not_enough_players() {
        let result = Game::new(GameVariant::Standard, vec![Player::new("Solo")]);
        assert!(matches!(result, Err(GfError::NotEnoughPlayers)));
    }

    #[test]
    fn test_game_new_too_many_players() {
        let players: Vec<Player> = (0..9).map(|i| Player::new(format!("P{i}"))).collect();
        let result = Game::new(GameVariant::Standard, players);
        assert!(matches!(result, Err(GfError::TooManyPlayers)));
    }

    #[test]
    fn test_game_state_snapshot() {
        let game = two_player_game();
        let state = game.state().unwrap();
        assert_eq!(state.phase, GamePhase::WaitingForAsk);
        assert_eq!(state.current_player, 0);
        assert_eq!(state.players.len(), 2);
        assert!(state.last_event.is_none());
        assert!(state.winner.is_none());
        assert!(state.ask_log.is_empty());
    }

    #[test]
    fn test_game_act_draw_when_waiting_for_ask_is_error() {
        let mut game = two_player_game();
        let result = game.act(PlayerAction::Draw);
        assert_eq!(result, Err(GfError::OutOfTurn));
    }

    #[test]
    fn test_game_already_over_error() {
        let mut game = two_player_game();
        // Force game-over phase manually.
        game.phase = GamePhase::GameOver;
        let result = game.act(PlayerAction::Draw);
        assert_eq!(result, Err(GfError::GameAlreadyOver));
    }

    #[test]
    fn test_game_invalid_target_self() {
        let mut game = two_player_game();
        let rank = game.players[0].held_ranks().into_iter().next().unwrap();
        let result = game.act(PlayerAction::Ask { target: 0, rank });
        assert_eq!(result, Err(GfError::InvalidTarget));
    }

    #[test]
    fn test_game_invalid_target_out_of_range() {
        let mut game = two_player_game();
        let rank = game.players[0].held_ranks().into_iter().next().unwrap();
        let result = game.act(PlayerAction::Ask { target: 99, rank });
        assert_eq!(result, Err(GfError::InvalidTarget));
    }

    #[test]
    fn test_game_invalid_ask_player_lacks_rank() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        // Force player 0's hand empty so any ask is invalid, then give them one
        // card whose rank they definitely don't have 2 copies of.  Instead we
        // simply pick a rank player 0 doesn't hold by scanning the held ranks.
        let held: std::collections::HashSet<cardpack::prelude::Pip> =
            game.players[0].held_ranks().into_iter().collect();

        // Find a rank not in player 0's hand.  We know Standard52 has 13 ranks.
        // The initial deal of 7 may not cover all 13, so this should be safe.
        let all_ranks = [
            FrenchBasicCard::ACE_SPADES.rank,
            FrenchBasicCard::KING_SPADES.rank,
            FrenchBasicCard::QUEEN_SPADES.rank,
            FrenchBasicCard::JACK_SPADES.rank,
            FrenchBasicCard::TEN_SPADES.rank,
            FrenchBasicCard::NINE_SPADES.rank,
            FrenchBasicCard::EIGHT_SPADES.rank,
            FrenchBasicCard::SEVEN_SPADES.rank,
            FrenchBasicCard::SIX_SPADES.rank,
            FrenchBasicCard::FIVE_SPADES.rank,
            FrenchBasicCard::FOUR_SPADES.rank,
            FrenchBasicCard::TREY_SPADES.rank,
            FrenchBasicCard::DEUCE_SPADES.rank,
        ];

        if let Some(&rank) = all_ranks.iter().find(|r| !held.contains(r)) {
            let result = game.act(PlayerAction::Ask { target: 1, rank });
            assert_eq!(result, Err(GfError::InvalidAsk));
        }
        // If all 13 ranks happen to be in player 0's hand (impossible with 7
        // cards), this test is vacuously satisfied.
    }

    #[test]
    fn test_game_ask_when_waiting_for_draw_is_error() {
        let mut game = two_player_game();
        game.phase = GamePhase::WaitingForDraw;
        let rank = game.players[0].held_ranks().into_iter().next().unwrap();
        let result = game.act(PlayerAction::Ask { target: 1, rank });
        assert_eq!(result, Err(GfError::OutOfTurn));
    }

    #[test]
    fn test_game_initial_hands_dealt() {
        let game = two_player_game();
        // Standard rules deal 7 cards each to 2 players.
        // After books are collected from initial deal (if any), count may be lower.
        // Either way both hands should be non-empty after deal.
        // (The initial deal with a fully random deck may have produced a book;
        //  that's fine — we just check the pile shrank by at least 14 cards.)
        assert!(game.draw_pile.len() <= 52 - 14);
    }

    #[test]
    fn test_game_is_over_false_initially() {
        let game = two_player_game();
        assert!(!game.is_over());
    }

    #[test]
    fn test_game_draw_pile_reduced_after_deal() {
        let game = two_player_game();
        // 52 - 7*2 = 38 cards left (minus any book cards removed from hand)
        assert!(game.draw_pile.len() <= 38);
    }

    fn clear_hand(player: &mut Player) {
        for rank in player.held_ranks() {
            player.give_cards_of_rank(&rank);
        }
    }

    fn three_player_game() -> Game {
        let players: Vec<Player> = (0..3).map(|i| Player::new(format!("P{i}"))).collect();
        Game::new(GameVariant::Standard, players).unwrap()
    }

    // --- handle_ask: book produced from complete ask (line 570) ---

    #[test]
    fn test_handle_ask_produces_book_when_four_of_a_rank() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        // Player 0 holds 3 Aces; player 1 holds 1 Ace + 1 King.
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[0].receive_card(FrenchBasicCard::ACE_HEARTS);
        game.players[0].receive_card(FrenchBasicCard::ACE_DIAMONDS);
        game.players[1].receive_card(FrenchBasicCard::ACE_CLUBS);
        game.players[1].receive_card(FrenchBasicCard::KING_SPADES);

        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        let result = game
            .act(PlayerAction::Ask {
                target: 1,
                rank: ace_rank,
            })
            .unwrap();
        assert!(
            matches!(result, GameEvent::Book { .. }),
            "expected Book event, got {result:?}"
        );
        assert_eq!(game.phase, GamePhase::BookCompleted);
    }

    // --- handle_draw: matched flag (line 637) ---

    #[test]
    fn test_handle_draw_sets_matched_true_when_rank_matches() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        game.phase = GamePhase::WaitingForDraw;
        game.last_asked_rank = Some(FrenchBasicCard::ACE_CLUBS.rank);
        game.draw_pile = BasicPile::from(vec![
            FrenchBasicCard::ACE_CLUBS,
            FrenchBasicCard::KING_SPADES,
        ]);
        let result = game.act(PlayerAction::Draw).unwrap();
        assert!(
            matches!(result, GameEvent::Drew { matched: true, .. }),
            "expected matched=true, got {result:?}"
        );
    }

    #[test]
    fn test_handle_draw_sets_matched_false_when_rank_differs() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        // Clear random initial hands and set known state so that:
        //   - drawing KING_SPADES cannot complete a book (no Kings in hand), and
        //   - both players share an Ace rank (prevents a deadlock-triggered GameOver
        //     when the pile becomes empty after the draw).
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::ACE_HEARTS);
        game.phase = GamePhase::WaitingForDraw;
        game.last_asked_rank = Some(FrenchBasicCard::ACE_CLUBS.rank);
        game.draw_pile = BasicPile::from(vec![FrenchBasicCard::KING_SPADES]);
        let result = game.act(PlayerAction::Draw).unwrap();
        assert!(
            matches!(result, GameEvent::Drew { matched: false, .. }),
            "expected matched=false, got {result:?}"
        );
    }

    #[test]
    fn test_handle_draw_forms_book_on_non_matching_draw() {
        // Regression test for gf-history-1778474562.yaml: a non-matching draw
        // can still complete a book of the drawn card's rank. Player 0 holds
        // three 4s plus a 9, asks for 9 (Go Fish), draws the fourth 4. The
        // book of 4s must form even though matched=false; the turn still
        // advances to player 1 per the standard "no match ends turn" rule.
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        // P0: three 4s + one 9.
        game.players[0].receive_card(FrenchBasicCard::FOUR_SPADES);
        game.players[0].receive_card(FrenchBasicCard::FOUR_HEARTS);
        game.players[0].receive_card(FrenchBasicCard::FOUR_DIAMONDS);
        game.players[0].receive_card(FrenchBasicCard::NINE_SPADES);
        // P1: one Ace + one 9 (sharing the 9 rank with P0 keeps the game
        // out of deadlock-triggered GameOver after the draw).
        game.players[1].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::NINE_HEARTS);

        game.phase = GamePhase::WaitingForDraw;
        game.last_asked_rank = Some(FrenchBasicCard::NINE_SPADES.rank);
        // Pile has the fourth 4 on top, plus a non-book filler so the pile
        // is non-empty after the draw and the win-condition check is a no-op.
        game.draw_pile = BasicPile::from(vec![
            FrenchBasicCard::FOUR_CLUBS,
            FrenchBasicCard::KING_SPADES,
        ]);

        let result = game.act(PlayerAction::Draw).unwrap();

        match result {
            GameEvent::Book { player, ref rank } => {
                assert_eq!(player, 0, "book must be credited to player 0");
                assert_eq!(rank, "4", "book must be of rank 4");
            }
            other => panic!("expected Book event, got {other:?}"),
        }
        assert_eq!(
            game.players[0].book_count(),
            1,
            "player 0 should hold 1 book"
        );
        assert_eq!(
            game.current_player(),
            1,
            "turn must advance on non-matching draw even when a book forms"
        );
        assert_eq!(*game.phase(), GamePhase::WaitingForAsk);
    }

    // --- replenish_until_has_cards (lines 702-723) ---

    #[test]
    fn test_replenish_invalid_player_index_returns_false() {
        let mut game = two_player_game();
        assert!(!game.replenish_until_has_cards(99));
    }

    #[test]
    fn test_replenish_player_with_cards_returns_true_immediately() {
        let mut game = two_player_game();
        assert!(game.replenish_until_has_cards(0));
    }

    #[test]
    fn test_replenish_empty_hand_empty_pile_returns_false() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        game.draw_pile = BasicPile::default();
        assert!(!game.replenish_until_has_cards(0));
    }

    #[test]
    fn test_replenish_empty_hand_non_empty_pile_returns_true() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        assert!(!game.draw_pile.is_empty());
        let result = game.replenish_until_has_cards(0);
        assert!(result);
        assert!(!game.players[0].hand_is_empty());
    }

    // --- advance_turn (lines 751, 769) ---

    #[test]
    fn test_advance_turn_advances_to_next_player_with_cards() {
        let mut game = two_player_game();
        game.current_player = 0;
        game.advance_turn();
        assert_eq!(game.current_player, 1);
    }

    #[test]
    fn test_advance_turn_wraps_around_with_modulo() {
        // Tests the `(next + 1) % count` arithmetic at the loop body.
        // current=2 in a 3-player game; first candidate (player 1) has no cards
        // and pile is empty so the loop must advance via `(1 + 1) % 3 = 2`.
        let mut game = three_player_game();
        game.current_player = 0;
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();
        game.advance_turn();
        // Player 1 is skipped (empty hand, pile gone); player 2 still has cards.
        assert_eq!(game.current_player, 2);
    }

    #[test]
    fn test_advance_turn_initial_next_wraps_from_last_player() {
        // Verifies `(start + 1) % count` at line 747 wraps correctly.
        let mut game = three_player_game();
        game.current_player = 2;
        game.advance_turn();
        // next = (2 + 1) % 3 = 0; player 0 has cards.
        assert_eq!(game.current_player, 0);
    }

    #[test]
    fn test_advance_turn_skips_empty_handed_player_when_pile_empty() {
        // Verifies `!hand_is_empty()` check: without the `!` the player with an
        // empty hand would be chosen instead of skipped.
        let mut game = three_player_game();
        game.current_player = 0;
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();
        game.advance_turn();
        assert_eq!(game.current_player, 2);
        assert!(!game.players[2].hand_is_empty());
    }

    // --- ensure_current_player_can_ask (lines 791, 797) ---

    #[test]
    fn test_ensure_current_player_can_ask_no_op_in_waiting_for_draw_phase() {
        // The guard `phase != WaitingForAsk && phase != BookCompleted` must gate the whole function.
        let mut game = two_player_game();
        game.phase = GamePhase::WaitingForDraw;
        clear_hand(&mut game.players[0]);
        game.ensure_current_player_can_ask();
        // Wrong phase — function should return early without advancing or replenishing.
        assert_eq!(game.current_player, 0);
        assert!(game.players[0].hand_is_empty());
    }

    #[test]
    fn test_ensure_current_player_can_ask_replenishes_from_pile() {
        // Tests `if !replenish_until_has_cards(cp)` — if the `!` were removed,
        // a successful replenish would wrongly advance the turn.
        let mut game = two_player_game();
        game.phase = GamePhase::WaitingForAsk;
        clear_hand(&mut game.players[0]);
        assert!(!game.draw_pile.is_empty());
        game.ensure_current_player_can_ask();
        // Replenish succeeded — same player keeps turn and now has cards.
        assert_eq!(game.current_player, 0);
        assert!(!game.players[0].hand_is_empty());
    }

    #[test]
    fn test_ensure_current_player_can_ask_advances_when_pile_empty() {
        let mut game = two_player_game();
        game.phase = GamePhase::WaitingForAsk;
        clear_hand(&mut game.players[0]);
        game.draw_pile = BasicPile::default();
        game.ensure_current_player_can_ask();
        // Pile empty + hand empty → advance to next player.
        assert_eq!(game.current_player, 1);
    }

    // --- collect_books_for_player (line 854) ---

    #[test]
    fn test_collect_books_for_player_removes_complete_books() {
        use crate::rules::StandardRules;
        use cardpack::prelude::FrenchBasicCard;
        let mut player = Player::new("Alice");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        player.receive_card(FrenchBasicCard::ACE_HEARTS);
        player.receive_card(FrenchBasicCard::ACE_DIAMONDS);
        player.receive_card(FrenchBasicCard::ACE_CLUBS);
        player.receive_card(FrenchBasicCard::KING_SPADES);
        Game::collect_books_for_player(&mut player, &StandardRules);
        assert_eq!(player.book_count(), 1);
        assert_eq!(player.hand_size(), 1);
    }

    #[test]
    fn test_collect_books_for_player_no_op_for_mixed_hand() {
        use crate::rules::StandardRules;
        use cardpack::prelude::FrenchBasicCard;
        let mut player = Player::new("Bob");
        player.receive_card(FrenchBasicCard::ACE_SPADES);
        player.receive_card(FrenchBasicCard::KING_SPADES);
        player.receive_card(FrenchBasicCard::QUEEN_SPADES);
        Game::collect_books_for_player(&mut player, &StandardRules);
        assert_eq!(player.book_count(), 0);
        assert_eq!(player.hand_size(), 3);
    }

    // --- no_productive_ask_exists (line 877) ---

    #[test]
    fn test_no_productive_ask_exists_false_when_rank_overlap() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::ACE_HEARTS);
        assert!(!game.no_productive_ask_exists());
    }

    #[test]
    fn test_no_productive_ask_exists_true_when_no_overlap() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::KING_HEARTS);
        assert!(game.no_productive_ask_exists());
    }

    #[test]
    fn test_no_productive_ask_exists_true_when_all_hands_empty() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        assert!(game.no_productive_ask_exists());
    }

    // --- check_win_condition (lines 914-953) ---

    #[test]
    fn test_check_win_condition_returns_none_when_pile_not_empty() {
        let mut game = two_player_game();
        assert!(!game.draw_pile.is_empty());
        assert!(game.check_win_condition().is_none());
    }

    #[test]
    fn test_check_win_condition_returns_none_when_productive_ask_exists() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::ACE_HEARTS);
        game.draw_pile = BasicPile::default();
        // Pile empty but overlap exists → no game over.
        assert!(game.check_win_condition().is_none());
    }

    #[test]
    fn test_check_win_condition_game_over_when_all_hands_empty() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();
        let result = game.check_win_condition();
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), GameEvent::GameOver { .. }));
        assert_eq!(game.phase, GamePhase::GameOver);
    }

    #[test]
    fn test_check_win_condition_winner_has_most_books() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();
        game.players[1].add_book(BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]));
        let result = game.check_win_condition().unwrap();
        assert!(
            matches!(result, GameEvent::GameOver { winner: Some(1) }),
            "expected winner=Some(1), got {result:?}"
        );
    }

    #[test]
    fn test_check_win_condition_tie_gives_no_winner() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();
        let book = BasicPile::from(vec![
            FrenchBasicCard::ACE_SPADES,
            FrenchBasicCard::ACE_HEARTS,
            FrenchBasicCard::ACE_DIAMONDS,
            FrenchBasicCard::ACE_CLUBS,
        ]);
        game.players[0].add_book(book.clone());
        game.players[1].add_book(book);
        let result = game.check_win_condition().unwrap();
        assert!(
            matches!(result, GameEvent::GameOver { winner: None }),
            "expected winner=None on tie, got {result:?}"
        );
    }

    #[test]
    fn test_check_win_condition_deadlock_triggers_game_over() {
        use cardpack::prelude::FrenchBasicCard;
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.players[0].receive_card(FrenchBasicCard::ACE_SPADES);
        game.players[1].receive_card(FrenchBasicCard::KING_HEARTS);
        game.draw_pile = BasicPile::default();
        let result = game.check_win_condition();
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), GameEvent::GameOver { .. }));
    }

    // --- check_win_condition: history feature — pending_turn_events logic (line 954) ---

    /// Game ends mid-turn: GameOver must be appended to the existing pending
    /// turn before flush so the turn record is complete.
    #[cfg(feature = "history")]
    #[test]
    fn test_check_win_condition_gameover_appended_to_pending_turn() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();

        // Simulate mid-turn by injecting a sentinel event into the accumulator.
        game.pending_turn_events
            .push(GameEvent::GoFish { player: 0 });

        game.check_win_condition();

        let last_turn = game.history.turns.last().expect("expected a flushed turn");
        assert!(
            last_turn
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::GameOver { .. })),
            "GameOver must be in the last turn when pending events existed: {last_turn:?}"
        );
    }

    /// Game ends after advance_turn already flushed: no synthetic GameOver-only
    /// turn should be created.
    #[cfg(feature = "history")]
    #[test]
    fn test_check_win_condition_no_synthetic_turn_when_pending_empty() {
        let mut game = two_player_game();
        clear_hand(&mut game.players[0]);
        clear_hand(&mut game.players[1]);
        game.draw_pile = BasicPile::default();

        assert!(game.pending_turn_events.is_empty());
        let turns_before = game.history.turns.len();

        game.check_win_condition();

        assert_eq!(
            game.history.turns.len(),
            turns_before,
            "no synthetic GameOver-only turn should be added when pending events were empty"
        );
    }

    #[test]
    fn test_custom_is_valid_ask_honored_by_engine() {
        // Custom rule: every ask is valid regardless of hand contents.
        // Without the fix, the engine short-circuits with GfError::InvalidAsk
        // before consulting is_valid_ask. After the fix it must succeed.
        use crate::game::action::PlayerAction;
        use crate::player::Player;
        use crate::rules::{GameVariant, GoFishRules};
        use cardpack::prelude::{BasicPile, FrenchBasicCard, Pip};

        struct AnyAskValid;
        impl GoFishRules for AnyAskValid {
            fn name(&self) -> &'static str {
                "AnyAskValid"
            }
            fn deck(&self) -> BasicPile {
                // Returns a fixed, unshuffled pile. Game::new deals round-robin from index 0,
                // so the hand layout in comments above is deterministic.
                // Round-robin deal (hand_size=2):
                //   A: [ACE_SPADES, ACE_HEARTS]
                //   B: [KING_SPADES, KING_HEARTS]
                //   draw: [QUEEN_SPADES, QUEEN_HEARTS]
                BasicPile::from(vec![
                    FrenchBasicCard::ACE_SPADES,
                    FrenchBasicCard::KING_SPADES,
                    FrenchBasicCard::ACE_HEARTS,
                    FrenchBasicCard::KING_HEARTS,
                    FrenchBasicCard::QUEEN_SPADES,
                    FrenchBasicCard::QUEEN_HEARTS,
                ])
            }
            fn book_size(&self) -> usize {
                4
            }
            fn initial_hand_size(&self, _: usize) -> usize {
                2
            }
            fn min_players(&self) -> usize {
                2
            }
            fn max_players(&self) -> usize {
                4
            }
            fn is_valid_ask(&self, _hand: &BasicPile, _rank: &Pip) -> bool {
                true
            }
            fn is_book(&self, cards: &BasicPile) -> bool {
                if cards.len() != 4 {
                    return false;
                }
                let first = match cards.v().first() {
                    Some(c) => c.rank,
                    None => return false,
                };
                cards.v().iter().all(|c| c.rank == first)
            }
        }

        let players = vec![Player::new("A"), Player::new("B")];
        let mut game = Game::new(GameVariant::Custom(Box::new(AnyAskValid)), players).unwrap();

        // A holds [ACE_SPADES, ACE_HEARTS] — does NOT hold King rank.
        let king_rank = FrenchBasicCard::KING_SPADES.rank;
        let result = game.act(PlayerAction::Ask {
            target: 1,
            rank: king_rank,
        });
        assert!(
            result.is_ok(),
            "custom is_valid_ask (always true) must permit asking for unheld rank, got: {result:?}"
        );
    }

    #[test]
    fn test_custom_is_book_honored_by_engine() {
        // Custom rule: a pair of the same rank is a book.
        // Without the fix, engine counts cards.len() >= 4, so a pair is never
        // collected as a book. After the fix it must emit GameEvent::Book.
        use crate::game::action::PlayerAction;
        use crate::player::Player;
        use crate::rules::{GameVariant, GoFishRules};
        use cardpack::prelude::{BasicPile, FrenchBasicCard, Pip};

        struct PairIsBook;
        impl GoFishRules for PairIsBook {
            fn name(&self) -> &'static str {
                "PairIsBook"
            }
            fn deck(&self) -> BasicPile {
                // Round-robin deal (hand_size=2):
                //   iter1: A gets ACE_SPADES, B gets ACE_HEARTS
                //   iter2: A gets KING_SPADES, B gets KING_HEARTS
                //   draw: [QUEEN_SPADES, QUEEN_HEARTS]
                // A: [ACE_SPADES, KING_SPADES], B: [ACE_HEARTS, KING_HEARTS]
                // No startup books: each player has 1 ace + 1 king, is_book([1 card]) = false.
                BasicPile::from(vec![
                    FrenchBasicCard::ACE_SPADES,
                    FrenchBasicCard::ACE_HEARTS,
                    FrenchBasicCard::KING_SPADES,
                    FrenchBasicCard::KING_HEARTS,
                    FrenchBasicCard::QUEEN_SPADES,
                    FrenchBasicCard::QUEEN_HEARTS,
                ])
            }
            fn book_size(&self) -> usize {
                2
            }
            fn initial_hand_size(&self, _: usize) -> usize {
                2
            }
            fn min_players(&self) -> usize {
                2
            }
            fn max_players(&self) -> usize {
                4
            }
            fn is_valid_ask(&self, hand: &BasicPile, rank: &Pip) -> bool {
                hand.iter().any(|c| &c.rank == rank)
            }
            fn is_book(&self, cards: &BasicPile) -> bool {
                if cards.len() != 2 {
                    return false;
                }
                match (cards.v().first(), cards.v().get(1)) {
                    (Some(a), Some(b)) => a.rank == b.rank,
                    _ => false,
                }
            }
        }

        let players = vec![Player::new("A"), Player::new("B")];
        let mut game = Game::new(GameVariant::Custom(Box::new(PairIsBook)), players).unwrap();

        // A: [ACE_SPADES, KING_SPADES], B: [ACE_HEARTS, KING_HEARTS]
        // A asks B for ACE. B gives ACE_HEARTS. A now has [ACE_SPADES, ACE_HEARTS, KING_SPADES].
        // check_and_collect_book: cards_of_rank(ace) = [ACE_SPADES, ACE_HEARTS]
        // is_book([2 aces]) = true → Book collected.
        let ace_rank = FrenchBasicCard::ACE_SPADES.rank;
        let event = game
            .act(PlayerAction::Ask {
                target: 1,
                rank: ace_rank,
            })
            .unwrap();
        assert!(
            matches!(event, GameEvent::Book { .. }),
            "custom is_book (pair=book) must collect a 2-card book, got: {event:?}"
        );
    }
}
