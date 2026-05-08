# gfcore audit / code review

_Date:_ 2026-05-07  
_Auditor:_ GPT-5.4

## Executive summary

`gfcore` is in solid shape overall. The crate has unusually strong API documentation coverage, a large amount of unit and integration testing for its size, clear module boundaries, and clean host-side quality gates.

What I verified directly:

- `cargo test --all-features` ✅
- `cargo test --doc --all-features` ✅
- `cargo clippy --all-features -- -D warnings` ✅
- `cargo deny check advisories` ✅

I did **not** find a host-side correctness failure in the core game engine during this review.

The most important issues I found are:

1. **The public `Custom` rules API is materially over-promised by the current engine implementation.** The engine does not consult `GoFishRules::is_valid_ask` or `GoFishRules::is_book`, so custom variants can silently behave incorrectly.
2. **The documented wasm test workflow is broken in-repo.** The repo claims `.cargo/config.toml` configures the wasm runner, but that file is absent, and the documented `cargo test --target wasm32-unknown-unknown --test wasm --features wasm` command fails as-is.
3. **There are a few doc/API drift issues** that will confuse consumers, especially around `get_game_yaml()` and a stale version example.

## Scope reviewed

I reviewed the following areas:

- crate metadata and feature model in `Cargo.toml`
- public API surface in `src/lib.rs` and `src/prelude.rs`
- core engine in `src/game/mod.rs`, `src/game/action.rs`, and `src/game/state.rs`
- player model in `src/player/`
- rules system in `src/rules/`
- bot layer in `src/bot/`
- history serialization in `src/history/`
- wasm bindings in `src/wasm_api.rs`
- CI / automation in `.github/workflows/CI.yaml`, `.github/workflows/audit.yml`, and `Makefile`
- integration coverage in `tests/`

## Strengths

### 1. Documentation discipline is excellent

The crate consistently documents public APIs with examples and doc tests. This is rare in early-stage libraries and materially improves maintainability and usability.

### 2. The game engine has strong invariant-focused testing

`src/game/state.rs` includes extensive unit tests targeting edge cases, and the integration tests in `tests/game_integration.rs` and `tests/history_integration.rs` go beyond trivial happy-path checks.

### 3. Error handling is conservative and mostly library-friendly

The library code avoids `unwrap()`/`expect()` in production paths and keeps fallible behavior behind `Result<T, GfError>` where appropriate.

### 4. Feature gating is conceptually clean

The `history` and `wasm` features are separated sensibly, and the host-side `--all-features` build is clean.

## Findings

| Severity | Confidence | Area | Summary |
|---|---:|---|---|
| High | High | `src/rules/mod.rs`, `src/game/state.rs` | `GameVariant::Custom` over-promises extensibility: engine ignores `is_valid_ask()` and `is_book()` |
| Medium | High | `tests/wasm.rs`, repo config, CI | Documented wasm test command fails as-is because the runner is not configured in-repo |
| Low | High | `src/wasm_api.rs` | `get_game_yaml()` docs say the game must be over, but implementation returns in-progress history too |
| Low | High | `src/error/mod.rs`, `src/game/state.rs` | `GfError::EmptyDrawPile` appears to be dead public API |
| Low | Medium | `src/wasm_api.rs` | Example output for `version()` is stale (`0.1.0` vs package `0.0.1`) |

---

## Detailed findings

### Finding 1 — `GameVariant::Custom` is not truly custom

**Severity:** High  
**Confidence:** High

#### Why this matters

The public API presents `GameVariant::Custom(Box<dyn GoFishRules + Send + Sync>)` as a way to supply a fully custom ruleset. In practice, the engine only uses a subset of the trait:

- `name()`
- `deck()`
- `book_size()`
- `initial_hand_size()`
- `min_players()`
- `max_players()`

The actual move validation and book detection logic are hard-coded in the engine.

#### Evidence

- `src/rules/mod.rs` documents `GameVariant::Custom` as:
  - “A fully custom variant supplied by the caller.”
- `src/game/state.rs` validates asks with direct hand inspection in `handle_ask()` rather than `rules.is_valid_ask(...)`.
- `src/game/state.rs` detects books via `book_size` + same-rank counting in `check_and_collect_book()` / `collect_books_for_player()` rather than `rules.is_book(...)`.
- A search of the repo shows `is_valid_ask()` and `is_book()` are effectively defined and tested in the rules layer, but never consulted by the engine.

#### Impact

Any caller implementing a non-standard ruleset can get silently incorrect game behavior even though the public type system suggests otherwise.

Examples of unsupported-but-advertised customization:

- variants where asking rules differ from “you must already hold the rank”
- variants where a book is not strictly “N cards of the same rank”
- variants with family semantics that are not rank-based

#### Recommendation

Choose one of these directions explicitly:

1. **Narrow the contract**: document that custom rules only control deck setup, hand sizes, player bounds, and book size.
2. **Honor the trait fully**: route ask validation and book validation through `GoFishRules` from the engine.

Until then, this is the most important API-level correctness risk in the crate.

---

### Finding 2 — wasm test workflow is broken as documented

**Severity:** Medium  
**Confidence:** High

#### Why this matters

The repo includes a wasm runtime test suite and documentation that suggests the tests are runnable via a normal cargo command. In its current state, that command fails in this repository without extra environment setup.

#### Evidence

- `tests/wasm.rs` says:
  - `cargo test --target wasm32-unknown-unknown --test wasm --features wasm`
  - and claims `.cargo/config.toml` sets the runner to `wasm-bindgen-test-runner`
