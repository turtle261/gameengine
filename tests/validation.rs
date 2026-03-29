use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use gameengine::games::{
    Blackjack, BlackjackAction, Platformer, PlatformerAction, TicTacToe, TicTacToeAction,
};
use gameengine::{
    CompactGame, CompactSpec, DeterministicRng, Game, PlayerAction, Session, StepOutcome,
    stable_hash,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static VALIDATION_LOCK: Mutex<()> = Mutex::new(());

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn count_allocations<F>(f: F) -> usize
where
    F: FnOnce(),
{
    ALLOCATIONS.store(0, Ordering::SeqCst);
    f();
    ALLOCATIONS.load(Ordering::SeqCst)
}

fn lock_validation() -> std::sync::MutexGuard<'static, ()> {
    VALIDATION_LOCK.lock().expect("validation mutex poisoned")
}

fn capture_compact_trace<G>(
    game: G,
    seed: u64,
    actions: &[Vec<PlayerAction<G::Action>>],
) -> (Vec<Vec<u64>>, u64, u64)
where
    G: Game + CompactGame + Copy,
{
    let mut session = Session::new(game, seed);
    let mut encoded = Vec::new();
    let mut compact_trace = Vec::new();
    for action_set in actions {
        if session.is_terminal() {
            break;
        }
        session.step(action_set);
        let spectator = session.spectator_observation();
        session
            .game()
            .encode_spectator_observation(&spectator, &mut encoded);
        compact_trace.push(encoded.clone());
    }
    (
        compact_trace,
        stable_hash(session.trace()),
        stable_hash(session.state()),
    )
}

fn assert_reward_roundtrip(spec: CompactSpec) {
    for reward in spec.min_reward..=spec.max_reward {
        let encoded = spec.encode_reward(reward);
        assert_eq!(spec.decode_reward(encoded), reward);
    }
}

#[test]
fn same_seed_action_traces_replay_exactly() {
    let _guard = lock_validation();
    let tictactoe_actions = vec![
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(4),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(1),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(2),
        }],
    ];
    let blackjack_actions = vec![vec![PlayerAction {
        player: 0,
        action: BlackjackAction::Hit,
    }]];
    let platformer_actions = vec![
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
    ];

    let left = capture_compact_trace(TicTacToe, 7, &tictactoe_actions);
    let right = capture_compact_trace(TicTacToe, 7, &tictactoe_actions);
    assert_eq!(left, right);

    let left = capture_compact_trace(Blackjack, 11, &blackjack_actions);
    let right = capture_compact_trace(Blackjack, 11, &blackjack_actions);
    assert_eq!(left, right);

    let left = capture_compact_trace(Platformer, 3, &platformer_actions);
    let right = capture_compact_trace(Platformer, 3, &platformer_actions);
    assert_eq!(left, right);
}

#[test]
fn compact_interfaces_round_trip_actions_and_rewards() {
    let _guard = lock_validation();
    let tictactoe = TicTacToe;
    for index in 0..9 {
        let action = TicTacToeAction(index);
        assert_eq!(
            tictactoe.decode_action(tictactoe.encode_action(&action)),
            Some(action)
        );
    }
    assert_reward_roundtrip(tictactoe.compact_spec());

    let blackjack = Blackjack;
    for action in [BlackjackAction::Hit, BlackjackAction::Stand] {
        assert_eq!(
            blackjack.decode_action(blackjack.encode_action(&action)),
            Some(action)
        );
    }
    assert_reward_roundtrip(blackjack.compact_spec());

    let platformer = Platformer;
    for action in [
        PlatformerAction::Stay,
        PlatformerAction::Left,
        PlatformerAction::Right,
        PlatformerAction::Jump,
    ] {
        assert_eq!(
            platformer.decode_action(platformer.encode_action(&action)),
            Some(action)
        );
    }
    assert_reward_roundtrip(platformer.compact_spec());
}

