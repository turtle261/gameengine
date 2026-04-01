//! Builtin presenters for builtin environments.

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::builtin::{
    Blackjack, BlackjackAction, BlackjackObservation, BlackjackPhase, TicTacToe, TicTacToeAction,
    TicTacToeCell, TicTacToeObservation,
};
#[cfg(feature = "physics")]
use crate::builtin::{Platformer, PlatformerAction, PlatformerConfig, PlatformerObservation};
#[cfg(feature = "physics")]
use crate::physics::PhysicsOracleView2d;

#[cfg(feature = "physics")]
use super::OraclePresenter;
use super::scene::Color;
use super::{
    ActionCommand, ActionSink, FrameMetrics, ObservationPresenter, Point2, Presenter, Rect,
    RenderGameView, Scene2d,
};

const BG: Color = Color::from_rgb8(17, 24, 39);
const PANEL: Color = Color::from_rgb8(30, 41, 59);
const PANEL_ALT: Color = Color::from_rgb8(15, 23, 42);
const ACCENT: Color = Color::from_rgb8(56, 189, 248);
const ACCENT_WARM: Color = Color::from_rgb8(251, 191, 36);
const SUCCESS: Color = Color::from_rgb8(34, 197, 94);
#[cfg(feature = "physics")]
const DANGER: Color = Color::from_rgb8(248, 113, 113);
const TEXT: Color = Color::from_rgb8(241, 245, 249);
const MUTED: Color = Color::from_rgb8(148, 163, 184);

/// Observation presenter for tic-tac-toe.
#[derive(Clone, Copy, Debug, Default)]
pub struct TicTacToePresenter {
    cursor: Point2,
}

impl TicTacToePresenter {
    fn board_rect(metrics: FrameMetrics) -> Rect {
        let size = metrics.width.min(metrics.height) as f32 * 0.66;
        Rect::new((metrics.width as f32 - size) * 0.5, 72.0, size, size)
    }

    fn cell_rect(metrics: FrameMetrics, index: usize) -> Rect {
        let board = Self::board_rect(metrics);
        let cell = board.width / 3.0;
        let row = index / 3;
        let col = index % 3;
        Rect::new(
            board.x + col as f32 * cell,
            board.y + row as f32 * cell,
            cell,
            cell,
        )
    }

    fn cell_at(&self, metrics: FrameMetrics) -> Option<u8> {
        (0..9).find_map(|index| {
            Self::cell_rect(metrics, index)
                .contains(self.cursor)
                .then_some(index as u8)
        })
    }

    fn submit_index(&self, sink: &mut dyn ActionSink<TicTacToe>, index: u8) {
        sink.submit_command(ActionCommand::Pulse(TicTacToeAction(index)));
    }
}

impl Presenter<TicTacToe> for TicTacToePresenter {
    fn title(&self, _game: &TicTacToe) -> String {
        "gameengine :: TicTacToe".to_string()
    }

    fn preferred_window_size(&self) -> (u32, u32) {
        (860, 780)
    }

