#![cfg(feature = "builtin")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "parallel")]
use gameengine::InteractiveSession;
use gameengine::buffer::Buffer;
use gameengine::builtin::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::builtin::{Platformer, PlatformerAction};
use gameengine::{
    CompactSpec, DeterministicRng, FixedVec, Game, GameAuthoring, PlayerAction, PlayerReward,
    Session, KernelOutcome, stable_hash,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static VALIDATION_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static COUNT_ALLOCATIONS_ON_THIS_THREAD: Cell<bool> = const { Cell::new(false) };
}

struct AllocationCountGuard;

impl AllocationCountGuard {
    fn enter() -> Self {
        COUNT_ALLOCATIONS_ON_THIS_THREAD.with(|enabled| enabled.set(true));
        Self
    }
}

impl Drop for AllocationCountGuard {
    fn drop(&mut self) {
        COUNT_ALLOCATIONS_ON_THIS_THREAD.with(|enabled| enabled.set(false));
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOCATIONS_ON_THIS_THREAD.with(|enabled| {
            if enabled.get() {
                ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
            }
        });
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
    let _guard = AllocationCountGuard::enter();
    f();
    ALLOCATIONS.load(Ordering::SeqCst)
}

fn lock_validation() -> std::sync::MutexGuard<'static, ()> {
    match VALIDATION_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn capture_compact_trace<G>(
    game: G,
    seed: u64,
    actions: &[Vec<PlayerAction<G::Action>>],
) -> (Vec<Vec<u64>>, u64, u64)
where
    G: Game + Copy,
{
    let mut session = Session::new(game, seed);
    let mut compact_trace = Vec::new();
    for action_set in actions {
        if session.is_terminal() {
            break;
        }
        session.step(action_set);
        let spectator = session.spectator_observation();
        let mut encoded = G::WordBuf::default();
        session
            .game()
            .encode_spectator_observation(&spectator, &mut encoded);
        compact_trace.push(encoded.as_slice().to_vec());
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
    #[cfg(feature = "physics")]
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

    #[cfg(feature = "physics")]
    {
        let left = capture_compact_trace(Platformer::default(), 3, &platformer_actions);
        let right = capture_compact_trace(Platformer::default(), 3, &platformer_actions);
        assert_eq!(left, right);
    }
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

    #[cfg(feature = "physics")]
    {
        let platformer = Platformer::default();
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
}

#[cfg(feature = "physics")]
#[test]
fn rollback_and_fork_restore_exact_state() {
    let _guard = lock_validation();
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

    let mut session = Session::new(Platformer::default(), 3);
    for action in &actions {
        session.step(std::slice::from_ref(action));
    }

    let state_at_two = session.state_at(2).expect("missing state at tick 2");
    let mut rewound = session.clone();
    assert!(rewound.rewind_to(2));
    assert_eq!(rewound.state(), &state_at_two);

    let fork = session.fork_at(2).expect("fork should exist");
    assert_eq!(fork.state(), &state_at_two);
    assert_eq!(stable_hash(fork.trace()), stable_hash(rewound.trace()));
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
    assert_eq!(trace_hash, 0x5b96_1efc_b075_3027);

    let blackjack_actions = vec![vec![PlayerAction {
        player: 0,
        action: BlackjackAction::Hit,
    }]];
    let (compact, trace_hash, _) = capture_compact_trace(Blackjack, 11, &blackjack_actions);
    assert_eq!(compact, vec![vec![140693832466, 1449, 132, 0]]);
    assert_eq!(trace_hash, 0xfb29_3f00_ff61_bdc7);

    #[cfg(feature = "physics")]
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
    #[cfg(feature = "physics")]
    {
        let (compact, trace_hash, _) =
            capture_compact_trace(Platformer::default(), 3, &platformer_actions);
        assert_eq!(
            compact,
            vec![
                vec![4128769],
                vec![4063489],
                vec![4063234],
                vec![4063235],
                vec![3932419],
                vec![3932164],
                vec![3932165],
                vec![3670277],
                vec![3670022],
                vec![3670023],
                vec![3145991],
                vec![3145736],
                vec![3145737],
                vec![2097417],
                vec![2097162],
                vec![2097163],
                vec![4194571],
            ]
        );
        assert_eq!(trace_hash, 0x1ee7_fb2e_3689_eabf);
    }
}

#[test]
fn step_hot_paths_do_not_allocate_after_init() {
    let _guard = lock_validation();
    let game = TicTacToe;
    let mut state = game.init(7);
    let mut rng = DeterministicRng::from_seed_and_stream(7, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut action = FixedVec::<PlayerAction<TicTacToeAction>, 1>::default();
    action
        .push(PlayerAction {
            player: 0,
            action: TicTacToeAction(0),
        })
        .unwrap();
    let allocations = count_allocations(|| {
        game.step_in_place(&mut state, &action, &mut rng, &mut outcome);
    });
    assert_eq!(allocations, 0, "tictactoe step allocated: {allocations}");

    let game = Blackjack;
    let mut state = game.init(11);
    let mut rng = DeterministicRng::from_seed_and_stream(11, 1);
    let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
    let mut action = FixedVec::<PlayerAction<BlackjackAction>, 1>::default();
    action
        .push(PlayerAction {
            player: 0,
            action: BlackjackAction::Hit,
        })
        .unwrap();
    let allocations = count_allocations(|| {
        game.step_in_place(&mut state, &action, &mut rng, &mut outcome);
    });
    assert_eq!(allocations, 0, "blackjack step allocated: {allocations}");

    #[cfg(feature = "physics")]
    {
        let game = Platformer::default();
        let mut state = game.init(3);
        let mut rng = DeterministicRng::from_seed_and_stream(3, 1);
        let mut outcome = KernelOutcome::<FixedVec<PlayerReward, 1>>::default();
        let mut action = FixedVec::<PlayerAction<PlatformerAction>, 1>::default();
        action
            .push(PlayerAction {
                player: 0,
                action: PlatformerAction::Right,
            })
            .unwrap();
        let allocations = count_allocations(|| {
            game.step_in_place(&mut state, &action, &mut rng, &mut outcome);
        });
        assert_eq!(allocations, 0, "platformer step allocated: {allocations}");
    }
}

#[cfg(feature = "physics")]
#[test]
fn platformer_world_view_exposes_consistent_physics_snapshot() {
    let _guard = lock_validation();
    let session = Session::new(Platformer::default(), 3);
    let world = session.world_view();
    assert_eq!(world.physics.bodies.len(), 7);
    assert!(world.physics.invariant());
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
        (
            13,
            (0..320)
                .map(|_| {
                    vec![PlayerAction {
                        player: 0,
                        action: TicTacToeAction(9),
                    }]
                })
                .collect(),
        ),
    ];
    let parallel = replay_many(&TicTacToe, &traces);
    let serial: Vec<_> = traces
        .iter()
        .map(|(seed, steps)| {
            let mut session = InteractiveSession::new(TicTacToe, *seed);
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

#[test]
fn canonical_history_serialization_matches_specification_bit_layout() {
    use gameengine::core::env::{ActionToken, DefaultEnvironment};
    use gameengine::core::observe::Observer;
    use gameengine::{AixiEnvironment, BitStream, serialize_action, serialize_percept};

    // Reset a fresh TicTacToe environment and serialize the first percept.
    // TicTacToe's compact spec: action_count=9, observation_bits=18,
    // observation_stream_len=1 (=> observation_word_bits=18),
    // reward_bits=3, min_reward=-3, max_reward=2, reward_offset=3.
    let mut env: DefaultEnvironment<TicTacToe, 1> =
        DefaultEnvironment::new(TicTacToe, 7, Observer::Player(0));
    // Compact spec is fully determined by (game, params) — per specification
    // §6.5, `compact_spec()` equals `compact_spec_for(default_params())`.
    let spec: CompactSpec = TicTacToe.compact_spec();
    assert_eq!(spec.action_bits(), 4);
    assert_eq!(spec.observation_word_bits(), 18);
    assert_eq!(spec.reward_word_bits(), 3);

    let reset: gameengine::Percept<1> = env.reset_seed(7).expect("reset");
    let mut stream: BitStream = BitStream::new();
    serialize_percept(&mut stream, &spec, &reset);
    // One 18-bit observation word + 3-bit reward + 1 terminal bit = 22 bits.
    assert_eq!(stream.bit_len(), 22);
    // Reset percept has reward=0 (encoded=reward_offset=3) and terminal=false.
    // Reward field occupies bits [18..21], terminal bit at position 21.
    // Extract and confirm:
    let mut reward_encoded: u64 = 0;
    for i in 0..3 {
        if stream.get_bit(18 + i) {
            reward_encoded |= 1u64 << i;
        }
    }
    assert_eq!(reward_encoded, 3);
    assert!(!stream.get_bit(21));

    // Serialize the action `4` (center move) and confirm its 4-bit LSB layout.
    let token: ActionToken = ActionToken::try_new(4, 9).expect("alphabet");
    let mut action_stream: BitStream = BitStream::new();
    serialize_action(&mut action_stream, &spec, token);
    assert_eq!(action_stream.bit_len(), 4);
    // 4 = 0b0100 -> LSB-first bits: [0, 0, 1, 0].
    assert!(!action_stream.get_bit(0));
    assert!(!action_stream.get_bit(1));
    assert!(action_stream.get_bit(2));
    assert!(!action_stream.get_bit(3));
}
