//! Static registry describing builtin games and policy metadata.

/// Policy metadata surfaced by CLI and UI.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PolicyDescriptor {
    /// Stable policy identifier.
    pub name: &'static str,
    /// Human-facing policy description.
    pub description: &'static str,
}

/// Control prompt metadata for interactive play.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ControlMap {
    /// Human input prompt shown by the CLI.
    pub prompt: &'static str,
}

/// Full static descriptor for one builtin game.
#[derive(Clone, Copy, Debug)]
pub struct GameDescriptor {
    /// Stable external game name.
    pub name: &'static str,
    /// CLI runner callback used by descriptor-driven dispatch.
    #[cfg(feature = "cli")]
    pub(crate) runner: fn(crate::cli::CliConfig, crate::cli::RunMode) -> Result<(), String>,
    /// Optional controls metadata for interactive frontends.
    pub controls: Option<&'static ControlMap>,
    /// True when the default renderer supports this game.
    pub default_renderer: bool,
    /// True when the physics renderer supports this game.
    pub physics_renderer: bool,
    /// Supported policy descriptors.
    pub policies: &'static [PolicyDescriptor],
}

const STANDARD_POLICIES: [PolicyDescriptor; 4] = [
    PolicyDescriptor {
        name: "human",
        description: "Interactive stdin policy",
    },
    PolicyDescriptor {
        name: "random",
        description: "Uniform random legal actions",
    },
    PolicyDescriptor {
        name: "first",
        description: "Always pick the first legal action",
    },
    PolicyDescriptor {
        name: "script:<a,b,c>",
        description: "Comma-separated deterministic action script",
    },
];

const TICTACTOE_CONTROLS: ControlMap = ControlMap {
    prompt: "choose move [0-8]",
};
const BLACKJACK_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [hit/stand]",
};
const COIN_FLIP_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [0/1]",
};
const BIASED_RPS_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [rock/paper/scissors]",
};
const KUHN_POKER_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [bet/pass]",
};
const EXTENDED_TIGER_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [stand/listen/open1/open2]",
};
#[cfg(feature = "physics")]
const PLATFORMER_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [stay/left/right/jump]",
};

/// Returns all builtin game descriptors enabled for the current feature set.
pub fn all_games() -> &'static [GameDescriptor] {
    #[cfg(feature = "physics")]
    {
        static GAMES: [GameDescriptor; 7] = [
            GameDescriptor {
                name: "coin_flip",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_biased_coinflip,
                controls: Some(&COIN_FLIP_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "biased_rock_paper_scissor",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_biased_rps,
                controls: Some(&BIASED_RPS_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "kuhn_poker",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_kuhn_poker,
                controls: Some(&KUHN_POKER_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "extended_tiger",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_extended_tiger,
                controls: Some(&EXTENDED_TIGER_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "tictactoe",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_tictactoe,
                controls: Some(&TICTACTOE_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "blackjack",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_blackjack,
                controls: Some(&BLACKJACK_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "platformer",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_platformer,
                controls: Some(&PLATFORMER_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: cfg!(feature = "render"),
                policies: &STANDARD_POLICIES,
            },
        ];
        &GAMES
    }

    #[cfg(not(feature = "physics"))]
    {
        static GAMES: [GameDescriptor; 6] = [
            GameDescriptor {
                name: "coin_flip",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_biased_coinflip,
                controls: Some(&COIN_FLIP_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "biased_rock_paper_scissor",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_biased_rps,
                controls: Some(&BIASED_RPS_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "kuhn_poker",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_kuhn_poker,
                controls: Some(&KUHN_POKER_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "extended_tiger",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_extended_tiger,
                controls: Some(&EXTENDED_TIGER_CONTROLS),
                default_renderer: false,
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "tictactoe",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_tictactoe,
                controls: Some(&TICTACTOE_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                name: "blackjack",
                #[cfg(feature = "cli")]
                runner: crate::cli::run_blackjack,
                controls: Some(&BLACKJACK_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
        ];
        &GAMES
    }
}

/// Finds a builtin game descriptor by stable name.
pub fn find_game(name: &str) -> Option<&'static GameDescriptor> {
    all_games()
        .iter()
        .find(|descriptor| descriptor.name == name)
}