    fn on_window_event(
        &mut self,
        event: &WindowEvent,
        metrics: FrameMetrics,
        _view: &RenderGameView<'_, TicTacToe>,
        actions: &mut dyn ActionSink<TicTacToe>,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(index) = self.cell_at(metrics) {
                    self.submit_index(actions, index);
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let index =
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1)
                        | PhysicalKey::Code(KeyCode::Numpad1) => Some(0),
                        PhysicalKey::Code(KeyCode::Digit2)
                        | PhysicalKey::Code(KeyCode::Numpad2) => Some(1),
                        PhysicalKey::Code(KeyCode::Digit3)
                        | PhysicalKey::Code(KeyCode::Numpad3) => Some(2),
                        PhysicalKey::Code(KeyCode::Digit4)
                        | PhysicalKey::Code(KeyCode::Numpad4) => Some(3),
                        PhysicalKey::Code(KeyCode::Digit5)
                        | PhysicalKey::Code(KeyCode::Numpad5) => Some(4),
                        PhysicalKey::Code(KeyCode::Digit6)
                        | PhysicalKey::Code(KeyCode::Numpad6) => Some(5),
                        PhysicalKey::Code(KeyCode::Digit7)
                        | PhysicalKey::Code(KeyCode::Numpad7) => Some(6),
                        PhysicalKey::Code(KeyCode::Digit8)
                        | PhysicalKey::Code(KeyCode::Numpad8) => Some(7),
                        PhysicalKey::Code(KeyCode::Digit9)
                        | PhysicalKey::Code(KeyCode::Numpad9) => Some(8),
                        _ => None,
                    };
                if let Some(index) = index {
                    self.submit_index(actions, index);
                }
            }
            _ => {}
        }
    }

    fn populate_scene(
        &mut self,
        scene: &mut Scene2d,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, TicTacToe>,
    ) {
        scene.set_clear_color(BG);
        let observation = view.player_observation();
        let board = Self::board_rect(metrics);
        scene.panel(
            Rect::new(
                32.0,
                24.0,
                metrics.width as f32 - 64.0,
                metrics.height as f32 - 48.0,
            ),
            PANEL_ALT,
            Some((ACCENT, 2.0)),
            0,
        );
        scene.panel(board, PANEL, Some((ACCENT, 3.0)), 5);

        let cell = board.width / 3.0;
        for split in 1..3 {
            let offset = board.x + split as f32 * cell;
            scene.line(
                Point2::new(offset, board.y),
                Point2::new(offset, board.bottom()),
                4.0,
                ACCENT,
                10,
            );
        }
        for split in 1..3 {
            let offset = board.y + split as f32 * cell;
            scene.line(
                Point2::new(board.x, offset),
                Point2::new(board.right(), offset),
                4.0,
                ACCENT,
                10,
            );
        }

        for index in 0..9 {
            let cell_rect = Self::cell_rect(metrics, index);
            scene.hit_region(index as u64, cell_rect, "tictactoe-cell");
            match observation.board[index] {
                TicTacToeCell::Player => {
                    scene.line(
                        Point2::new(cell_rect.x + 18.0, cell_rect.y + 18.0),
                        Point2::new(cell_rect.right() - 18.0, cell_rect.bottom() - 18.0),
                        8.0,
                        ACCENT_WARM,
                        20,
                    );
                    scene.line(
                        Point2::new(cell_rect.right() - 18.0, cell_rect.y + 18.0),
                        Point2::new(cell_rect.x + 18.0, cell_rect.bottom() - 18.0),
                        8.0,
                        ACCENT_WARM,
                        20,
                    );
                }
                TicTacToeCell::Opponent => {
                    scene.circle(cell_rect.center(), cell_rect.width * 0.28, SUCCESS, 20);
                    scene.circle(cell_rect.center(), cell_rect.width * 0.18, PANEL, 21);
                }
                TicTacToeCell::Empty => {}
            }
        }

        scene.text(
            Point2::new(54.0, 40.0),
            Rect::new(54.0, 40.0, metrics.width as f32 - 108.0, 120.0),
            "Observation UI: click a square or press 1-9.",
            24.0,
            TEXT,
            40,
        );
        scene.text(
            Point2::new(54.0, metrics.height as f32 - 148.0),
            Rect::new(
                54.0,
                metrics.height as f32 - 148.0,
                metrics.width as f32 - 108.0,
                96.0,
            ),
            tictactoe_status(observation, view.reward_for(0), view.tick()),
            22.0,
            MUTED,
            40,
        );
    }
}

impl ObservationPresenter<TicTacToe> for TicTacToePresenter {}

/// Observation presenter for blackjack.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlackjackPresenter {
    cursor: Point2,
}

impl BlackjackPresenter {
    fn hit_button(metrics: FrameMetrics) -> Rect {
        Rect::new(64.0, metrics.height as f32 - 136.0, 180.0, 64.0)
    }

    fn stand_button(metrics: FrameMetrics) -> Rect {
        Rect::new(264.0, metrics.height as f32 - 136.0, 180.0, 64.0)
    }

    fn card_rect(origin: Point2, index: usize) -> Rect {
        Rect::new(origin.x + index as f32 * 88.0, origin.y, 72.0, 104.0)
    }

