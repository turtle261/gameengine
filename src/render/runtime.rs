use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::mem;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use bytemuck::{Pod, Zeroable};
#[cfg(not(target_arch = "wasm32"))]
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Cache, Color as GlyphColor, Family, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
#[cfg(not(target_arch = "wasm32"))]
use wgpu::{
    BlendState, Buffer, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Features, FragmentState,
    Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PolygonMode, PresentMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute,
    VertexBufferLayout, VertexState, VertexStepMode,
};
#[cfg(not(target_arch = "wasm32"))]
use winit::application::ApplicationHandler;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(not(target_arch = "wasm32"))]
use winit::window::{Window, WindowId};

use crate::game::Game;
use crate::policy::Policy;
use crate::session::{HistoryStore, SessionKernel};
use crate::types::{PlayerAction, Reward, StepOutcome, Tick};

#[cfg(not(target_arch = "wasm32"))]
use super::pacer::TickPacer;
use super::scene::Scene2d;
#[cfg(not(target_arch = "wasm32"))]
use super::scene::{CircleCommand, Color, LineCommand, Point2, Rect};

#[cfg(not(target_arch = "wasm32"))]
const GEOMETRY_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderMode {
    Observation,
    OracleWorld,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    pub tick_rate_hz: f64,
    pub max_catch_up_ticks: usize,
    pub vsync: bool,
    pub show_debug_overlay: bool,
    pub mode: RenderMode,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            tick_rate_hz: 12.0,
            max_catch_up_ticks: 8,
            vsync: true,
            show_debug_overlay: false,
            mode: RenderMode::Observation,
            window_width: 1024,
            window_height: 768,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameMetrics {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionCommand<A> {
    Pulse(A),
    SetContinuous(A),
    ClearContinuous,
}

pub trait ActionSink<G: Game> {
    fn submit_command(&mut self, command: ActionCommand<G::Action>);
}

pub trait TickDriver<G: Game> {
    type History: HistoryStore<G>;

    fn session(&self) -> &SessionKernel<G, Self::History>;
    fn last_outcome(&self) -> Option<&StepOutcome<G::RewardBuf>>;
    fn advance_ticks(&mut self, due_ticks: usize);
}

pub trait Presenter<G: Game> {
    fn title(&self, game: &G) -> String;

    fn preferred_window_size(&self) -> (u32, u32) {
        (960, 640)
    }

    fn on_window_event(
        &mut self,
        event: &WindowEvent,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, G>,
        actions: &mut dyn ActionSink<G>,
    );

    fn populate_scene(
        &mut self,
        scene: &mut Scene2d,
        metrics: FrameMetrics,
        view: &RenderGameView<'_, G>,
    );
}

pub trait ObservationPresenter<G: Game>: Presenter<G> {}

pub trait OraclePresenter<G: Game>: Presenter<G> {}

#[derive(Debug)]
pub(crate) struct ViewCache<G: Game> {
    tick: Tick,
    player_observation: G::PlayerObservation,
    spectator_observation: G::SpectatorObservation,
    world_view: G::WorldView,
    previous_world_view: Option<G::WorldView>,
    last_outcome: Option<StepOutcome<G::RewardBuf>>,
    is_terminal: bool,
    interpolation_alpha: f32,
}

impl<G: Game> Clone for ViewCache<G> {
    fn clone(&self) -> Self {
        Self {
            tick: self.tick,
            player_observation: self.player_observation.clone(),
            spectator_observation: self.spectator_observation.clone(),
            world_view: self.world_view.clone(),
            previous_world_view: self.previous_world_view.clone(),
            last_outcome: self.last_outcome.clone(),
            is_terminal: self.is_terminal,
            interpolation_alpha: self.interpolation_alpha,
        }
    }
}

impl<G: Game> ViewCache<G> {
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub(crate) fn from_session<H: HistoryStore<G>>(session: &SessionKernel<G, H>) -> Self {
        Self {
            tick: session.current_tick(),
            player_observation: session.player_observation(0),
            spectator_observation: session.spectator_observation(),
            world_view: session.world_view(),
            previous_world_view: None,
            last_outcome: None,
            is_terminal: session.is_terminal(),
            interpolation_alpha: 0.0,
        }
    }
}

pub struct RenderGameView<'a, G: Game> {
    game: &'a G,
    cache: &'a ViewCache<G>,
}

impl<'a, G: Game> RenderGameView<'a, G> {
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub(crate) fn from_cache(game: &'a G, cache: &'a ViewCache<G>) -> Self {
        Self { game, cache }
    }

    pub fn game(&self) -> &'a G {
        self.game
    }

    pub fn tick(&self) -> Tick {
        self.cache.tick
    }

    pub fn player_observation(&self) -> &G::PlayerObservation {
        &self.cache.player_observation
    }

    pub fn spectator_observation(&self) -> &G::SpectatorObservation {
        &self.cache.spectator_observation
    }

    pub fn world_view(&self) -> &G::WorldView {
        &self.cache.world_view
    }

    pub fn previous_world_view(&self) -> Option<&G::WorldView> {
        self.cache.previous_world_view.as_ref()
    }

    pub fn last_outcome(&self) -> Option<&StepOutcome<G::RewardBuf>> {
        self.cache.last_outcome.as_ref()
    }

    pub fn reward_for(&self, player: usize) -> Reward {
        self.last_outcome().map_or(0, |outcome| outcome.reward_for(player))
    }

    pub fn is_terminal(&self) -> bool {
        self.cache.is_terminal
    }

    pub fn interpolation_alpha(&self) -> f32 {
        self.cache.interpolation_alpha
    }
}

