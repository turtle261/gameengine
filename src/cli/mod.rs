//! Command-line entrypoints for listing, playing, replaying, and validating games.

use std::env;
use std::fmt::Debug;
use std::io::{self, Write};

use crate::buffer::Buffer;
#[cfg(feature = "builtin")]
use crate::builtin::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use crate::builtin::{Platformer, PlatformerAction};
use crate::core::observe::{Observe, Observer};
use crate::policy::{FirstLegalPolicy, Policy, RandomPolicy, ScriptedPolicy};
use crate::registry::{all_games, find_game};
#[cfg(feature = "render")]
use crate::render::{PassivePolicyDriver, RenderConfig, RenderMode, RendererApp, TurnBasedDriver};
#[cfg(all(feature = "render", feature = "physics"))]
use crate::render::{RealtimeDriver, builtin};
#[cfg(feature = "render")]
use crate::session::InteractiveSession;
use crate::{Game, PlayerAction, Session, stable_hash};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum RunMode {
    Play,
    Replay,
}

#[derive(Debug)]
enum PolicyChoice<A> {
    Human,
    Random,
    First,
    Scripted(Vec<A>),
}

fn resolve_policy_choice<A>(
    mode: RunMode,
    policy: &str,
    parse_script: fn(&str) -> Result<Vec<A>, String>,
    game_name: &'static str,
) -> Result<PolicyChoice<A>, String> {
    match policy {
        "human" if matches!(mode, RunMode::Play) => Ok(PolicyChoice::Human),
        "human" => Err(format!(
            "unsupported {game_name} policy for replay mode: human"
        )),
        "random" => Ok(PolicyChoice::Random),
        "first" => Ok(PolicyChoice::First),
        script if script.starts_with("script:") => parse_script(script)
            .map(PolicyChoice::Scripted)
            .map_err(|error| format!("{game_name} script parse error: {error}")),
        other => Err(format!("unsupported {game_name} policy: {other}")),
    }
}

/// Runs the CLI using process command-line arguments.
pub fn run_from_env() -> Result<(), String> {
    run_from_args(env::args().skip(1))
}

/// Runs the CLI using a supplied argument iterator.
pub fn run_from_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        print_usage();
        return Ok(());
    };

    match command.as_str() {
        "list" => {
            for descriptor in all_games() {
                println!("{}", descriptor.name);
            }
            Ok(())
        }
        "play" => {
            let game = args
                .next()
                .ok_or_else(|| "missing game name for play".to_string())?;
            let config = CliConfig::parse(args)?;
            run_descriptor(&game, config, RunMode::Play)
        }
        "replay" => {
            let game = args
                .next()
                .ok_or_else(|| "missing game name for replay".to_string())?;
            let config = CliConfig::parse(args)?;
            run_descriptor(&game, config, RunMode::Replay)
        }
        "validate" => run_validation_smoke(),
        _ => Err(format!("unknown command: {command}")),
    }
}

fn run_descriptor(game_name: &str, config: CliConfig, mode: RunMode) -> Result<(), String> {
    let descriptor = find_game(game_name).ok_or_else(|| format!("unknown game: {game_name}"))?;
    (descriptor.runner)(config, mode)
}

/// Parsed command-line execution configuration.
#[derive(Clone, Debug)]
pub(crate) struct CliConfig {
    seed: u64,
    max_steps: usize,
    policy: String,
    policy_explicit: bool,
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
            policy_explicit: false,
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
                    config.policy_explicit = true;
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

    fn policy_for_mode(&self, mode: RunMode) -> &str {
        if self.policy_explicit {
            &self.policy
        } else {
            match mode {
                RunMode::Play => "human",
                RunMode::Replay => "first",
            }
        }
    }
}

#[cfg(feature = "render")]
fn should_stop_at_tick(tick: u64, max_steps: Option<usize>) -> bool {
    max_steps.is_some_and(|limit| tick as usize >= limit)
}

