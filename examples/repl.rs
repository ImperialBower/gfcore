//! Interactive Go Fish REPL — one human player vs three bot opponents.
//!
//! # Running
//!
//! ```sh
//! cargo run --example repl
//! ```
//!
//! On your turn the REPL shows your hand (ranks and counts), lists the other
//! players, and asks you to choose a target and a rank.  Bot turns run
//! automatically with a short pause so you can follow the action.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use cardpack::prelude::{BasicPile, Pip};
use gfcore::bot::BotProfile;
use gfcore::prelude::{Game, GameEvent, GamePhase, GameState, GameVariant, Player, PlayerAction};

fn main() {
    // Three bots from the default roster: Harriet (smart), Bertram (smart), Lucky (random).
    let mut all_profiles = BotProfile::default_profiles();
    let bots: Vec<BotProfile> = all_profiles.drain(..3).collect();

    let players = vec![
        Player::new("You"),
        Player::new_bot(bots[0].name.clone(), bots[0].clone()),
        Player::new_bot(bots[1].name.clone(), bots[1].clone()),
        Player::new_bot(bots[2].name.clone(), bots[2].clone()),
    ];

    // Keep profiles alongside the game for bot decision-making.
    let profiles: Vec<Option<BotProfile>> = vec![
        None,
        Some(bots[0].clone()),
        Some(bots[1].clone()),
        Some(bots[2].clone()),
    ];

    let names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();

    let mut game = Game::new(GameVariant::Standard, players).expect("valid 4-player game");

    println!("\n=== Go Fish ===");
    println!(
        "You vs {} (smart), {} (smart), {} (random)\n",
        names[1], names[2], names[3]
    );

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        if game.is_over() {
            break;
        }

        let state = game.state().expect("state");
        let cp = state.current_player;

        // Status bar — one line per player.
        println!();
        println!("  Draw pile: {} cards", state.draw_pile_size);
        for view in &state.players {
            let marker = if view.index == cp { ">" } else { " " };
            println!(
                "  {} {:<10}  {:>2} cards   {} books",
                marker, names[view.index], view.hand_size, view.books
            );
        }

        if cp == 0 {
            human_turn(&mut game, &state, &names, &mut lines);
        } else {
            bot_turn(&mut game, &state, &names, &profiles[cp]);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    // Final scoreboard.
    let final_state = game.state().expect("final state");
    println!("\n=== Final Scores ===");
    let mut scores: Vec<_> = final_state
        .players
        .iter()
        .map(|v| (names[v.index].as_str(), v.books))
        .collect();
    scores.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (rank, (name, books)) in scores.iter().enumerate() {
        let suffix = match rank {
            0 => "  <-- winner",
            _ => "",
        };
        println!("  {}. {:<12} {} books{}", rank + 1, name, books, suffix);
    }
    println!();
}

// ---------------------------------------------------------------------------
// Human turn
// ---------------------------------------------------------------------------

fn human_turn(
    game: &mut Game,
    state: &GameState,
    names: &[String],
    lines: &mut impl Iterator<Item = io::Result<String>>,
) {
    let hand = state.players[0].hand.as_ref().expect("human hand visible");
    let groups = rank_groups(hand);

    let hand_str = groups
        .iter()
        .map(|(ch, n)| format!("{ch}({n})"))
        .collect::<Vec<_>>()
        .join("  ");
    println!("\n  Your hand:  {hand_str}");

    if state.phase == GamePhase::WaitingForDraw {
        println!("  Go Fish!  Press Enter to draw.");
        print!("  > ");
        let _ = io::stdout().flush();
        let _ = lines.next();
        let event = game.act(PlayerAction::Draw).expect("draw");
        print_event(&event, names);
        return;
    }

    // Choose who to ask.
    println!("  Who do you want to ask?");
    for view in state.players.iter().filter(|v| v.index != 0) {
        println!(
            "    [{}] {:<10}  {} cards  {} books",
            view.index, names[view.index], view.hand_size, view.books
        );
    }
    let target = read_usize(lines, "  Target: ", 1, names.len() - 1);

    // Choose which rank to ask for.
    let valid: Vec<char> = groups.iter().map(|(ch, _)| *ch).collect();
    let valid_str = valid
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("/");
    println!("  Rank? [{valid_str}]");
    let rank = read_rank(lines, hand);

    let event = game.act(PlayerAction::Ask { target, rank }).expect("ask");
    print_event(&event, names);
}

// ---------------------------------------------------------------------------
// Bot turn
// ---------------------------------------------------------------------------

fn bot_turn(game: &mut Game, state: &GameState, names: &[String], profile: &Option<BotProfile>) {
    let cp = state.current_player;
    let profile = profile.as_ref().expect("bot has a profile");

    if state.phase == GamePhase::WaitingForDraw {
        let event = game.act(PlayerAction::Draw).expect("bot draw");
        print_event(&event, names);
        return;
    }

    let hand = state.players[cp]
        .hand
        .as_ref()
        .expect("bot hand visible on their turn")
        .clone();

    let action = profile.decide(&hand, &state.players, &state.ask_log);

    if let PlayerAction::Ask { target, rank } = &action {
        println!(
            "\n  {} asks {} for {}s.",
            names[cp], names[*target], rank.index
        );
    }

    let event = game.act(action).expect("bot action");
    print_event(&event, names);
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

fn print_event(event: &GameEvent, names: &[String]) {
    match event {
        GameEvent::GoFish { player } => {
            println!("  -> Go Fish!  {} must draw.", names[*player]);
        }
        GameEvent::Gave {
            from,
            to,
            rank,
            count,
        } => {
            let s = if *count == 1 { "" } else { "s" };
            println!(
                "  -> {} gives {} {} {rank}{s}.",
                names[*from], names[*to], count
            );
        }
        GameEvent::Drew { player, matched } => {
            if *matched {
                println!("  -> {} drew a match!  Gets to ask again.", names[*player]);
            } else {
                println!(
                    "  -> {} drew (no match).  Next player's turn.",
                    names[*player]
                );
            }
        }
        GameEvent::Book { player, rank } => {
            println!(
                "  -> {} completed a book of {rank}s!  Gets another turn.",
                names[*player]
            );
        }
        GameEvent::GameOver { winner } => match winner {
            Some(w) => println!("\n  {} wins the game!", names[*w]),
            None => println!("\n  It's a tie!"),
        },
        GameEvent::Asked { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

/// Returns cards in the hand grouped by rank, sorted by weight descending
/// (high ranks first: A, K, Q, J, T, 9, …).
fn rank_groups(hand: &BasicPile) -> Vec<(char, usize)> {
    let mut map: HashMap<char, (usize, usize)> = HashMap::new(); // index → (weight, count)
    for card in hand {
        let entry = map.entry(card.rank.index).or_insert((card.rank.weight, 0));
        entry.1 += 1;
    }
    let mut groups: Vec<(char, usize, usize)> = map
        .into_iter()
        .map(|(ch, (weight, count))| (ch, weight, count))
        .collect();
    groups.sort_by_key(|b| std::cmp::Reverse(b.1));
    groups
        .into_iter()
        .map(|(ch, _, count)| (ch, count))
        .collect()
}

fn read_usize(
    lines: &mut impl Iterator<Item = io::Result<String>>,
    prompt: &str,
    min: usize,
    max: usize,
) -> usize {
    loop {
        print!("{prompt}");
        let _ = io::stdout().flush();
        let line = lines.next().expect("stdin is open").expect("valid UTF-8");
        if let Ok(n) = line.trim().parse::<usize>() {
            if n >= min && n <= max {
                return n;
            }
        }
        println!("  Please enter a number from {min} to {max}.");
    }
}

fn read_rank(lines: &mut impl Iterator<Item = io::Result<String>>, hand: &BasicPile) -> Pip {
    loop {
        print!("  Rank: ");
        let _ = io::stdout().flush();
        let line = lines.next().expect("stdin is open").expect("valid UTF-8");
        let ch = line.trim().to_uppercase().chars().next().unwrap_or('\0');
        if let Some(card) = hand
            .iter()
            .find(|c| c.rank.index.to_ascii_uppercase() == ch)
        {
            return card.rank;
        }
        println!("  You must ask for a rank you already hold.");
    }
}