#[derive(Debug)]
pub struct TurnBasedDriver<G: Game, H: HistoryStore<G>> {
    session: SessionKernel<G, H>,
    pending_action: Option<G::Action>,
    last_outcome: Option<StepOutcome<G::RewardBuf>>,
}

impl<G: Game, H: HistoryStore<G>> TurnBasedDriver<G, H> {
    pub fn new(session: SessionKernel<G, H>) -> Self {
        Self {
            session,
            pending_action: None,
            last_outcome: None,
        }
    }
}

impl<G: Game, H: HistoryStore<G>> ActionSink<G> for TurnBasedDriver<G, H> {
    fn submit_command(&mut self, command: ActionCommand<G::Action>) {
        match command {
            ActionCommand::Pulse(action) | ActionCommand::SetContinuous(action) => {
                self.pending_action = Some(action);
            }
            ActionCommand::ClearContinuous => {
                self.pending_action = None;
            }
        }
    }
}

impl<G: Game, H: HistoryStore<G>> TickDriver<G> for TurnBasedDriver<G, H> {
    type History = H;

    fn session(&self) -> &SessionKernel<G, H> {
        &self.session
    }

    fn last_outcome(&self) -> Option<&StepOutcome<G::RewardBuf>> {
        self.last_outcome.as_ref()
    }

    fn advance_ticks(&mut self, due_ticks: usize) {
        if due_ticks == 0 || self.session.is_terminal() {
            return;
        }
        let Some(action) = self.pending_action.take() else {
            return;
        };
        let actions = [PlayerAction { player: 0, action }];
        self.last_outcome = Some(self.session.step(&actions).clone());
    }
}

#[derive(Debug)]
pub struct RealtimeDriver<G: Game, H: HistoryStore<G>> {
    session: SessionKernel<G, H>,
    neutral_action: G::Action,
    continuous_action: Option<G::Action>,
    pulse_action: Option<G::Action>,
    last_outcome: Option<StepOutcome<G::RewardBuf>>,
}

impl<G: Game, H: HistoryStore<G>> RealtimeDriver<G, H> {
    pub fn new(session: SessionKernel<G, H>, neutral_action: G::Action) -> Self {
        Self {
            session,
            neutral_action,
            continuous_action: None,
            pulse_action: None,
            last_outcome: None,
        }
    }
}

impl<G: Game, H: HistoryStore<G>> ActionSink<G> for RealtimeDriver<G, H> {
    fn submit_command(&mut self, command: ActionCommand<G::Action>) {
        match command {
            ActionCommand::Pulse(action) => {
                self.pulse_action = Some(action);
            }
            ActionCommand::SetContinuous(action) => {
                self.continuous_action = Some(action);
            }
            ActionCommand::ClearContinuous => {
                self.continuous_action = None;
            }
        }
    }
}

impl<G: Game, H: HistoryStore<G>> TickDriver<G> for RealtimeDriver<G, H> {
    type History = H;

