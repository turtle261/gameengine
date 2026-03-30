mod pacer;
mod runtime;
mod scene;

#[cfg(feature = "builtin-games")]
pub mod builtin;

pub use pacer::TickPacer;
pub use runtime::{
    ActionCommand, ActionSink, FrameMetrics, PassivePolicyDriver, RenderConfig, RenderError,
    RenderGameView, RenderMode, RendererApp, RealtimeDriver, TickDriver, TurnBasedDriver,
    Presenter, ObservationPresenter, OraclePresenter,
};
pub use scene::{
    CircleCommand, Color, FrameText, HitRegion, LineCommand, PanelRegion, Point2, Rect, Scene2d,
    TexturedQuad, TextureHandle,
};
