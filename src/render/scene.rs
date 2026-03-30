#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba8(r, g, b, 255)
    }

    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

impl Point2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn left(self) -> f32 {
        self.x
    }

    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn top(self) -> f32 {
        self.y
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn center(self) -> Point2 {
        Point2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn contains(self, point: Point2) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.top()
            && point.y <= self.bottom()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextureHandle(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelRegion {
    pub rect: Rect,
    pub fill: Color,
    pub stroke: Option<(Color, f32)>,
    pub layer: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineCommand {
    pub start: Point2,
    pub end: Point2,
    pub thickness: f32,
    pub color: Color,
    pub layer: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircleCommand {
    pub center: Point2,
    pub radius: f32,
    pub color: Color,
    pub layer: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameText {
    pub position: Point2,
    pub bounds: Rect,
    pub content: String,
    pub size: f32,
    pub color: Color,
    pub layer: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexturedQuad {
    pub rect: Rect,
    pub uv_rect: Rect,
    pub texture: TextureHandle,
    pub tint: Color,
    pub layer: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRegion {
    pub id: u64,
    pub rect: Rect,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene2d {
    pub clear_color: Color,
    pub panels: Vec<PanelRegion>,
    pub lines: Vec<LineCommand>,
    pub circles: Vec<CircleCommand>,
    pub texts: Vec<FrameText>,
    pub textured_quads: Vec<TexturedQuad>,
    pub hit_regions: Vec<HitRegion>,
}

impl Default for Scene2d {
    fn default() -> Self {
        Self::with_capacities(16, 32, 16, 16, 8, 16)
    }
}

impl Scene2d {
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

    pub fn clear(&mut self) {
        self.clear_color = Color::BLACK;
        self.panels.clear();
        self.lines.clear();
        self.circles.clear();
        self.texts.clear();
        self.textured_quads.clear();
        self.hit_regions.clear();
    }

    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    pub fn panel(&mut self, rect: Rect, fill: Color, stroke: Option<(Color, f32)>, layer: i32) {
        self.panels.push(PanelRegion {
            rect,
            fill,
            stroke,
            layer,
        });
    }

    pub fn line(&mut self, start: Point2, end: Point2, thickness: f32, color: Color, layer: i32) {
        self.lines.push(LineCommand {
            start,
            end,
            thickness,
            color,
            layer,
        });
    }

    pub fn circle(&mut self, center: Point2, radius: f32, color: Color, layer: i32) {
        self.circles.push(CircleCommand {
            center,
            radius,
            color,
            layer,
        });
    }

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