    fn submit(&self, sink: &mut dyn ActionSink<Blackjack>, action: BlackjackAction) {
        sink.submit_command(ActionCommand::Pulse(action));
    }
}

impl Presenter<Blackjack> for BlackjackPresenter {
    fn title(&self, _game: &Blackjack) -> String {
        "gameengine :: Blackjack".to_string()
    }

    fn preferred_window_size(&self) -> (u32, u32) {
        (980, 720)
    }

    fn on_window_event(
        &mut self,
        event: &WindowEvent,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, Blackjack>,
        actions: &mut dyn ActionSink<Blackjack>,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if view.player_observation().phase == BlackjackPhase::PlayerTurn
                    && !view.is_terminal()
                {
                    if Self::hit_button(metrics).contains(self.cursor) {
                        self.submit(actions, BlackjackAction::Hit);
                    } else if Self::stand_button(metrics).contains(self.cursor) {
                        self.submit(actions, BlackjackAction::Stand);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyH) => self.submit(actions, BlackjackAction::Hit),
                    PhysicalKey::Code(KeyCode::KeyS)
                    | PhysicalKey::Code(KeyCode::Enter)
                    | PhysicalKey::Code(KeyCode::Space) => {
                        self.submit(actions, BlackjackAction::Stand)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn populate_scene(
        &mut self,
        scene: &mut Scene2d,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, Blackjack>,
    ) {
        scene.set_clear_color(Color::from_rgb8(6, 24, 24));
        scene.panel(
            Rect::new(
                24.0,
                24.0,
                metrics.width as f32 - 48.0,
                metrics.height as f32 - 48.0,
            ),
            Color::from_rgb8(11, 42, 42),
            Some((ACCENT_WARM, 2.0)),
            0,
        );

        let observation = view.player_observation();
        scene.text(
            Point2::new(48.0, 38.0),
            Rect::new(48.0, 38.0, metrics.width as f32 - 96.0, 64.0),
            "Observation UI: H = hit, S/Enter = stand. Opponent information stays hidden until terminal.",
            22.0,
            TEXT,
            30,
        );

        let opponent_origin = Point2::new(64.0, 140.0);
        let player_origin = Point2::new(64.0, 360.0);
        for index in 0..observation.opponent_card_count as usize {
            let rect = Self::card_rect(opponent_origin, index);
            let hidden =
                index >= observation.opponent_visible_len as usize && !observation.terminal;
            draw_card(
                scene,
                rect,
                if hidden {
                    None
                } else {
                    Some(observation.opponent_cards[index])
                },
                if hidden { "?" } else { "" },
                if hidden {
                    PANEL_ALT
                } else {
                    Color::from_rgb8(245, 248, 255)
                },
            );
        }
        for index in 0..observation.player_len as usize {
            let rect = Self::card_rect(player_origin, index);
            draw_card(
                scene,
                rect,
                Some(observation.player_cards[index]),
                "",
                Color::from_rgb8(245, 248, 255),
            );
        }

        let hit = Self::hit_button(metrics);
        let stand = Self::stand_button(metrics);
        let active = observation.phase == BlackjackPhase::PlayerTurn && !observation.terminal;
        scene.panel(
            hit,
            if active { ACCENT } else { PANEL },
            Some((TEXT, 2.0)),
            10,
        );
        scene.panel(
            stand,
            if active { ACCENT_WARM } else { PANEL },
            Some((TEXT, 2.0)),
            10,
        );
        scene.text(hit.center(), hit, "HIT", 30.0, TEXT, 20);
        scene.text(stand.center(), stand, "STAND", 30.0, TEXT, 20);

        let status_rect = Rect::new(520.0, 118.0, metrics.width as f32 - 580.0, 540.0);
        scene.panel(status_rect, PANEL_ALT, Some((MUTED, 2.0)), 5);
        scene.text(
            Point2::new(status_rect.x + 18.0, status_rect.y + 18.0),
            status_rect,
            blackjack_status(observation, view.reward_for(0), view.tick()),
            22.0,
            TEXT,
            20,
        );
    }
}

impl ObservationPresenter<Blackjack> for BlackjackPresenter {}

#[cfg(feature = "physics")]
/// Observation presenter for platformer.
#[derive(Clone, Copy, Debug)]
pub struct PlatformerPresenter {
    /// Platformer configuration used for scene scaling.
    pub config: PlatformerConfig,
    cursor: Point2,
    left_held: bool,
    right_held: bool,
}

#[cfg(feature = "physics")]
impl Default for PlatformerPresenter {
    fn default() -> Self {
        Self {
            config: PlatformerConfig::default(),
            cursor: Point2::new(0.0, 0.0),
            left_held: false,
            right_held: false,
        }
    }
}

#[cfg(feature = "physics")]
impl PlatformerPresenter {
    fn emit_motion(&self, sink: &mut dyn ActionSink<Platformer>) {
        let command = match (self.left_held, self.right_held) {
            (true, false) => ActionCommand::SetContinuous(PlatformerAction::Left),
            (false, true) => ActionCommand::SetContinuous(PlatformerAction::Right),
            _ => ActionCommand::ClearContinuous,
        };
        sink.submit_command(command);
    }

    fn world_rect(metrics: FrameMetrics) -> Rect {
        Rect::new(
            72.0,
            96.0,
            metrics.width as f32 - 144.0,
            metrics.height as f32 - 224.0,
        )
    }

    fn unit_rect(metrics: FrameMetrics, config: PlatformerConfig, x: u8, y: u8) -> Rect {
        let world = Self::world_rect(metrics);
        let width = world.width / config.width as f32;
        let height = world.height / config.height as f32;
        Rect::new(
            world.x + x as f32 * width,
            world.y + (config.height as f32 - y as f32 - 1.0) * height,
            width,
            height,
        )
    }
}

#[cfg(feature = "physics")]
impl Presenter<Platformer> for PlatformerPresenter {
    fn title(&self, _game: &Platformer) -> String {
        "gameengine :: Platformer".to_string()
    }

    fn preferred_window_size(&self) -> (u32, u32) {
        (1180, 620)
    }

    fn on_window_event(
        &mut self,
        event: &WindowEvent,
        _metrics: FrameMetrics,
        _view: &RenderGameView<'_, Platformer>,
        actions: &mut dyn ActionSink<Platformer>,
    ) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Point2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::KeyboardInput { event, .. } => match (event.physical_key, event.state) {
                (PhysicalKey::Code(KeyCode::ArrowLeft), ElementState::Pressed)
                | (PhysicalKey::Code(KeyCode::KeyA), ElementState::Pressed) => {
                    self.left_held = true;
                    self.emit_motion(actions);
                }
                (PhysicalKey::Code(KeyCode::ArrowLeft), ElementState::Released)
                | (PhysicalKey::Code(KeyCode::KeyA), ElementState::Released) => {
                    self.left_held = false;
                    self.emit_motion(actions);
                }
                (PhysicalKey::Code(KeyCode::ArrowRight), ElementState::Pressed)
                | (PhysicalKey::Code(KeyCode::KeyD), ElementState::Pressed) => {
                    self.right_held = true;
                    self.emit_motion(actions);
                }
                (PhysicalKey::Code(KeyCode::ArrowRight), ElementState::Released)
                | (PhysicalKey::Code(KeyCode::KeyD), ElementState::Released) => {
                    self.right_held = false;
                    self.emit_motion(actions);
                }
                (PhysicalKey::Code(KeyCode::ArrowUp), ElementState::Pressed)
                | (PhysicalKey::Code(KeyCode::Space), ElementState::Pressed)
                | (PhysicalKey::Code(KeyCode::KeyW), ElementState::Pressed) => {
                    actions.submit_command(ActionCommand::Pulse(PlatformerAction::Jump));
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn populate_scene(
        &mut self,
        scene: &mut Scene2d,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, Platformer>,
    ) {
        scene.set_clear_color(Color::from_rgb8(10, 18, 36));
        let observation = view.player_observation();
        let world = Self::world_rect(metrics);
        scene.panel(world, Color::from_rgb8(22, 40, 70), Some((ACCENT, 2.0)), 0);

        let ground = Rect::new(world.x, world.bottom() - 6.0, world.width, 6.0);
        scene.panel(ground, Color::from_rgb8(74, 124, 89), None, 1);

        for (index, berry_x) in self.config.berry_xs.iter().enumerate() {
            if observation.remaining_berries & (1u8 << index) == 0 {
                continue;
            }
            let berry_rect = Self::unit_rect(metrics, self.config, *berry_x, self.config.berry_y);
            scene.circle(
                berry_rect.center(),
                berry_rect.width.min(berry_rect.height) * 0.22,
                ACCENT_WARM,
                10,
            );
        }

        let player = Self::unit_rect(metrics, self.config, observation.x, observation.y);
        scene.panel(player, ACCENT, Some((TEXT, 2.0)), 20);
        scene.text(
            Point2::new(72.0, 28.0),
            Rect::new(72.0, 28.0, metrics.width as f32 - 144.0, 54.0),
            "Observation UI: hold A/D or arrows to move, press Space to jump.",
            22.0,
            TEXT,
            30,
        );
        scene.text(
            Point2::new(72.0, metrics.height as f32 - 92.0),
            Rect::new(
                72.0,
                metrics.height as f32 - 92.0,
                metrics.width as f32 - 144.0,
                72.0,
            ),
            platformer_status(observation, view.reward_for(0), view.tick()),
            22.0,
            MUTED,
            30,
        );
    }
}

#[cfg(feature = "physics")]
impl ObservationPresenter<Platformer> for PlatformerPresenter {}

#[cfg(feature = "physics")]
/// Oracle/world presenter for platformer physics debugging.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlatformerPhysicsPresenter {
    inner: PlatformerPresenter,
}

#[cfg(feature = "physics")]
impl PlatformerPhysicsPresenter {
    /// Creates a physics presenter with explicit platformer config.
    pub fn new(config: PlatformerConfig) -> Self {
        Self {
            inner: PlatformerPresenter {
                config,
                ..PlatformerPresenter::default()
            },
        }
    }
}

#[cfg(feature = "physics")]
impl Presenter<Platformer> for PlatformerPhysicsPresenter {
    fn title(&self, _game: &Platformer) -> String {
        "gameengine :: Platformer Oracle Physics".to_string()
    }

    fn preferred_window_size(&self) -> (u32, u32) {
        (1240, 720)
    }

    fn on_window_event(
        &mut self,
        event: &WindowEvent,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, Platformer>,
        actions: &mut dyn ActionSink<Platformer>,
    ) {
        self.inner.on_window_event(event, metrics, view, actions);
    }

    fn populate_scene(
        &mut self,
        scene: &mut Scene2d,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, Platformer>,
    ) {
        scene.set_clear_color(Color::from_rgb8(14, 16, 22));
        let world = view.world_view();
        let bounds = world.physics.bounds();
        let bodies = world.physics.bodies();
        let contacts = world.physics.contacts();
        let frame = Rect::new(
            48.0,
            72.0,
            metrics.width as f32 - 96.0,
            metrics.height as f32 - 180.0,
        );
        scene.panel(frame, PANEL_ALT, Some((ACCENT_WARM, 2.0)), 0);

        for body in bodies {
            let previous_center = view
                .previous_world_view()
                .and_then(|previous| previous.physics.body(body.id))
                .map(|previous| previous.position)
                .unwrap_or(body.position);
            let alpha = view.interpolation_alpha();
            let center_x = previous_center.x.to_f64() as f32 * (1.0 - alpha)
                + body.position.x.to_f64() as f32 * alpha;
            let center_y = previous_center.y.to_f64() as f32 * (1.0 - alpha)
                + body.position.y.to_f64() as f32 * alpha;
            let min_x = center_x - body.half_extents.x.to_f64() as f32;
            let min_y = center_y - body.half_extents.y.to_f64() as f32;
            let max_x = center_x + body.half_extents.x.to_f64() as f32;
            let max_y = center_y + body.half_extents.y.to_f64() as f32;
            let rect = physics_rect(frame, bounds, min_x, min_y, max_x, max_y);
            let fill = match body.kind {
                crate::physics::BodyKind::Static => MUTED,
                crate::physics::BodyKind::Kinematic => ACCENT,
                crate::physics::BodyKind::Trigger => ACCENT_WARM,
            };
            scene.panel(rect, fill, Some((TEXT, 1.0)), 10);
        }

        for contact in contacts {
            if let (Some(a), Some(b)) =
                (world.physics.body(contact.a), world.physics.body(contact.b))
            {
                let a_rect = physics_rect(
                    frame,
                    bounds,
                    (a.position.x - a.half_extents.x).to_f64() as f32,
                    (a.position.y - a.half_extents.y).to_f64() as f32,
                    (a.position.x + a.half_extents.x).to_f64() as f32,
                    (a.position.y + a.half_extents.y).to_f64() as f32,
                );
                let b_rect = physics_rect(
                    frame,
                    bounds,
                    (b.position.x - b.half_extents.x).to_f64() as f32,
                    (b.position.y - b.half_extents.y).to_f64() as f32,
                    (b.position.x + b.half_extents.x).to_f64() as f32,
                    (b.position.y + b.half_extents.y).to_f64() as f32,
                );
                scene.line(a_rect.center(), b_rect.center(), 3.0, DANGER, 20);
            }
        }

        scene.text(
            Point2::new(56.0, 28.0),
            Rect::new(56.0, 28.0, metrics.width as f32 - 112.0, 64.0),
            "Oracle physics view: useful for debugging and understanding the environment. It may reveal more than the intended observation.",
            20.0,
            TEXT,
            30,
        );
        scene.text(
            Point2::new(56.0, metrics.height as f32 - 88.0),
            Rect::new(
                56.0,
                metrics.height as f32 - 88.0,
                metrics.width as f32 - 112.0,
                72.0,
            ),
            format!(
                "{}\nphysics bodies={} contacts={} physics_tick={}",
                platformer_status(view.player_observation(), view.reward_for(0), view.tick()),
                bodies.len(),
                contacts.len(),
                world.physics.tick()
            ),
            21.0,
            MUTED,
            30,
        );
    }
}

#[cfg(feature = "physics")]
impl OraclePresenter<Platformer> for PlatformerPhysicsPresenter {}

fn tictactoe_status(observation: &TicTacToeObservation, reward: i64, tick: u64) -> String {
    let headline = if observation.terminal {
        match observation.winner {
            Some(0) => "You won.",
            Some(1) => "Opponent won.",
            None => "Draw.",
            Some(_) => "Terminal.",
        }
    } else {
        "Your move."
    };
    format!("{headline}\nreward={reward}\ntick={tick}")
}

fn blackjack_status(observation: &BlackjackObservation, reward: i64, tick: u64) -> String {
    let phase = match observation.phase {
        BlackjackPhase::PlayerTurn => "player turn",
        BlackjackPhase::OpponentTurn => "opponent turn",
        BlackjackPhase::Terminal => "terminal",
    };
    let winner = if observation.terminal {
        match observation.winner {
            Some(0) => "player wins",
            Some(1) => "opponent wins",
            None => "push",
            Some(_) => "terminal",
        }
    } else {
        "hand in progress"
    };
    format!(
        "phase={phase}\nstatus={winner}\nplayer total={}{}\nopponent visible={} card(s)\nreward={reward}\ntick={tick}",
        observation.player_value.total,
        if observation.player_value.soft {
            " soft"
        } else {
            ""
        },
        observation.opponent_visible_len
    )
}

#[cfg(feature = "physics")]
fn platformer_status(observation: &PlatformerObservation, reward: i64, tick: u64) -> String {
    format!(
        "x={} y={}\nremaining_berries={:#08b}\nreward={reward}\ntick={tick}\nterminal={}",
        observation.x, observation.y, observation.remaining_berries, observation.terminal
    )
}

fn draw_card(scene: &mut Scene2d, rect: Rect, card: Option<u8>, fallback: &str, fill: Color) {
    scene.panel(rect, fill, Some((PANEL, 2.0)), 10);
    let label = card.map(card_label).unwrap_or_else(|| fallback.to_string());
    let label_color = if card.is_some() { PANEL } else { TEXT };
    scene.text(
        Point2::new(rect.x + 16.0, rect.y + 18.0),
        rect,
        label,
        30.0,
        label_color,
        20,
    );
}

fn card_label(card: u8) -> String {
    match card {
        1 => "A".to_string(),
        11 => "J".to_string(),
        12 => "Q".to_string(),
        13 => "K".to_string(),
        value => value.to_string(),
    }
}

#[cfg(feature = "physics")]
fn physics_rect(
    frame: Rect,
    bounds: crate::math::Aabb2<crate::math::StrictF64>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> Rect {
    let world_width = (bounds.max.x - bounds.min.x).to_f64() as f32;
    let world_height = (bounds.max.y - bounds.min.y).to_f64() as f32;
    let x_scale = frame.width / world_width.max(1.0);
    let y_scale = frame.height / world_height.max(1.0);
    Rect::new(
        frame.x + (min_x - bounds.min.x.to_f64() as f32) * x_scale,
        frame.y + frame.height - (max_y - bounds.min.y.to_f64() as f32) * y_scale,
        (max_x - min_x) * x_scale,
        (max_y - min_y) * y_scale,
    )
}

#[cfg(all(test, feature = "physics"))]
mod tests {
    use super::{
        BlackjackPresenter, PlatformerPhysicsPresenter, PlatformerPresenter, TicTacToePresenter,
    };
    use crate::builtin::{Blackjack, Platformer, TicTacToe};
    use crate::render::{
        FrameMetrics, Presenter, RealtimeDriver, RenderGameView, Scene2d, TickDriver,
        TurnBasedDriver,
    };
    use crate::session::Session;

    type TicTacToeDriver =
        TurnBasedDriver<TicTacToe, crate::session::FixedHistory<TicTacToe, 256, 32, 8>>;

    fn tictactoe_view() -> (TicTacToeDriver, FrameMetrics) {
        (
            TurnBasedDriver::new(Session::new(TicTacToe, 1)),
            FrameMetrics {
                width: 900,
                height: 700,
                scale_factor: 1.0,
            },
        )
    }

    #[test]
    fn tictactoe_presenter_emits_scene() {
        let (driver, metrics) = tictactoe_view();
        let cache = super::super::runtime::ViewCache::from_session(driver.session());
        let view = RenderGameView::from_cache(driver.session().game(), &cache);
        let mut presenter = TicTacToePresenter::default();
        let mut scene = Scene2d::default();
        presenter.populate_scene(&mut scene, metrics, &view);
        assert!(!scene.lines.is_empty());
        assert_eq!(scene.hit_regions.len(), 9);
    }

    #[test]
    fn blackjack_presenter_emits_cards() {
        let driver = TurnBasedDriver::new(Session::new(Blackjack, 1));
        let metrics = FrameMetrics {
            width: 1000,
            height: 720,
            scale_factor: 1.0,
        };
        let cache = super::super::runtime::ViewCache::from_session(driver.session());
        let view = RenderGameView::from_cache(driver.session().game(), &cache);
        let mut presenter = BlackjackPresenter::default();
        let mut scene = Scene2d::default();
        presenter.populate_scene(&mut scene, metrics, &view);
        assert!(scene.panels.len() >= 5);
        assert!(!scene.texts.is_empty());
    }

    #[test]
    fn platformer_presenters_emit_geometry() {
        let session = Session::new(Platformer::default(), 1);
        let driver = RealtimeDriver::new(session, crate::builtin::PlatformerAction::Stay);
        let metrics = FrameMetrics {
            width: 1180,
            height: 620,
            scale_factor: 1.0,
        };
        let cache = super::super::runtime::ViewCache::from_session(driver.session());
        let view = RenderGameView::from_cache(driver.session().game(), &cache);
        let mut observation_presenter = PlatformerPresenter::default();
        let mut oracle_presenter =
            PlatformerPhysicsPresenter::new(crate::builtin::PlatformerConfig::default());
        let mut observation_scene = Scene2d::default();
        let mut oracle_scene = Scene2d::default();
        observation_presenter.populate_scene(&mut observation_scene, metrics, &view);
        oracle_presenter.populate_scene(&mut oracle_scene, metrics, &view);
        assert!(!observation_scene.panels.is_empty());
        assert!(!oracle_scene.panels.is_empty());
    }
}
