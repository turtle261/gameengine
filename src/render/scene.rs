//! Immediate-mode 2D scene command structures used by the renderer.

/// RGBA color in normalized `[0, 1]` channels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
    /// Alpha channel.
    pub a: f32,
}

impl Color {
    /// Opaque white color.
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    /// Opaque black color.
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);

    /// Creates an opaque RGB color.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Creates an RGBA color.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque color from 8-bit channels.
    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba8(r, g, b, 255)
    }

    /// Creates a color from 8-bit channels.
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

/// 2D point in screen space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

impl Point2 {
    /// Creates a point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Axis-aligned rectangle in screen space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left coordinate.
    pub x: f32,
    /// Top coordinate.
    pub y: f32,
    /// Rectangle width.
    pub width: f32,
    /// Rectangle height.
    pub height: f32,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns left edge.
    pub fn left(self) -> f32 {
        self.x
    }

    /// Returns right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Returns top edge.
    pub fn top(self) -> f32 {
        self.y
    }

    /// Returns bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// Returns rectangle center.
    pub fn center(self) -> Point2 {
        Point2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Returns whether `point` lies inside the rectangle bounds.
    pub fn contains(self, point: Point2) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.top()
            && point.y <= self.bottom()
    }
}

/// Handle to a texture resource.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextureHandle(pub u32);

/// Filled panel draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRegion {
    /// Panel rectangle.
    pub rect: Rect,
    /// Fill color.
    pub fill: Color,
    /// Optional stroke `(color, thickness)`.
    pub stroke: Option<(Color, f32)>,
    /// Layer ordering key.
    pub layer: i32,
}

/// Thick line draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineCommand {
    /// Start point.
    pub start: Point2,
    /// End point.
    pub end: Point2,
    /// Line thickness in pixels.
    pub thickness: f32,
    /// Line color.
    pub color: Color,
    /// Layer ordering key.
    pub layer: i32,
}

/// Filled circle draw command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircleCommand {
    /// Circle center.
    pub center: Point2,
    /// Circle radius in pixels.
    pub radius: f32,
    /// Fill color.
    pub color: Color,
    /// Layer ordering key.
    pub layer: i32,
}

/// Text draw command.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameText {
    /// Anchor position for text layout.
    pub position: Point2,
    /// Text clipping/layout bounds.
    pub bounds: Rect,
    /// UTF-8 content.
    pub content: String,
    /// Font size in pixels.
    pub size: f32,
    /// Text color.
    pub color: Color,
    /// Layer ordering key.
    pub layer: i32,
}

/// Textured rectangle command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexturedQuad {
    /// Destination rectangle.
    pub rect: Rect,
    /// Source UV rectangle.
    pub uv_rect: Rect,
    /// Texture handle.
    pub texture: TextureHandle,
    /// Multiplicative tint color.
    pub tint: Color,
    /// Layer ordering key.
    pub layer: i32,
}

/// Input hit-test region metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRegion {
    /// Stable region id.
    pub id: u64,
    /// Hit-test bounds.
    pub rect: Rect,
    /// Debug label.
    pub label: &'static str,
}

/// Full frame scene command buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene2d {
    /// Clear color for the frame.
    pub clear_color: Color,
    /// Panel commands.
    pub panels: Vec<PanelRegion>,
    /// Line commands.
    pub lines: Vec<LineCommand>,
    /// Circle commands.
    pub circles: Vec<CircleCommand>,
    /// Text commands.
    pub texts: Vec<FrameText>,
    /// Textured quad commands.
    pub textured_quads: Vec<TexturedQuad>,
    /// Hit regions for interaction logic.
    pub hit_regions: Vec<HitRegion>,
}

impl Default for Scene2d {
    fn default() -> Self {
        Self::with_capacities(16, 32, 16, 16, 8, 16)
    }
}