- There is **no** `.cargo/config.toml` in the repository.
- Running the documented command in this repo produced:
  - `cannot execute binary file`
- Re-running with an explicit runner env var moved execution forward, confirming the first failure is configuration-related.
- `.github/workflows/CI.yaml` does not include a wasm job, so this path is not currently enforced in CI.

#### Impact

- contributors following the test docs will hit a broken command
- wasm regressions can slip through because CI does not exercise the runtime path
- the current test suite also emitted a wasm-target warning (`unused_must_use`) that host-side clippy does not catch

#### Recommendation

- Add a real `.cargo/config.toml` with the wasm runner, **or** remove the claim from docs and update commands to use `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`
- Add a CI job that runs wasm tests
- Consider documenting/pinning the expected `wasm-bindgen-cli` version to reduce schema mismatch churn
- Fix the `unused_must_use` warning in `tests/wasm.rs`

#### Note

A second wasm run failed due a local `wasm-bindgen-test-runner` / crate schema version mismatch (`0.2.120` vs `0.2.121`). I am **not** counting that as a repository defect because it is environment/version skew, not an in-repo code problem.

---

### Finding 3 — `get_game_yaml()` docs do not match behavior

**Severity:** Low  
**Confidence:** High

#### Evidence

In `src/wasm_api.rs`, the docs for `get_game_yaml()` say it returns an error if:

- no game is in progress
- the game is not yet over
- the `history` feature is not enabled

But the implementation only checks:

- whether a game exists
- whether `history` is enabled

It directly serializes `game.record()` and therefore returns partial history for an in-progress game.

#### Impact

JavaScript consumers may build logic around a stricter contract than the API actually enforces.

#### Recommendation

Either:

- update the docs to say in-progress history is allowed, or
- enforce `game.is_over()` before returning YAML

---

### Finding 4 — `GfError::EmptyDrawPile` looks unused and semantically stale

**Severity:** Low  
**Confidence:** High

#### Evidence

- `GfError::EmptyDrawPile` is defined and documented in `src/error/mod.rs`
- a repo search found no production usage of that variant
- `src/game/state.rs` handles draw-on-empty-pile by emitting `GameEvent::Drew { matched: false }` and advancing play rather than returning an error

#### Impact

This creates ambiguity in the public error model: consumers see an error variant that the engine does not actually produce.

#### Recommendation

Either:

- remove the variant if it is not part of the intended contract, or
- use it in the engine, or
- explicitly document that it is reserved for future or external parser-facing use

---

### Finding 5 — stale version example in wasm docs

**Severity:** Low  
**Confidence:** Medium

#### Evidence

In `src/wasm_api.rs`, the `version()` example shows:

```javascript
console.log(version()); // "0.1.0"
```

But `Cargo.toml` currently declares version `0.0.1`.

#### Recommendation

Update the example to avoid stale literal output, e.g. “returns the crate version string” without pinning a specific number.

---

## Remediation Verification

_Date:_ 2026-05-07  
_Auditor:_ GitHub Copilot

A review of the codebase was conducted to verify the remediation of the findings from the original audit.

| Finding                                      | Status      | Notes                                                                                                                                                                                          |
| :------------------------------------------- | :---------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. `GameVariant::Custom` not truly custom    | **FIXED**   | The game engine in `src/game/state.rs` now correctly calls `rules.is_valid_ask()` and `rules.is_book()`, allowing custom variants to function as expected. New integration tests confirm this behavior. |
| 2. Wasm test workflow broken                 | **NOT FIXED** | The `.cargo/config.toml` file required to configure the wasm test runner is still missing from the repository, and the CI workflow has not been updated to run wasm tests.                     |
| 3. `get_game_yaml()` docs/behavior mismatch  | **FIXED**   | The documentation for `get_game_yaml()` in `src/wasm_api.rs` has been corrected to state that it returns history for in-progress games, matching the implementation.                             |
| 4. `GfError::EmptyDrawPile` unused           | **FIXED**   | The `GfError::EmptyDrawPile` variant has been removed from `src/error/mod.rs`, eliminating the dead public API.                                                                                |
| 5. Stale version example in wasm docs        | **FIXED**   | The example output for the `version()` function in `src/wasm_api.rs` has been updated to `0.0.1` to match the version in `Cargo.toml`.                                                          |

## Coverage / process observations

### What passed cleanly

These checks passed during the audit:

```bash
cargo test --all-features
cargo test --doc --all-features
cargo clippy --all-features -- -D warnings
cargo deny check advisories
```

### What failed

This command failed as documented in the repo because no wasm runner is configured in-repo:

```bash
cargo test --target wasm32-unknown-unknown --test wasm --features wasm
```

### CI coverage gap

Current CI does **not** appear to run:

- wasm runtime tests
- `--all-features` clippy
- a dedicated wasm target build/test job

That gap is important because the repository has a meaningful feature-gated surface in `src/wasm_api.rs`.

## Recommended priority order

1. **Fix or narrow `GameVariant::Custom`** so the public contract matches reality.
2. **Repair wasm test ergonomics and CI coverage**.
3. **Align `get_game_yaml()` docs with implementation**.
4. **Remove or justify `GfError::EmptyDrawPile`**.
5. **Clean up small doc drift items** like the stale version example.

## Bottom line

This is a promising crate with strong fundamentals: documentation, tests, and core engine quality are all above average for an early library. The main risk is not day-to-day engine stability, but **API contract drift**:

- one important case in `Custom` rules semantics
- one important case in wasm workflow/documentation
- a few smaller behavior-vs-doc mismatches

If those are addressed, the repo will be in notably stronger shape for external users and future feature growth.
