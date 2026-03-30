use std::env;
use std::io::{self, Write};

#[cfg(feature = "render")]
use gameengine::InteractiveSession;
use gameengine::buffer::Buffer;
use gameengine::games::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::games::{Platformer, PlatformerAction};
use gameengine::policy::{FirstLegalPolicy, Policy, RandomPolicy, ScriptedPolicy};
#[cfg(feature = "render")]
use gameengine::render::{
    PassivePolicyDriver, RenderConfig, RenderMode, RendererApp, TurnBasedDriver,
};
#[cfg(all(feature = "render", feature = "physics"))]
use gameengine::render::{RealtimeDriver, builtin};
use gameengine::{CompactGame, Game, Session, stable_hash};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "list" => {
            println!("tictactoe");
            println!("blackjack");
            #[cfg(feature = "physics")]
            println!("platformer");
            Ok(())
        }
        "play" | "replay" => {
            let game = args
                .next()
                .ok_or_else(|| "missing game name for play/replay".to_string())?;
            let config = CliConfig::parse(args)?;
            match game.as_str() {
                "tictactoe" => run_tictactoe(config),
                "blackjack" => run_blackjack(config),
                #[cfg(feature = "physics")]
                "platformer" => run_platformer(config),
                _ => Err(format!("unknown game: {game}")),
            }
        }
        "validate" => run_validation_smoke(),
        _ => Err(format!("unknown command: {command}")),
    }
}

#[derive(Clone, Debug)]
struct CliConfig {
    seed: u64,
    max_steps: usize,
    policy: String,
    render: bool,
    render_physics: bool,
    ticks_per_second: f64,
    vsync: bool,
    debug_overlay: bool,
}

impl CliConfig {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self {
            seed: 1,
            max_steps: 64,
            policy: "human".to_string(),
            render: false,
            render_physics: false,
            ticks_per_second: 12.0,
            vsync: true,
            debug_overlay: false,
        };

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--seed" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "missing value after --seed".to_string())?;
                    config.seed = value
                        .parse()
                        .map_err(|_| format!("invalid seed value: {value}"))?;
                }
                "--max-steps" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "missing value after --max-steps".to_string())?;
                    config.max_steps = value
                        .parse()
                        .map_err(|_| format!("invalid max-steps value: {value}"))?;
                }
                "--policy" => {
                    config.policy = iter
                        .next()
                        .ok_or_else(|| "missing value after --policy".to_string())?;
                }
                "--render" => {
                    config.render = true;
                }
                "--render-physics" => {
                    config.render_physics = true;
                }
                "--ticks-per-second" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "missing value after --ticks-per-second".to_string())?;
                    config.ticks_per_second = value
                        .parse()
                        .map_err(|_| format!("invalid ticks-per-second value: {value}"))?;
                }
                "--no-vsync" => {
                    config.vsync = false;
                }
                "--debug-overlay" => {
                    config.debug_overlay = true;
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }

        if config.render && config.render_physics {
            return Err("--render and --render-physics are mutually exclusive".to_string());
        }
        if !config.ticks_per_second.is_finite() || config.ticks_per_second <= 0.0 {
            return Err("--ticks-per-second must be a finite positive number".to_string());
        }

        Ok(config)
    }
}

fn run_tictactoe(config: CliConfig) -> Result<(), String> {
    if config.render_physics {
        return Err("tictactoe does not support --render-physics".to_string());
    }
    #[cfg(feature = "render")]
    if config.render {
        return run_tictactoe_render(config);
    }
    if config.render {
        return Err("the crate was built without the render feature".to_string());
    }

    let game = TicTacToe;
    let mut session = Session::new(game, config.seed);
    let mut human = HumanTicTacToe;
    let mut random = RandomPolicy;
    let mut first = FirstLegalPolicy;
    let mut scripted = ScriptedPolicy::new(parse_tictactoe_script(&config.policy));
    let trace_hash = match config.policy.as_str() {
        "human" => run_with_policy(&mut session, config.max_steps, &mut human),
        "random" => run_with_policy(&mut session, config.max_steps, &mut random),
        "first" => run_with_policy(&mut session, config.max_steps, &mut first),
        policy if policy.starts_with("script:") => {
            run_with_policy(&mut session, config.max_steps, &mut scripted)
        }
        other => return Err(format!("unsupported tictactoe policy: {other}")),
    };
    println!("trace hash: {trace_hash:016x}");
    Ok(())
}

