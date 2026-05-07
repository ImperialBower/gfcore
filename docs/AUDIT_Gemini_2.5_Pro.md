# gfcore audit / code review

_Date:_ 2026-05-07
_Auditor:_ Gemini 2.5 Pro

## Executive summary

`gfcore` is a well-structured and well-documented crate. The code quality is high, and the test coverage is good. However, I have identified a few issues that need to be addressed.

The most important issues I found are:

1.  **The public `Custom` rules API is not fully implemented.** The engine does not consult `GoFishRules::is_valid_ask` or `GoFishRules::is_book`, which can lead to incorrect behavior for custom game variants.
2.  **The wasm test workflow is broken.** The documented command for running wasm tests fails due to a missing configuration file, and there is no CI job to enforce wasm test execution.
3.  **There are some documentation and API drift issues.** The documentation for `get_game_yaml()` is incorrect, and the version example in the wasm API is stale.

## Scope reviewed

I reviewed the following areas:

-   crate metadata and feature model in `Cargo.toml`
-   public API surface in `src/lib.rs` and `src/prelude.rs`
-   core engine in `src/game/mod.rs`, `src/game/action.rs`, and `src/game/state.rs`
-   rules system in `src/rules/`
-   wasm bindings in `src/wasm_api.rs`
-   CI / automation in `.github/workflows/CI.yaml`
-   integration coverage in `tests/`

## Strengths

### 1. Documentation and testing

The crate has excellent documentation and test coverage. Public APIs are well-documented with examples and doc tests. The unit and integration tests cover a wide range of scenarios, including edge cases.

### 2. Code quality

The code is clean, well-organized, and easy to understand. The error handling is robust, and the use of `clippy` ensures a high level of code quality.

### 3. Feature gating

The `history` and `wasm` features are well-implemented and allow for a flexible and modular design.

## Findings

| Severity | Confidence | Area                                       | Summary                                                                                             |
| :------- | :--------- | :----------------------------------------- | :-------------------------------------------------------------------------------------------------- |
| High     | High       | `src/rules/mod.rs`, `src/game/state.rs`    | `GameVariant::Custom` is not truly custom: engine ignores `is_valid_ask()` and `is_book()`          |
| Medium   | High       | `tests/wasm.rs`, repo config, CI           | Documented wasm test command fails as-is because the runner is not configured in-repo               |
| Low      | High       | `src/wasm_api.rs`                          | `get_game_yaml()` docs do not match behavior                                                        |
| Low      | High       | `src/wasm_api.rs`                          | Example output for `version()` is stale (`0.1.0` vs package `0.0.1`)                                |
| Low      | High       | `src/error/mod.rs`, `src/game/state.rs`    | `GfError::EmptyDrawPile` appears to be dead public API                                              |

---

## Detailed findings

### Finding 1 — `GameVariant::Custom` is not truly custom

**Severity:** High
**Confidence:** High

The public API suggests that `GameVariant::Custom` allows for a fully custom ruleset by implementing the `GoFishRules` trait. However, the game engine in `src/game/state.rs` does not use the `is_valid_ask()` and `is_book()` methods from this trait. Instead, it uses its own hard-coded logic for validating asks and detecting books.

This means that any custom game variant that relies on these methods will not behave as expected.

**Recommendation:**

Either update the game engine to use the `is_valid_ask()` and `is_book()` methods from the `GoFishRules` trait, or update the documentation to clarify the limitations of `GameVariant::Custom`.

### Finding 2 — wasm test workflow is broken as documented

**Severity:** Medium
**Confidence:** High

The `tests/wasm.rs` file provides instructions for running the wasm tests, but the command fails because the `.cargo/config.toml` file that is supposed to configure the wasm runner is missing from the repository.

Additionally, the CI configuration in `.github/workflows/CI.yaml` does not include a job for running the wasm tests. This means that wasm-related regressions can go undetected.

**Recommendation:**

-   Add the `.cargo/config.toml` file to the repository with the wasm runner configuration.
-   Add a new job to the CI workflow to run the wasm tests.

### Finding 3 — `get_game_yaml()` docs do not match behavior

**Severity:** Low
**Confidence:** High

The documentation for `get_game_yaml()` in `src/wasm_api.rs` states that the function returns an error if the game is not yet over. However, the implementation returns the history for in-progress games as well.

**Recommendation:**

Update the documentation to match the implementation, or update the implementation to match the documentation.

### Finding 4 — stale version example in wasm docs

**Severity:** Low
**Confidence:** High

The example for the `version()` function in `src/wasm_api.rs` shows "0.1.0" as the output, but the `Cargo.toml` file specifies the version as "0.0.1".

**Recommendation:**

Update the example to show the correct version number.

### Finding 5 — `GfError::EmptyDrawPile` looks unused

**Severity:** Low
**Confidence:** High

The `GfError::EmptyDrawPile` variant is defined in `src/error/mod.rs` but it is never used in the game engine. The engine handles an empty draw pile by emitting a `GameEvent::Drew { matched: false }` event.

**Recommendation:**

Remove the `GfError::EmptyDrawPile` variant if it is not needed, or update the engine to use it.

