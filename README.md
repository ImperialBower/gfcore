[![Build and Test](https://github.com/ImperialBower/gfcore/actions/workflows/CI.yaml/badge.svg)](https://github.com/ImperialBower/gfcore/actions/workflows/CI.yaml)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE-APACHE)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![Crates.io Version](https://img.shields.io/crates/v/gfcore.svg)](https://crates.io/crates/gfcore)
[![Rustdocs](https://docs.rs/gfcore/badge.svg)](https://docs.rs/gfcore/)
[![AI-BOM](https://img.shields.io/badge/AI--BOM-Claude%20Sonnet%204.6-blueviolet)](AI-BOM.md)

# gfcore

Go Fish card game engine for the ImperialBower project ecosystem.

Quick start — create a game, take a turn, inspect state:

```rust
use gfcore::prelude::*;

let players = vec![
    Player::new("Alice"),
    Player::new("Bob"),
];
let mut game = Game::new(GameVariant::Standard, players)?;

// Ask player 1 for Aces
let event = game.act(PlayerAction::Ask { target: 1, rank: /* Ace pip */ })?;
```

See `cargo doc --open` for the full API reference.

### Interactive REPL

Play a quick game against three bots in your terminal:

```sh
cargo run --example repl
```

```
=== Go Fish ===
You vs Harriet (smart), Bertram (smart), Lucky (random)


  Draw pile: 24 cards
  > You          7 cards   0 books
    Harriet      7 cards   0 books
    Bertram      7 cards   0 books
    Lucky        7 cards   0 books

  Your hand:  K♦  Q♠  J♥  T♣  8♥ 8♦  6♠
  Who do you want to ask?
    [1] Harriet     7 cards  0 books
    [2] Bertram     7 cards  0 books
    [3] Lucky       7 cards  0 books
  Target: 1
  Rank? [K/Q/J/T/8/6]
  Rank: 8
  -> Harriet gives You 1 8.
```

## Backstory

This is an experiment in using AI code generation to create a very simple mirror to my 
[pkcore](https://github.com/ImperialBower/pkcore) poker library for a different card game. 