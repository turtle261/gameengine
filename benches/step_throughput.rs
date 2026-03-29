use criterion::{Criterion, criterion_group, criterion_main};
use gameengine::games::{
    Blackjack, BlackjackAction, Platformer, PlatformerAction, TicTacToe, TicTacToeAction,
};
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

fn bench_platformer(c: &mut Criterion) {
    c.bench_function("platformer_step", |b| {
        b.iter(|| {
            let mut session = Session::new(Platformer, 5);
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

criterion_group!(benches, bench_tictactoe, bench_blackjack, bench_platformer);
criterion_main!(benches);
