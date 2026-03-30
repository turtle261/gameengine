#![cfg(feature = "builtin-games")]

use criterion::{Criterion, criterion_group, criterion_main};
use gameengine::games::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::games::{Platformer, PlatformerAction};
use gameengine::{PlayerAction, Session};

fn bench_tictactoe(c: &mut Criterion) {
    c.bench_function("tictactoe_step", |b| {
        b.iter(|| {
            let mut session = Session::new(TicTacToe, 7);
            let actions = [
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
            ];
            for action in &actions {
                if session.is_terminal() {
                    break;
                }
                session.step(std::slice::from_ref(action));
            }
        })
    });
}

fn bench_blackjack(c: &mut Criterion) {
    c.bench_function("blackjack_step", |b| {
        b.iter(|| {
            let mut session = Session::new(Blackjack, 11);
            let actions = [
                PlayerAction {
                    player: 0,
                    action: BlackjackAction::Hit,
                },
                PlayerAction {
                    player: 0,
                    action: BlackjackAction::Stand,
                },
            ];
            for action in &actions {
                if session.is_terminal() {
                    break;
                }
                session.step(std::slice::from_ref(action));
            }
        })
    });
}

#[cfg(feature = "physics")]
fn bench_platformer(c: &mut Criterion) {
    c.bench_function("platformer_step", |b| {
        b.iter(|| {
            let mut session = Session::new(Platformer::default(), 5);
            let actions = [
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
            ];
            for action in &actions {
                if session.is_terminal() {
                    break;
                }
                session.step(std::slice::from_ref(action));
            }
        })
    });
}

#[cfg(feature = "physics")]
fn bench_platformer_rewind(c: &mut Criterion) {
    c.bench_function("platformer_rewind", |b| {
        b.iter(|| {
            let mut session = Session::new(Platformer::default(), 5);
            let actions = [
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
                    action: PlatformerAction::Right,
                },
            ];
            for action in &actions {
                session.step(std::slice::from_ref(action));
            }
            assert!(session.rewind_to(2));
        })
    });
}

#[cfg(feature = "physics")]
criterion_group!(
    benches,
    bench_tictactoe,
    bench_blackjack,
    bench_platformer,
    bench_platformer_rewind
);
#[cfg(not(feature = "physics"))]
criterion_group!(benches, bench_tictactoe, bench_blackjack);
criterion_main!(benches);
