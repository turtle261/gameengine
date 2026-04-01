use std::env;

#[cfg(feature = "builtin")]
use gameengine::builtin::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::builtin::{Platformer, PlatformerAction};
#[cfg(feature = "builtin")]
use gameengine::{PlayerAction, Session, stable_hash};

#[cfg(feature = "builtin")]
fn run_tictactoe(iterations: u64) -> u64 {
    let mut session = Session::new(TicTacToe, 7);
    let script = [
        PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        },
        PlayerAction {
            player: 0,
            action: TicTacToeAction(4),
        },
        PlayerAction {
            player: 0,
            action: TicTacToeAction(8),
        },
        PlayerAction {
            player: 0,
            action: TicTacToeAction(2),
        },
    ];

    let mut digest = 0u64;
    for index in 0..iterations {
        if session.is_terminal() || session.current_tick() >= 200 {
            session.reset(7);
        }
        let action = &script[(index as usize) % script.len()];
        let outcome = session.step(std::slice::from_ref(action));
        digest = digest.wrapping_add(outcome.reward_for(0) as u64);
        digest ^= session.current_tick();
    }
    digest ^ stable_hash(session.trace())
}

#[cfg(feature = "builtin")]
fn run_blackjack(iterations: u64) -> u64 {
    let mut session = Session::new(Blackjack, 11);
    let script = [
        PlayerAction {
            player: 0,
            action: BlackjackAction::Hit,
        },
        PlayerAction {
            player: 0,
            action: BlackjackAction::Stand,
        },
    ];

    let mut digest = 0u64;
    for index in 0..iterations {
        if session.is_terminal() || session.current_tick() >= 200 {
            session.reset(11);
        }
        let action = &script[(index as usize) % script.len()];
        let outcome = session.step(std::slice::from_ref(action));
        digest = digest.wrapping_add(outcome.reward_for(0) as u64);
        digest ^= session.current_tick();
    }
    digest ^ stable_hash(session.trace())
}

#[cfg(feature = "physics")]
fn run_platformer(iterations: u64) -> u64 {
    let mut session = Session::new(Platformer::default(), 5);
    let script = [
        PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Left,
        },
    ];

    let mut digest = 0u64;
    for index in 0..iterations {
        if session.is_terminal() || session.current_tick() >= 200 {
            session.reset(5);
        }
        let action = &script[(index as usize) % script.len()];
        let outcome = session.step(std::slice::from_ref(action));
        digest = digest.wrapping_add(outcome.reward_for(0) as u64);
        digest ^= session.current_tick();
    }
    digest ^ stable_hash(session.trace())
}

#[cfg(feature = "builtin")]
fn main() {
    let mut args = env::args().skip(1);
    let game = args.next().unwrap_or_else(|| "platformer".to_string());
    let iterations = args
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(2_000_000);

    let digest = match game.as_str() {
        "tictactoe" => run_tictactoe(iterations),
        "blackjack" => run_blackjack(iterations),
        #[cfg(feature = "physics")]
        "platformer" => run_platformer(iterations),
        _ => {
            eprintln!("unknown game '{game}', expected tictactoe|blackjack|platformer");
            std::process::exit(2);
        }
    };

    println!("game={game} iterations={iterations} digest={digest:016x}");
}

#[cfg(not(feature = "builtin"))]
fn main() {
    let _ = env::args();
    eprintln!("perf_probe requires the builtin feature");
    std::process::exit(1);
}