    fn session(&self) -> &SessionKernel<G, H> {
        &self.session
    }

    fn last_outcome(&self) -> Option<&StepOutcome<G::RewardBuf>> {
        self.last_outcome.as_ref()
    }

    fn advance_ticks(&mut self, due_ticks: usize) {
        for _ in 0..due_ticks {
            if self.session.is_terminal() {
                break;
            }
            let action = self
                .pulse_action
                .take()
                .or(self.continuous_action)
                .unwrap_or(self.neutral_action);
            let actions = [PlayerAction { player: 0, action }];
            self.last_outcome = Some(self.session.step(&actions).clone());
        }
    }
}

#[derive(Debug)]
pub struct PassivePolicyDriver<G: Game, H: HistoryStore<G>, P: Policy<G>> {
    session: SessionKernel<G, H>,
    policy: P,
    last_outcome: Option<StepOutcome<G::RewardBuf>>,
}

impl<G: Game, H: HistoryStore<G>, P: Policy<G>> PassivePolicyDriver<G, H, P> {
    pub fn new(session: SessionKernel<G, H>, policy: P) -> Self {
        Self {
            session,
            policy,
            last_outcome: None,
        }
    }
}

impl<G: Game, H: HistoryStore<G>, P: Policy<G>> ActionSink<G>
    for PassivePolicyDriver<G, H, P>
{
    fn submit_command(&mut self, _command: ActionCommand<G::Action>) {}
}

impl<G: Game, H: HistoryStore<G>, P: Policy<G>> TickDriver<G>
    for PassivePolicyDriver<G, H, P>
{
    type History = H;

    fn session(&self) -> &SessionKernel<G, H> {
        &self.session
    }

    fn last_outcome(&self) -> Option<&StepOutcome<G::RewardBuf>> {
        self.last_outcome.as_ref()
    }

    fn advance_ticks(&mut self, due_ticks: usize) {
        for _ in 0..due_ticks {
            if self.session.is_terminal() {
                break;
            }
            let mut policies: [&mut dyn Policy<G>; 1] = [&mut self.policy];
            self.last_outcome = Some(self.session.step_with_policies(&mut policies).clone());
        }
    }
}

#[derive(Debug)]
pub struct RenderError {
    message: String,
}

impl RenderError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for RenderError {}

pub struct RendererApp<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> {
    config: RenderConfig,
    driver: D,
    presenter: P,
    _marker: std::marker::PhantomData<G>,
}

