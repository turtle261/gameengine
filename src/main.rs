use std::env;
use std::io::{self, Write};

use gameengine::buffer::Buffer;
use gameengine::games::{Blackjack, BlackjackAction, TicTacToe, TicTacToeAction};
#[cfg(feature = "physics")]
use gameengine::games::{Platformer, PlatformerAction};
use gameengine::policy::{FirstLegalPolicy, Policy, RandomPolicy, ScriptedPolicy};
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
                other => return Err(format!("unknown argument: {other}")),
            }
        }

        Ok(config)
    }
}

fn run_tictactoe(config: CliConfig) -> Result<(), String> {
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
    let mut policies: Vec<&mut dyn Policy<G>> = vec![policy];
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

fn print_usage() {
    println!("usage:");
    println!("  gameengine list");
    println!(
        "  gameengine play <game> [--seed N] [--max-steps N] [--policy human|random|first|script:...]"
    );
    println!("  gameengine replay <game> [--seed N] [--max-steps N] [--policy script:...]");
    println!("  gameengine validate");
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
