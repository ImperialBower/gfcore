# Claude Instructions for gfcore

These instructions guide Claude to generate code that aligns with the ImperialBower project standards for testing, documentation, and code quality.

## Crate Overview

`gfcore` is the Go Fish card game engine crate. It is built on `cardpack` v0.7 card primitives and sits alongside `pkcore` (poker engine) in the ImperialBower org. It is a pure library crate — no binaries.

## Error Handling

- **Never use `unwrap()`, `expect()`, or `panic!()` in library code.**
- These are acceptable in `#[cfg(test)]` blocks and `examples/`, but not in production paths.
- All fallible operations must return `Result<T, GfError>`.
- Use the `?` operator for error propagation throughout library code.
- Add new variants to `GfError` (in `src/error.rs`) as the domain requires — keep `#[non_exhaustive]`.

## Clippy Lints

The crate root (`src/lib.rs`) carries these crate-level warnings — generated code must not introduce new violations:

```rust
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
```

Run `cargo clippy --all-features` before finishing any task and resolve all warnings.

## Documentation Requirements

### Every public function must have a doc test

```rust
/// Returns whether the requesting player's hand contains the asked-for rank.
///
/// # Errors
///
/// Returns [`GfError::InvalidPlayer`] if `player_id` is out of range.
///
/// # Examples
///
/// ```
/// use gfcore::prelude::*;
/// // example showing success case
/// ```
pub fn has_rank(player_id: usize, rank: u8) -> Result<bool, GfError> { ... }
```

- Doc tests must compile and pass under `cargo test --doc`.
- The `# Errors` section is required for every function returning `Result`.
- The `# Panics` section is required if the function can panic (which should be rare — see Error Handling above).

### Module-level docs

Every module file opens with a `//!` module doc comment explaining its purpose.

### Crate root (`src/lib.rs`)

The crate root has a complete crate-level overview doc comment (`//!`) with a Quick Start example.

## Naming Conventions

- `snake_case` for functions, variables, module names.
- `PascalCase` for types, structs, enums, traits.
- No single-letter variable names outside loop indices (`i`, `j`, `k`).
- Prefer full English words (`player` not `plyr`, `rank` not `rk`).

## File Organisation

Prefer **domain-grouped files** over one-type-per-file:

- `src/error/mod.rs` — all error types
- `src/rules/mod.rs` — rule variants and configuration structs
- `src/game/mod.rs` — game state machine, `Game` struct, actions, phases
- `src/player/mod.rs` — `Player`, hand management, scoring
- `src/bot/mod.rs` — bot strategies and the `BotStrategy` trait
- `src/history/mod.rs` — history recording (feature-gated on `history`)
- `src/prelude.rs` — public re-exports for ergonomic imports

Use subdirectory modules (`foo/mod.rs`) rather than flat files (`foo.rs`) for all modules; each subdirectory module will gain sub-files as the module grows.

## Testing

### Unit tests

- Placed in a `#[cfg(test)]` block at the bottom of the file being tested, **or** in `tests/<module>_tests.rs`.
- Every public struct/enum/function has at least one unit test covering the happy path.
- Test names follow: `test_<subject>_<scenario>` (e.g. `test_game_new_default_player_count`).
- Include edge cases, error conditions, and boundary conditions.

### Integration tests

- Live in `tests/` at the crate root.
- Cover multi-module interactions and feature-flag combinations.

### Running tests

```bash
cargo test                      # all tests
cargo test --doc                # doc tests only
cargo clippy --all-features     # lint check
cargo test -- --nocapture       # with stdout
```

## Trait Implementations

- Implement `Display` for any type shown to users.
- Implement `Debug` for all public types (derive if possible).
- Implement `Default` where there is a sensible zero-state.
- Implement `Clone` when callers may need snapshots of game state.

## Feature Gates

- `history` (default): enables `uuid` and `serde_norway`; guards `src/history/mod.rs`.
- `wasm`: enables `wasm-bindgen`, `console_error_panic_hook`, `getrandom/wasm_js`.

Code that requires a feature must be wrapped in `#[cfg(feature = "...")]`.

## Checklist Before Finishing Any Task

- [ ] All new public functions have doc comments with `# Examples` and `# Errors`.
- [ ] Doc tests compile and pass (`cargo test --doc`).
- [ ] Unit tests cover happy path, edge cases, and error conditions.
- [ ] No `unwrap()` / `expect()` / `panic!()` outside `#[cfg(test)]`.
- [ ] `cargo clippy --all-features` reports no warnings.
- [ ] `cargo test` passes.

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [cardpack docs](https://docs.rs/cardpack/0.7.0)
- [pkcore](https://github.com/ImperialBower/pkcore) — sibling poker engine, same standards