fn run_blackjack(config: CliConfig) -> Result<(), String> {
    if config.render_physics {
        return Err("blackjack does not support --render-physics".to_string());
    }
    #[cfg(feature = "render")]
    if config.render {
        return run_blackjack_render(config);
    }
    if config.render {
        return Err("the crate was built without the render feature".to_string());
    }

    let game = Blackjack;
    let mut session = Session::new(game, config.seed);
    let mut human = HumanBlackjack;
    let mut random = RandomPolicy;
    let mut first = FirstLegalPolicy;
    let mut scripted = ScriptedPolicy::new(parse_blackjack_script(&config.policy));
    let trace_hash = match config.policy.as_str() {
        "human" => run_with_policy(&mut session, config.max_steps, &mut human),
        "random" => run_with_policy(&mut session, config.max_steps, &mut random),
        "first" => run_with_policy(&mut session, config.max_steps, &mut first),
        policy if policy.starts_with("script:") => {
            run_with_policy(&mut session, config.max_steps, &mut scripted)
        }
        other => return Err(format!("unsupported blackjack policy: {other}")),
    };
    println!("trace hash: {trace_hash:016x}");
    Ok(())
}

#[cfg(feature = "physics")]
fn run_platformer(config: CliConfig) -> Result<(), String> {
    #[cfg(feature = "render")]
    if config.render || config.render_physics {
        return run_platformer_render(config);
    }
    if config.render || config.render_physics {
        return Err("the crate was built without the render feature".to_string());
    }

    let game = Platformer::default();
    let mut session = Session::new(game, config.seed);
    let mut human = HumanPlatformer;
    let mut random = RandomPolicy;
    let mut first = FirstLegalPolicy;
    let mut scripted = ScriptedPolicy::new(parse_platformer_script(&config.policy));
    let trace_hash = match config.policy.as_str() {
        "human" => run_with_policy(&mut session, config.max_steps, &mut human),
        "random" => run_with_policy(&mut session, config.max_steps, &mut random),
        "first" => run_with_policy(&mut session, config.max_steps, &mut first),
        policy if policy.starts_with("script:") => {
            run_with_policy(&mut session, config.max_steps, &mut scripted)
        }
        other => return Err(format!("unsupported platformer policy: {other}")),
    };
    println!("trace hash: {trace_hash:016x}");
    Ok(())
}

fn run_with_policy<G, P>(session: &mut Session<G>, max_steps: usize, policy: &mut P) -> u64
where
    G: Game + CompactGame + Copy,
    P: Policy<G>,
{
    let mut policies: [&mut dyn Policy<G>; 1] = [policy];
    while !session.is_terminal() && (session.current_tick() as usize) < max_steps {
        let outcome = session.step_with_policies(&mut policies).clone();
        let spectator = session.spectator_observation();
        let mut compact = G::WordBuf::default();
        session
            .game()
            .encode_spectator_observation(&spectator, &mut compact);
        println!(
            "tick={} reward={} terminal={} compact={:?}",
            session.current_tick(),
            outcome.reward_for(0),
            session.is_terminal(),
            compact.as_slice(),
        );
        println!("{spectator:#?}");
    }
    stable_hash(session.trace())
}

fn run_validation_smoke() -> Result<(), String> {
    let ttt_hash = {
        let mut session = Session::new(TicTacToe, 7);
        let mut scripted = ScriptedPolicy::new(vec![
            TicTacToeAction(0),
            TicTacToeAction(4),
            TicTacToeAction(8),
        ]);
        run_with_policy(&mut session, 8, &mut scripted)
    };
    let blackjack_hash = {
        let mut session = Session::new(Blackjack, 11);
        let mut scripted = ScriptedPolicy::new(vec![BlackjackAction::Hit, BlackjackAction::Stand]);
        run_with_policy(&mut session, 8, &mut scripted)
    };
    #[cfg(feature = "physics")]
    let platformer_hash = {
        let mut session = Session::new(Platformer::default(), 3);
        let mut scripted = ScriptedPolicy::new(vec![
            PlatformerAction::Right,
            PlatformerAction::Jump,
            PlatformerAction::Right,
        ]);
        run_with_policy(&mut session, 8, &mut scripted)
    };
    println!("tictactoe trace hash: {ttt_hash:016x}");
    println!("blackjack trace hash: {blackjack_hash:016x}");
    #[cfg(feature = "physics")]
    println!("platformer trace hash: {platformer_hash:016x}");
    Ok(())
}