fn collect_scripted_joint_actions<G>(
    session: &Session<G>,
    script: &[G::Action],
    position: &mut usize,
    game_name: &'static str,
) -> Result<G::JointActionBuf, String>
where
    G: Game,
{
    let mut players = G::PlayerBuf::default();
    session.game().players_to_act(session.state(), &mut players);

    let mut joint_actions = G::JointActionBuf::default();
    let mut legal_actions = G::ActionBuf::default();
    for &player in players.as_slice() {
        legal_actions.clear();
        session
            .game()
            .legal_actions(session.state(), player, &mut legal_actions);
        if legal_actions.as_slice().is_empty() {
            return Err(format!(
                "{game_name} player {player} has no legal actions in a non-terminal state"
            ));
        }
        let Some(action) = script.get(*position).copied() else {
            return Err(format!(
                "{game_name} scripted policy exhausted at index {}",
                *position
            ));
        };
        if !legal_actions.as_slice().contains(&action) {
            return Err(format!(
                "{game_name} scripted policy action at index {} is illegal for current state",
                *position
            ));
        }
        joint_actions
            .push(PlayerAction { player, action })
            .expect("joint action buffer capacity exceeded");
        *position += 1;
    }

    Ok(joint_actions)
}

#[cfg(feature = "render")]
fn validate_scripted_policy<G>(
    game: G,
    seed: u64,
    script: &[G::Action],
    max_steps: Option<usize>,
    game_name: &'static str,
) -> Result<(), String>
where
    G: Game + Copy,
{
    let mut session = Session::new(game, seed);
    let mut position = 0usize;
    while !session.is_terminal() && !should_stop_at_tick(session.current_tick(), max_steps) {
        let joint_actions =
            collect_scripted_joint_actions(&session, script, &mut position, game_name)?;
        session.step_with_joint_actions(&joint_actions);
    }
    Ok(())
}

fn run_scripted_headless_game<G>(
    game: G,
    seed: u64,
    script: &[G::Action],
    max_steps: usize,
    game_name: &'static str,
) -> Result<u64, String>
where
    G: Game + Observe + Copy,
    G::Obs: Debug,
{
    let mut session = Session::new(game, seed);
    let mut position = 0usize;
    while !session.is_terminal() && (session.current_tick() as usize) < max_steps {
        let joint_actions =
            collect_scripted_joint_actions(&session, script, &mut position, game_name)?;
        let reward = {
            let outcome = session.step_with_joint_actions(&joint_actions);
            outcome.reward_for(0)
        };
        let observation = session.game().observe(session.state(), Observer::Player(0));
        let mut compact = G::WordBuf::default();
        session
            .game()
            .encode_observation(&observation, &mut compact);
        println!(
            "tick={} reward={} terminal={} compact={:?}",
            session.current_tick(),
            reward,
            session.is_terminal(),
            compact.as_slice(),
        );
        println!("{observation:#?}");
    }
    Ok(stable_hash(session.trace()))
}

fn run_headless_game<G, H>(
    game: G,
    config: &CliConfig,
    mode: RunMode,
    mut human: H,
    parse_script: fn(&str) -> Result<Vec<G::Action>, String>,
    game_name: &'static str,
) -> Result<(), String>
where
    G: Game + Observe + Copy,
    G::Obs: Debug,
    H: Policy<G>,
{
    let mut session = Session::new(game, config.seed);
    let mut random = RandomPolicy;
    let mut first = FirstLegalPolicy;
    let trace_hash =
        match resolve_policy_choice(mode, config.policy_for_mode(mode), parse_script, game_name)? {
            PolicyChoice::Human => run_with_policy(&mut session, config.max_steps, &mut human),
            PolicyChoice::Random => run_with_policy(&mut session, config.max_steps, &mut random),
            PolicyChoice::First => run_with_policy(&mut session, config.max_steps, &mut first),
            PolicyChoice::Scripted(script) => {
                run_scripted_headless_game(game, config.seed, &script, config.max_steps, game_name)?
            }
        };

    println!("trace hash: {trace_hash:016x}");
    Ok(())
}