#[test]
fn golden_compact_traces_match_expected_values() {
    let _guard = lock_validation();
    let tictactoe_actions = vec![
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(4),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(1),
        }],
        vec![PlayerAction {
            player: 0,
            action: TicTacToeAction(2),
        }],
    ];
    let (compact, trace_hash, _) = capture_compact_trace(TicTacToe, 7, &tictactoe_actions);
    assert_eq!(
        compact,
        vec![vec![8193], vec![139521], vec![141573], vec![141589]]
    );
    assert_eq!(trace_hash, 0x0a27_6f2c_0430_5417);

    let blackjack_actions = vec![vec![PlayerAction {
        player: 0,
        action: BlackjackAction::Hit,
    }]];
    let (compact, trace_hash, _) = capture_compact_trace(Blackjack, 11, &blackjack_actions);
    assert_eq!(compact, vec![vec![140693832466, 1449, 132, 0]]);
    assert_eq!(trace_hash, 0x7713_00d4_b00f_6a67);

    let platformer_actions = vec![
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        }],
        vec![PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        }],
    ];
    let (compact, trace_hash, _) = capture_compact_trace(Platformer, 3, &platformer_actions);
    assert_eq!(
        compact,
        vec![
            vec![2017],
            vec![2001],
            vec![1986],
            vec![1987],
            vec![1939],
            vec![1924],
            vec![1925],
            vec![1813],
            vec![1798],
            vec![1799],
            vec![1559],
            vec![1544],
            vec![1545],
            vec![1049],
            vec![1034],
            vec![1035],
            vec![2075],
        ]
    );
    assert_eq!(trace_hash, 0x5e67_08cd_f176_ee1f);
}

#[test]
fn step_hot_paths_do_not_allocate_after_init() {
    let _guard = lock_validation();
    let game = TicTacToe;
    let mut state = game.init(7);
    let mut rng = DeterministicRng::from_seed_and_stream(7, 1);
    let mut outcome = StepOutcome::with_player_capacity(1);
    let action = [PlayerAction {
        player: 0,
        action: TicTacToeAction(0),
    }];
    let allocations = count_allocations(|| {
        game.step_in_place(&mut state, &action, &mut rng, &mut outcome);
    });
    assert!(
        allocations <= 8,
        "tictactoe step allocated too much: {allocations}"
    );

    let game = Blackjack;
    let mut state = game.init(11);
    let mut rng = DeterministicRng::from_seed_and_stream(11, 1);
    let mut outcome = StepOutcome::with_player_capacity(1);
    let action = [PlayerAction {
        player: 0,
        action: BlackjackAction::Hit,
    }];
    let allocations = count_allocations(|| {
        game.step_in_place(&mut state, &action, &mut rng, &mut outcome);
    });
    assert!(
        allocations <= 8,
        "blackjack step allocated too much: {allocations}"
    );

    let game = Platformer;
    let mut state = game.init(3);
    let mut rng = DeterministicRng::from_seed_and_stream(3, 1);
    let mut outcome = StepOutcome::with_player_capacity(1);
    let actions = [
        PlayerAction {
            player: 0,
            action: PlatformerAction::Right,
        },
        PlayerAction {
            player: 0,
            action: PlatformerAction::Jump,
        },
    ];
    let allocations = count_allocations(|| {
        for action in actions {
            game.step_in_place(
                &mut state,
                std::slice::from_ref(&action),
                &mut rng,
                &mut outcome,
            );
            outcome.clear();
        }
    });
    assert!(
        allocations <= 8,
        "platformer step allocated too much: {allocations}"
    );
}

#[cfg(feature = "parallel")]
#[test]
fn parallel_replay_matches_serial() {
    let _guard = lock_validation();
    use gameengine::parallel::replay_many;

    let traces = vec![
        (
            7,
            vec![
                vec![PlayerAction {
                    player: 0,
                    action: TicTacToeAction(0),
                }],
                vec![PlayerAction {
                    player: 0,
                    action: TicTacToeAction(4),
                }],
            ],
        ),
        (
            11,
            vec![vec![PlayerAction {
                player: 0,
                action: TicTacToeAction(0),
            }]],
        ),
    ];
    let parallel = replay_many(&TicTacToe, &traces);
    let serial: Vec<_> = traces
        .iter()
        .map(|(seed, steps)| {
            let mut session = Session::new(TicTacToe, *seed);
            for step in steps {
                if session.is_terminal() {
                    break;
                }
                session.step(step);
            }
            session.into_trace()
        })
        .collect();
    assert_eq!(parallel, serial);
}