#[cfg(feature = "render")]
fn build_render_config(config: &CliConfig, mode: RenderMode) -> RenderConfig {
    RenderConfig {
        tick_rate_hz: config.ticks_per_second,
        max_catch_up_ticks: 8,
        vsync: config.vsync,
        show_debug_overlay: config.debug_overlay,
        mode,
        ..RenderConfig::default()
    }
}

#[cfg(feature = "render")]
fn run_tictactoe_render(config: CliConfig) -> Result<(), String> {
    use gameengine::render::builtin::TicTacToePresenter;

    let render_config = build_render_config(&config, RenderMode::Observation);
    match config.policy.as_str() {
        "human" => RendererApp::new(
            render_config,
            TurnBasedDriver::new(InteractiveSession::new(TicTacToe, config.seed)),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        "random" => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(TicTacToe, config.seed),
                RandomPolicy,
            ),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        "first" => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(TicTacToe, config.seed),
                FirstLegalPolicy,
            ),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        policy if policy.starts_with("script:") => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(TicTacToe, config.seed),
                ScriptedPolicy::new(parse_tictactoe_script(&config.policy)),
            ),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        other => Err(format!("unsupported tictactoe policy: {other}")),
    }
}

#[cfg(feature = "render")]
fn run_blackjack_render(config: CliConfig) -> Result<(), String> {
    use gameengine::render::builtin::BlackjackPresenter;

    let render_config = build_render_config(&config, RenderMode::Observation);
    match config.policy.as_str() {
        "human" => RendererApp::new(
            render_config,
            TurnBasedDriver::new(InteractiveSession::new(Blackjack, config.seed)),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        "random" => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(Blackjack, config.seed),
                RandomPolicy,
            ),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        "first" => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(Blackjack, config.seed),
                FirstLegalPolicy,
            ),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        policy if policy.starts_with("script:") => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(Blackjack, config.seed),
                ScriptedPolicy::new(parse_blackjack_script(&config.policy)),
            ),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        other => Err(format!("unsupported blackjack policy: {other}")),
    }
}

#[cfg(all(feature = "render", feature = "physics"))]
fn run_platformer_render(config: CliConfig) -> Result<(), String> {
    let mode = if config.render_physics {
        RenderMode::OracleWorld
    } else {
        RenderMode::Observation
    };
    let render_config = build_render_config(&config, mode);
    let game = Platformer::default();

    if config.render_physics {
        match config.policy.as_str() {
            "human" => RendererApp::new(
                render_config,
                RealtimeDriver::new(
                    InteractiveSession::new(game, config.seed),
                    PlatformerAction::Stay,
                ),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            "random" => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(InteractiveSession::new(game, config.seed), RandomPolicy),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            "first" => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    FirstLegalPolicy,
                ),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            policy if policy.starts_with("script:") => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    ScriptedPolicy::new(parse_platformer_script(&config.policy)),
                ),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            other => Err(format!("unsupported platformer policy: {other}")),
        }
    } else {
        match config.policy.as_str() {
            "human" => RendererApp::new(
                render_config,
                RealtimeDriver::new(
                    InteractiveSession::new(game, config.seed),
                    PlatformerAction::Stay,
                ),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            "random" => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(InteractiveSession::new(game, config.seed), RandomPolicy),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            "first" => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    FirstLegalPolicy,
                ),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            policy if policy.starts_with("script:") => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    ScriptedPolicy::new(parse_platformer_script(&config.policy)),
                ),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            other => Err(format!("unsupported platformer policy: {other}")),
        }
    }
}

fn print_usage() {
    println!("usage:");
    println!("  gameengine list");
    println!(
        "  gameengine play <game> [--seed N] [--max-steps N] [--policy human|random|first|script:...]"
    );
    println!("  gameengine replay <game> [--seed N] [--max-steps N] [--policy script:...]");
    println!("  gameengine validate");
    println!("optional rendering flags:");
    println!("  --render");
    println!("  --render-physics");
    println!("  --ticks-per-second <f64>");
    println!("  --no-vsync");
    println!("  --debug-overlay");
}

