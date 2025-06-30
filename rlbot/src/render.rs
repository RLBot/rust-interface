use rlbot_flat::flat::{Color, Line3D, PolyLine3D, Rect2D, Rect3D, RenderAnchor, RenderGroup, RenderMessage, String2D, String3D, TextHAlign, TextVAlign, Vector3};

pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };
pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
pub const GREEN: Color = Color { r: 0, g: 128, b: 0, a: 255 };
pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
pub const LIME: Color = Color { r: 0, g: 255, b: 0, a: 255 };
pub const YELLOW: Color = Color { r: 255, g: 255, b: 0, a: 255 };
pub const ORANGE: Color = Color { r: 255, g: 128, b: 0, a: 255 };
pub const CYAN: Color = Color { r: 0, g: 255, b: 255, a: 255 };
pub const PINK: Color = Color { r: 255, g: 0, b: 255, a: 255 };
pub const PURPLE: Color = Color { r: 128, g: 0, b: 128, a: 255 };
pub const TEAL: Color = Color { r: 0, g: 128, b: 128, a: 255 };

pub struct Renderer {
    pub group: RenderGroup,
    pub default_color: Color,
}

impl Renderer {
    pub fn new(group: i32, default_color: Color) -> Self {
        Self { group: RenderGroup { render_messages: vec![], id: group }, default_color }
    }

    pub fn build(self) -> RenderGroup {
        self.group
    }

    pub fn push(&mut self, message: impl Into<RenderMessage>) {
        self.group.render_messages.push(message.into());
    }

    pub fn line_3d(&mut self, start: impl Into<RenderAnchor>, end: impl Into<RenderAnchor>, color: Option<Color>) {
        self.group.render_messages.push(
            Line3D {
                start: Box::new(start.into()),
                end: Box::new(end.into()),
                color: color.unwrap_or(self.default_color),
            }.into()
        );
    }

    pub fn polyline_3d(&mut self, points: impl IntoIterator<Item = impl Into<Vector3>>, color: Option<Color>) {
        self.group.render_messages.push(PolyLine3D {
            points: points.into_iter().map(|p| p.into()).collect(),
            color: color.unwrap_or(self.default_color),
        }.into());
    }

    pub fn string_2d(&mut self, str: String2D) {
        self.group.render_messages.push(str.into());
    }

    pub fn string_3d(&mut self, str: String3D) {
        self.group.render_messages.push(str.into());
    }

    pub fn rect_2d(&mut self, rect: Rect2D) {
        self.group.render_messages.push(rect.into());
    }

    pub fn rect_3d(&mut self, rect: Rect3D) {
        self.group.render_messages.push(rect.into());
    }
}