impl Scene2d {
    /// Creates a scene with explicit command-buffer capacities.
    pub fn with_capacities(
        panels: usize,
        lines: usize,
        circles: usize,
        texts: usize,
        textured_quads: usize,
        hit_regions: usize,
    ) -> Self {
        Self {
            clear_color: Color::BLACK,
            panels: Vec::with_capacity(panels),
            lines: Vec::with_capacity(lines),
            circles: Vec::with_capacity(circles),
            texts: Vec::with_capacity(texts),
            textured_quads: Vec::with_capacity(textured_quads),
            hit_regions: Vec::with_capacity(hit_regions),
        }
    }

    /// Clears all commands while preserving allocated capacities.
    pub fn clear(&mut self) {
        self.clear_color = Color::BLACK;
        self.panels.clear();
        self.lines.clear();
        self.circles.clear();
        self.texts.clear();
        self.textured_quads.clear();
        self.hit_regions.clear();
    }

    /// Sets frame clear color.
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Enqueues a filled panel command.
    pub fn panel(&mut self, rect: Rect, fill: Color, stroke: Option<(Color, f32)>, layer: i32) {
        self.panels.push(PanelRegion {
            rect,
            fill,
            stroke,
            layer,
        });
    }

    /// Enqueues a thick line command.
    pub fn line(&mut self, start: Point2, end: Point2, thickness: f32, color: Color, layer: i32) {
        self.lines.push(LineCommand {
            start,
            end,
            thickness,
            color,
            layer,
        });
    }

    /// Enqueues a filled circle command.
    pub fn circle(&mut self, center: Point2, radius: f32, color: Color, layer: i32) {
        self.circles.push(CircleCommand {
            center,
            radius,
            color,
            layer,
        });
    }

    /// Enqueues a text command.
    pub fn text(
        &mut self,
        position: Point2,
        bounds: Rect,
        content: impl Into<String>,
        size: f32,
        color: Color,
        layer: i32,
    ) {
        self.texts.push(FrameText {
            position,
            bounds,
            content: content.into(),
            size,
            color,
            layer,
        });
    }

    /// Enqueues a textured-quad command.
    pub fn textured_quad(
        &mut self,
        rect: Rect,
        uv_rect: Rect,
        texture: TextureHandle,
        tint: Color,
        layer: i32,
    ) {
        self.textured_quads.push(TexturedQuad {
            rect,
            uv_rect,
            texture,
            tint,
            layer,
        });
    }

    /// Registers a hit-test region.
    pub fn hit_region(&mut self, id: u64, rect: Rect, label: &'static str) {
        self.hit_regions.push(HitRegion { id, rect, label });
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, Point2, Rect, Scene2d, TextureHandle};

    #[test]
    fn scene_buffers_clear_and_reuse_capacity() {
        let mut scene = Scene2d::default();
        scene.set_clear_color(Color::WHITE);
        scene.panel(Rect::new(0.0, 0.0, 10.0, 10.0), Color::BLACK, None, 0);
        scene.line(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            2.0,
            Color::WHITE,
            1,
        );
        scene.circle(Point2::new(4.0, 4.0), 2.0, Color::WHITE, 2);
        scene.text(
            Point2::new(2.0, 3.0),
            Rect::new(0.0, 0.0, 20.0, 20.0),
            "hello",
            16.0,
            Color::WHITE,
            3,
        );
        scene.textured_quad(
            Rect::new(0.0, 0.0, 4.0, 4.0),
            Rect::new(0.0, 0.0, 1.0, 1.0),
            TextureHandle(7),
            Color::WHITE,
            0,
        );
        scene.hit_region(9, Rect::new(0.0, 0.0, 1.0, 1.0), "cell");
        let line_capacity = scene.lines.capacity();
        scene.clear();
        assert_eq!(scene.lines.len(), 0);
        assert_eq!(scene.lines.capacity(), line_capacity);
        assert_eq!(scene.clear_color, Color::BLACK);
    }
}
