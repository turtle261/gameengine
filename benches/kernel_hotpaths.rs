#![cfg(feature = "builtin")]

use criterion::{Criterion, criterion_group, criterion_main};
use gameengine::builtin::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::builtin::{Platformer, PlatformerAction};
use gameengine::{PlayerAction, Session};

fn bench_tictactoe_kernel_step(c: &mut Criterion) {
    c.bench_function("tictactoe_session_step_kernel", |b| {
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
        let mut index = 0usize;
        b.iter(|| {
            if session.is_terminal() || session.current_tick() >= 200 {
                session.reset(7);
                index = 0;
            }
            let action = &script[index % script.len()];
            index += 1;
            let outcome = session.step(std::slice::from_ref(action));
            criterion::black_box(outcome.reward_for(0));
        })
    });
}

fn bench_tictactoe_checked_step(c: &mut Criterion) {
    c.bench_function("tictactoe_session_step_checked", |b| {
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
        let mut index = 0usize;
        b.iter(|| {
            if session.is_terminal() || session.current_tick() >= 200 {
                session.reset(7);
                index = 0;
            }
            let action = &script[index % script.len()];
            index += 1;
            let outcome = session.step_checked(std::slice::from_ref(action));
            criterion::black_box(outcome.reward_for(0));
        })
    });
}

fn bench_blackjack_kernel_step(c: &mut Criterion) {
    c.bench_function("blackjack_session_step_kernel", |b| {
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
        let mut index = 0usize;
        b.iter(|| {
            if session.is_terminal() || session.current_tick() >= 200 {
                session.reset(11);
                index = 0;
            }
            let action = &script[index % script.len()];
            index += 1;
            let outcome = session.step(std::slice::from_ref(action));
            criterion::black_box(outcome.reward_for(0));
        })
    });
}

#[cfg(feature = "physics")]
fn bench_platformer_kernel_step(c: &mut Criterion) {
    c.bench_function("platformer_session_step_kernel", |b| {
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
        let mut index = 0usize;
        b.iter(|| {
            if session.is_terminal() || session.current_tick() >= 200 {
                session.reset(5);
                index = 0;
            }
            let action = &script[index % script.len()];
            index += 1;
            let outcome = session.step(std::slice::from_ref(action));
            criterion::black_box(outcome.reward_for(0));
        })
    });
}

#[cfg(feature = "physics")]
fn bench_platformer_rewind_kernel(c: &mut Criterion) {
    c.bench_function("platformer_rewind_kernel", |b| {
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
            criterion::black_box(session.rewind_to(2));
        })
    });
}

#[cfg(feature = "physics")]
criterion_group!(
    benches,
    bench_tictactoe_kernel_step,
    bench_tictactoe_checked_step,
    bench_blackjack_kernel_step,
    bench_platformer_kernel_step,
    bench_platformer_rewind_kernel
);
#[cfg(not(feature = "physics"))]
criterion_group!(
    benches,
    bench_tictactoe_kernel_step,
    bench_tictactoe_checked_step,
    bench_blackjack_kernel_step
);
criterion_main!(benches);
