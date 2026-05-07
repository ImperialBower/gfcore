# AI Bill of Materials — gfcore

This document discloses the AI tools used during the development of `gfcore` and describes the scope of their involvement and the human oversight applied.

---

## Tools Used

| Tool | Provider | Interface | Model |
|------|----------|-----------|-------|
| Claude Code | Anthropic | CLI / claude.ai/code | Claude Sonnet 4.6 |

---

## Scope of AI Assistance

### Substantially AI-assisted
- **Rust source code** (`src/`) — game engine logic, data structures, trait implementations, error types, WASM bindings
- **Test suite** — unit tests, integration tests, doc tests, mutation-testing gap closure
- **CI/toolchain configuration** — `deny.toml` (license and dependency auditing), Miri flags, GitHub Actions workflows
- **Pre-publish cleanup** — `Cargo.toml` metadata, `exclude` patterns, doc comment quality

### Human-driven
- Project vision and game selection (Go Fish)
- Domain decisions: rule variants (Standard, Happy Families, Quartet), public API shape, feature-flag boundaries
- Architecture: module layout, `history` and `wasm` feature gate design
- All code review — every AI-generated change was reviewed and accepted or rejected by the human author before commit
- Git workflow: branching, commit messages, release decisions

---

## Human Oversight

All AI output was reviewed interactively via the Claude Code CLI before being staged or committed. The human author retained final decision authority on all design choices, public API surfaces, and accepted changes. No AI-generated code was merged without review.

---

## Notes

- The `wasm_api.rs` module and associated WASM test harness were AI-assisted; correctness was verified by the human author against the target JavaScript interface.
- The `cardpack` dependency (card primitives) is a pre-existing human-authored library; AI assistance was limited to its consumption, not its implementation.
- AI tools were used from project inception (2026-05-05) through the 0.1.0 release preparation.