fn prompt(message: &str) -> io::Result<String> {
    print!("{message}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn parse_tictactoe_script(spec: &str) -> Vec<TicTacToeAction> {
    parse_script(spec, |token| token.parse::<u8>().ok().map(TicTacToeAction))
}

fn parse_blackjack_script(spec: &str) -> Vec<BlackjackAction> {
    parse_script(spec, |token| match token.to_ascii_lowercase().as_str() {
        "hit" | "h" => Some(BlackjackAction::Hit),
        "stand" | "s" => Some(BlackjackAction::Stand),
        _ => None,
    })
}

#[cfg(feature = "physics")]
fn parse_platformer_script(spec: &str) -> Vec<PlatformerAction> {
    parse_script(spec, |token| match token.to_ascii_lowercase().as_str() {
        "stay" | "s" => Some(PlatformerAction::Stay),
        "left" | "l" => Some(PlatformerAction::Left),
        "right" | "r" => Some(PlatformerAction::Right),
        "jump" | "j" => Some(PlatformerAction::Jump),
        _ => None,
    })
}

fn parse_script<A, F>(spec: &str, parser: F) -> Vec<A>
where
    F: Fn(&str) -> Option<A>,
{
    let Some(script) = spec.strip_prefix("script:") else {
        return Vec::new();
    };
    script
        .split(',')
        .filter_map(|token| parser(token.trim()))
        .collect()
}

struct HumanTicTacToe;

impl Policy<TicTacToe> for HumanTicTacToe {
    fn choose_action(
        &mut self,
        _game: &TicTacToe,
        _state: &<TicTacToe as Game>::State,
        _player: usize,
        _observation: &<TicTacToe as Game>::PlayerObservation,
        legal_actions: &[<TicTacToe as Game>::Action],
        _rng: &mut gameengine::DeterministicRng,
    ) -> <TicTacToe as Game>::Action {
        loop {
            let input = prompt("choose move [0-8]: ").expect("stdin prompt failed");
            if let Ok(index) = input.trim().parse::<u8>() {
                let candidate = TicTacToeAction(index);
                if legal_actions.contains(&candidate) {
                    return candidate;
                }
            }
            println!("legal moves: {:?}", legal_actions);
        }
    }
}

struct HumanBlackjack;

impl Policy<Blackjack> for HumanBlackjack {
    fn choose_action(
        &mut self,
        _game: &Blackjack,
        _state: &<Blackjack as Game>::State,
        _player: usize,
        _observation: &<Blackjack as Game>::PlayerObservation,
        legal_actions: &[<Blackjack as Game>::Action],
        _rng: &mut gameengine::DeterministicRng,
    ) -> <Blackjack as Game>::Action {
        loop {
            let input = prompt("choose action [hit/stand]: ").expect("stdin prompt failed");
            let candidate = match input.trim().to_ascii_lowercase().as_str() {
                "hit" | "h" => BlackjackAction::Hit,
                "stand" | "s" => BlackjackAction::Stand,
                _ => {
                    println!("legal actions: {:?}", legal_actions);
                    continue;
                }
            };
            if legal_actions.contains(&candidate) {
                return candidate;
            }
            println!("legal actions: {:?}", legal_actions);
        }
    }
}

#[cfg(feature = "physics")]
struct HumanPlatformer;

#[cfg(feature = "physics")]
impl Policy<Platformer> for HumanPlatformer {
    fn choose_action(
        &mut self,
        _game: &Platformer,
        _state: &<Platformer as Game>::State,
        _player: usize,
        _observation: &<Platformer as Game>::PlayerObservation,
        legal_actions: &[<Platformer as Game>::Action],
        _rng: &mut gameengine::DeterministicRng,
    ) -> <Platformer as Game>::Action {
        loop {
            let input =
                prompt("choose action [stay/left/right/jump]: ").expect("stdin prompt failed");
            let candidate = match input.trim().to_ascii_lowercase().as_str() {
                "stay" | "s" => PlatformerAction::Stay,
                "left" | "l" => PlatformerAction::Left,
                "right" | "r" => PlatformerAction::Right,
                "jump" | "j" => PlatformerAction::Jump,
                _ => {
                    println!("legal actions: {:?}", legal_actions);
                    continue;
                }
            };
            if legal_actions.contains(&candidate) {
                return candidate;
            }
            println!("legal actions: {:?}", legal_actions);
        }
    }
}