pub(crate) fn run_tictactoe(config: CliConfig, mode: RunMode) -> Result<(), String> {
    if config.render_physics {
        return Err("tictactoe does not support --render-physics".to_string());
    }
    #[cfg(feature = "render")]
    if config.render {
        return run_tictactoe_render(config, mode);
    }
    if config.render {
        return Err("the crate was built without the render feature".to_string());
    }

    run_headless_game(
        TicTacToe,
        &config,
        mode,
        HumanTicTacToe,
        parse_tictactoe_script,
        "tictactoe",
    )
}

pub(crate) fn run_blackjack(config: CliConfig, mode: RunMode) -> Result<(), String> {
    if config.render_physics {
        return Err("blackjack does not support --render-physics".to_string());
    }
    #[cfg(feature = "render")]
    if config.render {
        return run_blackjack_render(config, mode);
    }
    if config.render {
        return Err("the crate was built without the render feature".to_string());
    }

    run_headless_game(
        Blackjack,
        &config,
        mode,
        HumanBlackjack,
        parse_blackjack_script,
        "blackjack",
    )
}

#[cfg(feature = "physics")]
pub(crate) fn run_platformer(config: CliConfig, mode: RunMode) -> Result<(), String> {
    #[cfg(feature = "render")]
    if config.render || config.render_physics {
        return run_platformer_render(config, mode);
    }
    if config.render || config.render_physics {
        return Err("the crate was built without the render feature".to_string());
    }

    run_headless_game(
        Platformer::default(),
        &config,
        mode,
        HumanPlatformer,
        parse_platformer_script,
        "platformer",
    )
}

fn run_with_policy<G, P>(session: &mut Session<G>, max_steps: usize, policy: &mut P) -> u64
where
    G: Game + Observe + Copy,
    G::Obs: Debug,
    P: Policy<G>,
{
    let mut policies: [&mut dyn Policy<G>; 1] = [policy];
    while !session.is_terminal() && (session.current_tick() as usize) < max_steps {
        let reward = {
            let outcome = session.step_with_policies(&mut policies);
            outcome.reward_for(0)
        };
        let observation = session.game().observe(session.state(), Observer::Player(0));
        let mut compact = G::WordBuf::default();
        session
            .game()
            .encode_observation(&observation, &mut compact);
        println!(
            "tick={} reward={} terminal={} compact={:?}",
            session.current_tick(),
            reward,
            session.is_terminal(),
            compact.as_slice(),
        );
        println!("{observation:#?}");
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
fn run_tictactoe_render(config: CliConfig, mode: RunMode) -> Result<(), String> {
    use crate::render::builtin::TicTacToePresenter;

    let render_config = build_render_config(&config, RenderMode::Observation);
    match resolve_policy_choice(
        mode,
        config.policy_for_mode(mode),
        parse_tictactoe_script,
        "tictactoe",
    )? {
        PolicyChoice::Human => RendererApp::new(
            render_config,
            TurnBasedDriver::new(InteractiveSession::new(TicTacToe, config.seed)),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::Random => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(TicTacToe, config.seed),
                RandomPolicy,
            ),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::First => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(TicTacToe, config.seed),
                FirstLegalPolicy,
            ),
            TicTacToePresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::Scripted(script) => {
            validate_scripted_policy(TicTacToe, config.seed, &script, None, "tictactoe")?;
            RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(TicTacToe, config.seed),
                    ScriptedPolicy::new_strict(script),
                ),
                TicTacToePresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string())
        }
    }
}

