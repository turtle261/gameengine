//! Static registry describing builtin games and policy metadata.

/// Statically known builtin game kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GameKind {
    /// Deterministic tic-tac-toe.
    TicTacToe,
    /// Deterministic blackjack.
    Blackjack,
    /// Deterministic physics-backed platformer.
    #[cfg(feature = "physics")]
    Platformer,
}

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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GameDescriptor {
    /// Internal game discriminator.
    pub kind: GameKind,
    /// Stable external game name.
    pub name: &'static str,
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
#[cfg(feature = "physics")]
const PLATFORMER_CONTROLS: ControlMap = ControlMap {
    prompt: "choose action [stay/left/right/jump]",
};

/// Returns all builtin game descriptors enabled for the current feature set.
pub fn all_games() -> &'static [GameDescriptor] {
    #[cfg(feature = "physics")]
    {
        static GAMES: [GameDescriptor; 3] = [
            GameDescriptor {
                kind: GameKind::TicTacToe,
                name: "tictactoe",
                controls: Some(&TICTACTOE_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                kind: GameKind::Blackjack,
                name: "blackjack",
                controls: Some(&BLACKJACK_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                kind: GameKind::Platformer,
                name: "platformer",
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
        static GAMES: [GameDescriptor; 2] = [
            GameDescriptor {
                kind: GameKind::TicTacToe,
                name: "tictactoe",
                controls: Some(&TICTACTOE_CONTROLS),
                default_renderer: cfg!(feature = "render"),
                physics_renderer: false,
                policies: &STANDARD_POLICIES,
            },
            GameDescriptor {
                kind: GameKind::Blackjack,
                name: "blackjack",
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
    all_games().iter().find(|descriptor| descriptor.name == name)
}