impl<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> RendererApp<G, D, P> {
    pub fn new(config: RenderConfig, driver: D, presenter: P) -> Self {
        Self {
            config,
            driver,
            presenter,
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<G: Game + 'static, D: TickDriver<G> + ActionSink<G> + 'static, P: Presenter<G> + 'static>
    RendererApp<G, D, P>
{
    pub fn run_native(self) -> Result<(), RenderError> {
        let event_loop = EventLoop::new().map_err(|error| RenderError::new(error.to_string()))?;
        let mut app = NativeApp::new(self.config, self.driver, self.presenter);
        event_loop
            .run_app(&mut app)
            .map_err(|error| RenderError::new(error.to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
impl<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> RendererApp<G, D, P> {
    pub fn run_native(self) -> Result<(), RenderError> {
        let RendererApp {
            config,
            driver,
            presenter,
            _marker,
        } = self;
        let _ = (config, driver, presenter);
        Err(RenderError::new(
            "native window rendering is not available on wasm32 targets",
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeApp<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> {
    config: RenderConfig,
    driver: Option<D>,
    presenter: Option<P>,
    window_state: Option<WindowState<G, D, P>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> NativeApp<G, D, P> {
    fn new(config: RenderConfig, driver: D, presenter: P) -> Self {
        Self {
            config,
            driver: Some(driver),
            presenter: Some(presenter),
            window_state: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> ApplicationHandler
    for NativeApp<G, D, P>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_state.is_some() {
            return;
        }

        let driver = self.driver.take().expect("driver already taken");
        let presenter = self.presenter.take().expect("presenter already taken");
        let (width, height) = presenter.preferred_window_size();
        let title = presenter.title(driver.session().game());
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(title)
                        .with_inner_size(LogicalSize::new(width as f64, height as f64)),
                )
                .expect("failed to create render window"),
        );
        let state = pollster::block_on(WindowState::new(window, self.config, driver, presenter))
            .expect("failed to initialize render backend");
        self.window_state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.window_state else {
            return;
        };
        if window_id != state.window_id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = state.redraw() {
                    eprintln!("{error}");
                    event_loop.exit();
                }
            }
            other => {
                state.handle_window_event(&other);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.window_state {
            state.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct WindowState<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> {
    config: RenderConfig,
    pacer: TickPacer,
    driver: D,
    presenter: P,
    cache: ViewCache<G>,
    scene: Scene2d,
    gpu: GpuState,
}

#[cfg(not(target_arch = "wasm32"))]
impl<G: Game, D: TickDriver<G> + ActionSink<G>, P: Presenter<G>> WindowState<G, D, P> {
    async fn new(
        window: Arc<Window>,
        config: RenderConfig,
        driver: D,
        presenter: P,
    ) -> Result<Self, RenderError> {
        let gpu = GpuState::new(window, config).await?;
        let cache = ViewCache::from_session(driver.session());
        Ok(Self {
            config,
            pacer: TickPacer::new(config.tick_rate_hz, config.max_catch_up_ticks),
            driver,
            presenter,
            cache,
            scene: Scene2d::default(),
            gpu,
        })
    }

    fn window_id(&self) -> WindowId {
        self.gpu.window.id()
    }

    fn request_redraw(&self) {
        self.gpu.window.request_redraw();
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.gpu.resize(size);
    }

    fn metrics(&self) -> FrameMetrics {
        FrameMetrics {
            width: self.gpu.surface_config.width,
            height: self.gpu.surface_config.height,
            scale_factor: self.gpu.window.scale_factor(),
        }
    }

    fn refresh_cache(&mut self, preserve_previous_world: bool) {
        if preserve_previous_world {
            self.cache.previous_world_view = Some(self.cache.world_view.clone());
        }
        let session = self.driver.session();
        self.cache.tick = session.current_tick();
        self.cache.player_observation = session.player_observation(0);
        self.cache.spectator_observation = session.spectator_observation();
        self.cache.world_view = session.world_view();
        self.cache.last_outcome = self.driver.last_outcome().cloned();
        self.cache.is_terminal = session.is_terminal();
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        let metrics = self.metrics();
        let snapshot = self.cache.clone();
        let game_ptr = self.driver.session().game() as *const G;
        // SAFETY:
        // `Session` stores the immutable game definition for the lifetime of the driver. The
        // mutable borrow below only updates controller state, not the game definition itself.
        let view = unsafe { RenderGameView::from_cache(&*game_ptr, &snapshot) };
        self.presenter
            .on_window_event(event, metrics, &view, &mut self.driver);
        self.request_redraw();
    }

    fn redraw(&mut self) -> Result<(), RenderError> {
        let due_ticks = self.pacer.consume_due_ticks(Instant::now());
        if due_ticks > 0 {
            self.driver.advance_ticks(due_ticks);
            self.refresh_cache(true);
        } else {
            self.cache.last_outcome = self.driver.last_outcome().cloned();
            self.cache.is_terminal = self.driver.session().is_terminal();
        }
        self.cache.interpolation_alpha = self.pacer.interpolation_alpha();

        self.scene.clear();
        let metrics = self.metrics();
        let snapshot = self.cache.clone();
        let game_ptr = self.driver.session().game() as *const G;
        // SAFETY:
        // The presenter only reads the immutable game definition through the view.
        let view = unsafe { RenderGameView::from_cache(&*game_ptr, &snapshot) };
        self.presenter
            .populate_scene(&mut self.scene, metrics, &view);
        if self.config.show_debug_overlay {
            add_debug_overlay(&mut self.scene, self.config.mode, &view, metrics);
        }
        self.gpu.render(&self.scene)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn add_debug_overlay<G: Game>(
    scene: &mut Scene2d,
    mode: RenderMode,
    view: &RenderGameView<'_, G>,
    metrics: FrameMetrics,
) {
    let panel = Rect::new(16.0, metrics.height as f32 - 108.0, 280.0, 92.0);
    scene.panel(
        panel,
        Color::from_rgba8(12, 17, 24, 230),
        Some((Color::from_rgb8(90, 124, 164), 2.0)),
        90,
    );
    let mode_label = match mode {
        RenderMode::Observation => "observation",
        RenderMode::OracleWorld => "oracle",
    };
    scene.text(
        Point2::new(panel.x + 12.0, panel.y + 12.0),
        panel,
        format!(
            "mode={mode_label}\ntick={}\nreward={}\nterminal={}",
            view.tick(),
            view.reward_for(0),
            view.is_terminal()
        ),
        18.0,
        Color::WHITE,
        100,
    );
}

#[cfg(not(target_arch = "wasm32"))]
struct GpuState {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    geometry_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_capacity: usize,
    staging_vertices: Vec<GpuVertex>,
    font_system: glyphon::FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffers: Vec<GlyphBuffer>,
    window: Arc<Window>,
}

#[cfg(not(target_arch = "wasm32"))]
impl GpuState {
    async fn new(window: Arc<Window>, config: RenderConfig) -> Result<Self, RenderError> {
        let instance = Instance::new(&InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| RenderError::new(error.to_string()))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..RequestAdapterOptions::default()
            })
            .await
            .map_err(|error| RenderError::new(error.to_string()))?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                required_features: Features::empty(),
                ..DeviceDescriptor::default()
            })
            .await
            .map_err(|error| RenderError::new(error.to_string()))?;

        let capabilities = surface.get_capabilities(&adapter);
        let surface_format = capabilities
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let size = window.inner_size();
        let mut surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if config.vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            alpha_mode: capabilities
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(CompositeAlphaMode::Opaque),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("gameengine-render-geometry"),
            source: ShaderSource::Wgsl(GEOMETRY_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("gameengine-render-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let geometry_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("gameengine-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBufferLayout {
                    array_stride: mem::size_of::<GpuVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: mem::size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertex_capacity = 4096;
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("gameengine-render-vertices"),
            size: vertex_capacity as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);
        let text_buffers = Vec::with_capacity(16);
        surface_config.width = surface_config.width.max(1);
        surface_config.height = surface_config.height.max(1);

        // Touch the font system once so the first frame does not pay the entire setup cost.
        let _ = GlyphBuffer::new(&mut font_system, Metrics::new(18.0, 22.0));

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            geometry_pipeline,
            vertex_buffer,
            vertex_capacity,
            staging_vertices: Vec::with_capacity(2048),
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffers,
            window,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn render(&mut self, scene: &Scene2d) -> Result<(), RenderError> {
        if self.surface_config.width == 0 || self.surface_config.height == 0 {
            return Ok(());
        }

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        self.prepare_text(scene)?;
        self.prepare_geometry(scene);
        self.ensure_vertex_capacity()?;

        if !self.staging_vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.staging_vertices));
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.surface
                    .get_current_texture()
                    .map_err(|error| RenderError::new(error.to_string()))?
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(RenderError::new("wgpu surface is out of memory"));
            }
            Err(wgpu::SurfaceError::Other) => {
                return Err(RenderError::new("wgpu surface returned an unknown error"));
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("gameengine-render-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("gameengine-render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: scene.clear_color.r as f64,
                            g: scene.clear_color.g as f64,
                            b: scene.clear_color.b as f64,
                            a: scene.clear_color.a as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !self.staging_vertices.is_empty() {
                pass.set_pipeline(&self.geometry_pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..self.staging_vertices.len() as u32, 0..1);
            }
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .map_err(|error| RenderError::new(error.to_string()))?;
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(())
    }

    fn prepare_text(&mut self, scene: &Scene2d) -> Result<(), RenderError> {
        while self.text_buffers.len() < scene.texts.len() {
            self.text_buffers.push(GlyphBuffer::new(
                &mut self.font_system,
                Metrics::new(18.0, 22.0),
            ));
        }

        let mut texts = scene.texts.clone();
        texts.sort_by_key(|text| text.layer);
        for (index, text) in texts.iter().enumerate() {
            let buffer = &mut self.text_buffers[index];
            *buffer = GlyphBuffer::new(
                &mut self.font_system,
                Metrics::new(text.size, text.size * 1.25),
            );
            buffer.set_size(
                &mut self.font_system,
                Some(text.bounds.width.max(1.0)),
                Some(text.bounds.height.max(1.0)),
            );
            buffer.set_text(
                &mut self.font_system,
                &text.content,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
        }

        let mut text_areas = Vec::with_capacity(scene.texts.len());
        for (index, text) in texts.iter().enumerate() {
            // SAFETY:
            // Each loop iteration accesses a distinct buffer slot by index, so the returned mutable
            // references do not alias each other while `text_areas` is alive for the immediate
            // `prepare` call below.
            let buffer = unsafe { &mut *self.text_buffers.as_mut_ptr().add(index) };
            text_areas.push(TextArea {
                buffer,
                left: text.position.x,
                top: text.position.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: text.bounds.left() as i32,
                    top: text.bounds.top() as i32,
                    right: text.bounds.right() as i32,
                    bottom: text.bounds.bottom() as i32,
                },
                default_color: to_glyph_color(text.color),
                custom_glyphs: &[],
            });
        }

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .map_err(|error| RenderError::new(error.to_string()))
    }

    fn prepare_geometry(&mut self, scene: &Scene2d) {
        self.staging_vertices.clear();
        let mut geometry = Vec::with_capacity(
            scene.panels.len() + scene.lines.len() + scene.circles.len() + scene.textured_quads.len(),
        );
        for panel in &scene.panels {
            geometry.push(GeometryPrimitive::Panel(panel));
        }
        for textured in &scene.textured_quads {
            geometry.push(GeometryPrimitive::TexturedQuad(textured));
        }
        for line in &scene.lines {
            geometry.push(GeometryPrimitive::Line(line));
        }
        for circle in &scene.circles {
            geometry.push(GeometryPrimitive::Circle(circle));
        }
        geometry.sort_by_key(GeometryPrimitive::layer);

        for primitive in geometry {
            match primitive {
                GeometryPrimitive::Panel(panel) => {
                    push_rect(&mut self.staging_vertices, panel.rect, panel.fill, self.surface_config.width, self.surface_config.height);
                    if let Some((stroke, thickness)) = panel.stroke {
                        push_stroked_rect(
                            &mut self.staging_vertices,
                            panel.rect,
                            stroke,
                            thickness,
                            self.surface_config.width,
                            self.surface_config.height,
                        );
                    }
                }
                GeometryPrimitive::TexturedQuad(quad) => {
                    // The render layer keeps the textured-quad command available for future sprite
                    // pipelines. Until a texture atlas is bound, it degrades to a tinted panel.
                    push_rect(
                        &mut self.staging_vertices,
                        quad.rect,
                        quad.tint,
                        self.surface_config.width,
                        self.surface_config.height,
                    );
                }
                GeometryPrimitive::Line(line) => push_line(
                    &mut self.staging_vertices,
                    *line,
                    self.surface_config.width,
                    self.surface_config.height,
                ),
                GeometryPrimitive::Circle(circle) => push_circle(
                    &mut self.staging_vertices,
                    *circle,
                    self.surface_config.width,
                    self.surface_config.height,
                ),
            }
        }
    }

    fn ensure_vertex_capacity(&mut self) -> Result<(), RenderError> {
        let required = self.staging_vertices.len() * mem::size_of::<GpuVertex>();
        if required <= self.vertex_capacity {
            return Ok(());
        }
        self.vertex_capacity = required.next_power_of_two();
        self.vertex_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("gameengine-render-vertices"),
            size: self.vertex_capacity as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
enum GeometryPrimitive<'a> {
    Panel(&'a super::scene::PanelRegion),
    TexturedQuad(&'a super::scene::TexturedQuad),
    Line(&'a LineCommand),
    Circle(&'a CircleCommand),
}

#[cfg(not(target_arch = "wasm32"))]
impl GeometryPrimitive<'_> {
    fn layer(&self) -> i32 {
        match self {
            Self::Panel(panel) => panel.layer,
            Self::TexturedQuad(quad) => quad.layer,
            Self::Line(line) => line.layer,
            Self::Circle(circle) => circle.layer,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[cfg(not(target_arch = "wasm32"))]
fn push_rect(
    out: &mut Vec<GpuVertex>,
    rect: Rect,
    color: Color,
    width: u32,
    height: u32,
) {
    let top_left = to_ndc(rect.left(), rect.top(), width, height);
    let top_right = to_ndc(rect.right(), rect.top(), width, height);
    let bottom_left = to_ndc(rect.left(), rect.bottom(), width, height);
    let bottom_right = to_ndc(rect.right(), rect.bottom(), width, height);
    let color = [color.r, color.g, color.b, color.a];
    out.extend_from_slice(&[
        GpuVertex {
            position: top_left,
            color,
        },
        GpuVertex {
            position: bottom_left,
            color,
        },
        GpuVertex {
            position: top_right,
            color,
        },
        GpuVertex {
            position: top_right,
            color,
        },
        GpuVertex {
            position: bottom_left,
            color,
        },
        GpuVertex {
            position: bottom_right,
            color,
        },
    ]);
}

#[cfg(not(target_arch = "wasm32"))]
fn push_stroked_rect(
    out: &mut Vec<GpuVertex>,
    rect: Rect,
    color: Color,
    thickness: f32,
    width: u32,
    height: u32,
) {
    push_rect(out, Rect::new(rect.x, rect.y, rect.width, thickness), color, width, height);
    push_rect(
        out,
        Rect::new(rect.x, rect.bottom() - thickness, rect.width, thickness),
        color,
        width,
        height,
    );
    push_rect(out, Rect::new(rect.x, rect.y, thickness, rect.height), color, width, height);
    push_rect(
        out,
        Rect::new(rect.right() - thickness, rect.y, thickness, rect.height),
        color,
        width,
        height,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn push_line(out: &mut Vec<GpuVertex>, line: LineCommand, width: u32, height: u32) {
    let dx = line.end.x - line.start.x;
    let dy = line.end.y - line.start.y;
    let length = (dx * dx + dy * dy).sqrt().max(0.0001);
    let nx = -dy / length * (line.thickness * 0.5);
    let ny = dx / length * (line.thickness * 0.5);
    let rect = [
        Point2::new(line.start.x + nx, line.start.y + ny),
        Point2::new(line.start.x - nx, line.start.y - ny),
        Point2::new(line.end.x + nx, line.end.y + ny),
        Point2::new(line.end.x - nx, line.end.y - ny),
    ];
    let color = [line.color.r, line.color.g, line.color.b, line.color.a];
    out.extend_from_slice(&[
        GpuVertex {
            position: to_ndc(rect[0].x, rect[0].y, width, height),
            color,
        },
        GpuVertex {
            position: to_ndc(rect[1].x, rect[1].y, width, height),
            color,
        },
        GpuVertex {
            position: to_ndc(rect[2].x, rect[2].y, width, height),
            color,
        },
        GpuVertex {
            position: to_ndc(rect[2].x, rect[2].y, width, height),
            color,
        },
        GpuVertex {
            position: to_ndc(rect[1].x, rect[1].y, width, height),
            color,
        },
        GpuVertex {
            position: to_ndc(rect[3].x, rect[3].y, width, height),
            color,
        },
    ]);
}

#[cfg(not(target_arch = "wasm32"))]
fn push_circle(out: &mut Vec<GpuVertex>, circle: CircleCommand, width: u32, height: u32) {
    let color = [circle.color.r, circle.color.g, circle.color.b, circle.color.a];
    let center = to_ndc(circle.center.x, circle.center.y, width, height);
    let segments = 24usize;
    for index in 0..segments {
        let start = index as f32 / segments as f32 * std::f32::consts::TAU;
        let end = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let p1 = Point2::new(
            circle.center.x + circle.radius * start.cos(),
            circle.center.y + circle.radius * start.sin(),
        );
        let p2 = Point2::new(
            circle.center.x + circle.radius * end.cos(),
            circle.center.y + circle.radius * end.sin(),
        );
        out.extend_from_slice(&[
            GpuVertex {
                position: center,
                color,
            },
            GpuVertex {
                position: to_ndc(p1.x, p1.y, width, height),
                color,
            },
            GpuVertex {
                position: to_ndc(p2.x, p2.y, width, height),
                color,
            },
        ]);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn to_ndc(x: f32, y: f32, width: u32, height: u32) -> [f32; 2] {
    [
        (x / width as f32) * 2.0 - 1.0,
        1.0 - (y / height as f32) * 2.0,
    ]
}

#[cfg(not(target_arch = "wasm32"))]
fn to_glyph_color(color: Color) -> GlyphColor {
    GlyphColor::rgba(
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        (color.a * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ActionCommand, ActionSink, FrameMetrics, PassivePolicyDriver, RenderConfig, RenderGameView,
        RenderMode, TickDriver, TurnBasedDriver, ViewCache,
    };
    use crate::buffer::FixedVec;
    use crate::game::Game;
    use crate::policy::FirstLegalPolicy;
    use crate::rng::DeterministicRng;
    use crate::session::Session;
    use crate::types::{PlayerAction, PlayerId, PlayerReward, Seed, StepOutcome, Termination};

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct CounterGame;

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    struct CounterState {
        value: u8,
        terminal: bool,
    }

    impl Game for CounterGame {
        type State = CounterState;
        type Action = u8;
        type PlayerObservation = CounterState;
        type SpectatorObservation = CounterState;
        type WorldView = CounterState;
        type PlayerBuf = FixedVec<PlayerId, 1>;
        type ActionBuf = FixedVec<u8, 2>;
        type JointActionBuf = FixedVec<PlayerAction<u8>, 1>;
        type RewardBuf = FixedVec<PlayerReward, 1>;
        type WordBuf = FixedVec<u64, 1>;

        fn name(&self) -> &'static str {
            "render-counter"
        }

        fn player_count(&self) -> usize {
            1
        }

        fn init(&self, _seed: Seed) -> Self::State {
            CounterState::default()
        }

        fn is_terminal(&self, state: &Self::State) -> bool {
            state.terminal
        }

        fn players_to_act(&self, state: &Self::State, out: &mut Self::PlayerBuf) {
            out.clear();
            if !state.terminal {
                out.push(0).unwrap();
            }
        }

        fn legal_actions(&self, _state: &Self::State, _player: PlayerId, out: &mut Self::ActionBuf) {
            out.clear();
            out.push(0).unwrap();
            out.push(1).unwrap();
        }

        fn observe_player(&self, state: &Self::State, _player: PlayerId) -> Self::PlayerObservation {
            *state
        }

        fn observe_spectator(&self, state: &Self::State) -> Self::SpectatorObservation {
            *state
        }

        fn world_view(&self, state: &Self::State) -> Self::WorldView {
            *state
        }

        fn step_in_place(
            &self,
            state: &mut Self::State,
            joint_actions: &Self::JointActionBuf,
            _rng: &mut DeterministicRng,
            out: &mut StepOutcome<Self::RewardBuf>,
        ) {
            let delta = if joint_actions.is_empty() {
                0
            } else {
                joint_actions.as_slice()[0].action
            };
            state.value = state.value.saturating_add(delta);
            state.terminal = state.value >= 2;
            out.rewards
                .push(PlayerReward {
                    player: 0,
                    reward: i64::from(delta),
                })
                .unwrap();
            out.termination = if state.terminal {
                Termination::Terminal { winner: Some(0) }
            } else {
                Termination::Ongoing
            };
        }
    }

    #[test]
    fn turn_based_driver_submits_pending_action_once() {
        let session = Session::new(CounterGame, 1);
        let mut driver = TurnBasedDriver::new(session);
        driver.submit_command(ActionCommand::Pulse(1));
        driver.advance_ticks(3);
        assert_eq!(driver.session().current_tick(), 1);
    }

    #[test]
    fn passive_policy_driver_matches_headless_progress() {
        let mut driver = PassivePolicyDriver::new(Session::new(CounterGame, 4), FirstLegalPolicy);
        driver.advance_ticks(2);
        assert!(driver.session().current_tick() >= 1);
    }

    #[test]
    fn render_config_defaults_to_observation_mode() {
        let config = RenderConfig::default();
        assert_eq!(config.mode, RenderMode::Observation);
        let _ = FrameMetrics::default();
    }

    #[test]
    fn render_view_reports_last_reward() {
        let mut driver = TurnBasedDriver::new(Session::new(CounterGame, 1));
        driver.submit_command(ActionCommand::Pulse(1));
        driver.advance_ticks(1);
        let cache = ViewCache {
            tick: driver.session().current_tick(),
            player_observation: driver.session().player_observation(0),
            spectator_observation: driver.session().spectator_observation(),
            world_view: driver.session().world_view(),
            previous_world_view: None,
            last_outcome: driver.last_outcome().cloned(),
            is_terminal: driver.session().is_terminal(),
            interpolation_alpha: 0.0,
        };
        let view = RenderGameView::from_cache(driver.session().game(), &cache);
        assert_eq!(view.reward_for(0), 1);
    }
}