#[cfg(feature = "render")]
fn run_blackjack_render(config: CliConfig, mode: RunMode) -> Result<(), String> {
    use crate::render::builtin::BlackjackPresenter;

    let render_config = build_render_config(&config, RenderMode::Observation);
    match resolve_policy_choice(
        mode,
        config.policy_for_mode(mode),
        parse_blackjack_script,
        "blackjack",
    )? {
        PolicyChoice::Human => RendererApp::new(
            render_config,
            TurnBasedDriver::new(InteractiveSession::new(Blackjack, config.seed)),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::Random => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(Blackjack, config.seed),
                RandomPolicy,
            ),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::First => RendererApp::new(
            render_config,
            PassivePolicyDriver::new(
                InteractiveSession::new(Blackjack, config.seed),
                FirstLegalPolicy,
            ),
            BlackjackPresenter::default(),
        )
        .run_native()
        .map_err(|error| error.to_string()),
        PolicyChoice::Scripted(script) => {
            validate_scripted_policy(Blackjack, config.seed, &script, None, "blackjack")?;
            RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(Blackjack, config.seed),
                    ScriptedPolicy::new_strict(script),
                ),
                BlackjackPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string())
        }
    }
}

#[cfg(all(feature = "render", feature = "physics"))]
fn run_platformer_render(config: CliConfig, mode: RunMode) -> Result<(), String> {
    let render_mode = if config.render_physics {
        RenderMode::OracleWorld
    } else {
        RenderMode::Observation
    };
    let render_config = build_render_config(&config, render_mode);
    let game = Platformer::default();

    let policy_choice = resolve_policy_choice(
        mode,
        config.policy_for_mode(mode),
        parse_platformer_script,
        "platformer",
    )?;

    if config.render_physics {
        match policy_choice {
            PolicyChoice::Human => RendererApp::new(
                render_config,
                RealtimeDriver::new(
                    InteractiveSession::new(game, config.seed),
                    PlatformerAction::Stay,
                ),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::Random => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(InteractiveSession::new(game, config.seed), RandomPolicy),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::First => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    FirstLegalPolicy,
                ),
                builtin::PlatformerPhysicsPresenter::new(game.config),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::Scripted(script) => {
                validate_scripted_policy(game, config.seed, &script, None, "platformer")?;
                RendererApp::new(
                    render_config,
                    PassivePolicyDriver::new(
                        InteractiveSession::new(game, config.seed),
                        ScriptedPolicy::new_strict(script),
                    ),
                    builtin::PlatformerPhysicsPresenter::new(game.config),
                )
                .run_native()
                .map_err(|error| error.to_string())
            }
        }
    } else {
        match policy_choice {
            PolicyChoice::Human => RendererApp::new(
                render_config,
                RealtimeDriver::new(
                    InteractiveSession::new(game, config.seed),
                    PlatformerAction::Stay,
                ),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::Random => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(InteractiveSession::new(game, config.seed), RandomPolicy),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::First => RendererApp::new(
                render_config,
                PassivePolicyDriver::new(
                    InteractiveSession::new(game, config.seed),
                    FirstLegalPolicy,
                ),
                builtin::PlatformerPresenter::default(),
            )
            .run_native()
            .map_err(|error| error.to_string()),
            PolicyChoice::Scripted(script) => {
                validate_scripted_policy(game, config.seed, &script, None, "platformer")?;
                RendererApp::new(
                    render_config,
                    PassivePolicyDriver::new(
                        InteractiveSession::new(game, config.seed),
                        ScriptedPolicy::new_strict(script),
                    ),
                    builtin::PlatformerPresenter::default(),
                )
                .run_native()
                .map_err(|error| error.to_string())
            }
        }
    }
}

fn print_usage() {
    println!("usage:");
    println!("  gameengine list");
    println!(
        "  gameengine play <game> [--seed N] [--max-steps N] [--policy human|random|first|script:...]"
    );
    println!(
        "  gameengine replay <game> [--seed N] [--max-steps N] [--policy first|random|script:...]"
    );
    println!("  gameengine validate");
    println!("available games:");
    for descriptor in all_games() {
        println!("  - {}", descriptor.name);
    }
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

fn parse_tictactoe_script(spec: &str) -> Result<Vec<TicTacToeAction>, String> {
    parse_script(spec, |token| token.parse::<u8>().ok().map(TicTacToeAction))
}

fn parse_blackjack_script(spec: &str) -> Result<Vec<BlackjackAction>, String> {
    parse_script(spec, |token| match token.to_ascii_lowercase().as_str() {
        "hit" | "h" => Some(BlackjackAction::Hit),
        "stand" | "s" => Some(BlackjackAction::Stand),
        _ => None,
    })
}

#[cfg(feature = "physics")]
fn parse_platformer_script(spec: &str) -> Result<Vec<PlatformerAction>, String> {
    parse_script(spec, |token| match token.to_ascii_lowercase().as_str() {
        "stay" | "s" => Some(PlatformerAction::Stay),
        "left" | "l" => Some(PlatformerAction::Left),
        "right" | "r" => Some(PlatformerAction::Right),
        "jump" | "j" => Some(PlatformerAction::Jump),
        _ => None,
    })
}

fn parse_script<A, F>(spec: &str, parser: F) -> Result<Vec<A>, String>
where
    F: Fn(&str) -> Option<A>,
{
    let Some(script) = spec.strip_prefix("script:") else {
        return Ok(Vec::new());
    };

    let mut actions = Vec::new();
    for (index, token) in script.split(',').enumerate() {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(format!("empty action token at position {index}"));
        }
        let action = parser(trimmed)
            .ok_or_else(|| format!("invalid action token at position {index}: {trimmed}"))?;
        actions.push(action);
    }
    Ok(actions)
}

struct HumanTicTacToe;

impl Policy<TicTacToe> for HumanTicTacToe {
    fn choose_action(
        &mut self,
        _game: &TicTacToe,
        _state: &<TicTacToe as Game>::State,
        _player: usize,
        _observation: &<TicTacToe as Game>::Obs,
        legal_actions: &[<TicTacToe as Game>::Action],
        _rng: &mut crate::DeterministicRng,
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
        _observation: &<Blackjack as Game>::Obs,
        legal_actions: &[<Blackjack as Game>::Action],
        _rng: &mut crate::DeterministicRng,
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
        _observation: &<Platformer as Game>::Obs,
        legal_actions: &[<Platformer as Game>::Action],
        _rng: &mut crate::DeterministicRng,
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

#[cfg(test)]
mod tests {
    use super::{
        CliConfig, PolicyChoice, RunMode, parse_tictactoe_script, resolve_policy_choice,
        run_scripted_headless_game,
    };
    use crate::builtin::{TicTacToe, TicTacToeAction};

    #[test]
    fn replay_defaults_to_first_policy() {
        let config = CliConfig::parse(Vec::<String>::new()).unwrap();
        let choice = resolve_policy_choice(
            RunMode::Replay,
            config.policy_for_mode(RunMode::Replay),
            parse_tictactoe_script,
            "tictactoe",
        )
        .unwrap();
        assert!(matches!(choice, PolicyChoice::First));
    }

    #[test]
    fn replay_accepts_explicit_random_policy() {
        let choice = resolve_policy_choice(
            RunMode::Replay,
            "random",
            parse_tictactoe_script,
            "tictactoe",
        )
        .unwrap();
        assert!(matches!(choice, PolicyChoice::Random));
    }

    #[test]
    fn scripted_headless_run_reports_exhaustion() {
        let error =
            run_scripted_headless_game(TicTacToe, 1, &[TicTacToeAction(0)], 64, "tictactoe")
                .unwrap_err();
        assert!(error.contains("scripted policy exhausted"));
    }

    #[test]
    fn scripted_headless_run_reports_illegal_action() {
        let error = run_scripted_headless_game(
            TicTacToe,
            1,
            &[TicTacToeAction(0), TicTacToeAction(0)],
            64,
            "tictactoe",
        )
        .unwrap_err();
        assert!(error.contains("scripted policy action at index 1 is illegal"));
    }
}
