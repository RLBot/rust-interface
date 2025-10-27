use rlbot_flat::flat::{
    Color, Line3D, PolyLine3D, Rect2D, Rect3D, RenderAnchor, RenderGroup, RenderMessage, String2D,
    String3D, TextHAlign, TextVAlign, Vector3,
};

#[rustfmt::skip]
pub mod colors {
    use rlbot_flat::flat::Color;
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
}

/// The Renderer allows of easy construction of [`RenderGroup`]s for in-game
/// debug rendering. When done, call [`build`] and queue the resulting
/// [`RenderGroup`] in the packet queue.
///
/// [`build`]: Renderer::build
///
/// Example:
/// ```ignore
/// use rlbot::render::{Renderer};
/// use rlbot::render::colors::{BLUE, GREEN, RED};
///
/// let mut draw = Renderer::new(0);
///
/// draw.line_3d(car.pos, car.pos + car.forward() * 120., RED);
/// draw.line_3d(car.pos, car.pos + car.rightward() * 120., GREEN);
/// draw.line_3d(car.pos, car.pos + car.upward() * 120., BLUE);
///
/// packet_queue.push(draw.build());
/// ```
pub struct Renderer {
    pub group: RenderGroup,
}

impl Renderer {
    /// Create a new Renderer.
    /// Each render group must have a unique id.
    /// Re-using an id will result in overwriting (watch out when using hiveminds).
    pub fn new(group_id: i32) -> Self {
        Self {
            group: RenderGroup {
                render_messages: vec![],
                id: group_id,
            },
        }
    }

    /// Get the resulting [RenderGroup].
    pub fn build(self) -> RenderGroup {
        self.group
    }

    /// Add a [RenderMessage] to this group.
    pub fn push(&mut self, message: impl Into<RenderMessage>) {
        self.group.render_messages.push(message.into());
    }

    /// Draws a line between two anchors in 3d space.
    pub fn line_3d(
        &mut self,
        start: impl Into<RenderAnchor>,
        end: impl Into<RenderAnchor>,
        color: Color,
    ) {
        self.group.render_messages.push(
            Line3D {
                start: Box::new(start.into()),
                end: Box::new(end.into()),
                color,
            }
            .into(),
        );
    }

    /// Draws a line going through each of the provided points.
    pub fn polyline_3d(
        &mut self,
        points: impl IntoIterator<Item = impl Into<Vector3>>,
        color: Color,
    ) {
        self.group.render_messages.push(
            PolyLine3D {
                points: points.into_iter().map(|p| p.into()).collect(),
                color,
            }
            .into(),
        );
    }

    /// Draws text in 2d space.
    /// X and y uses screen-space coordinates, i.e. 0.1 is 10% of the screen width/height.
    /// Use `set_resolution` to change to pixel coordinates.
    /// Characters of the font are 20 pixels tall and 10 pixels wide when `scale == 1.0`.
    /// Consider using [push] and `..default()` when using multiple default values.
    pub fn string_2d(
        &mut self,
        text: String,
        x: f32,
        y: f32,
        scale: f32,
        foreground: Color,
        background: Color,
        h_align: TextHAlign,
        v_align: TextVAlign,
    ) {
        self.group.render_messages.push(
            String2D {
                text,
                x,
                y,
                scale,
                foreground,
                background,
                h_align,
                v_align,
            }
            .into(),
        );
    }

    /// Draws text anchored in 3d space.
    /// Characters of the font are 20 pixels tall and 10 pixels wide when `scale == 1.0`.
    /// Consider using [push] and `..default()` when using multiple default values.
    pub fn string_3d(
        &mut self,
        text: String,
        anchor: impl Into<RenderAnchor>,
        scale: f32,
        foreground: Color,
        background: Color,
        h_align: TextHAlign,
        v_align: TextVAlign,
    ) {
        self.group.render_messages.push(
            String3D {
                text,
                anchor: Box::new(anchor.into()),
                scale,
                foreground,
                background,
                h_align,
                v_align,
            }
            .into(),
        );
    }

    /// Draws a rectangle anchored in 2d space.
    /// X, y, width, and height uses screen-space coordinates, i.e. 0.1 is 10% of the screen width/height.
    /// Use `set_resolution` to change to pixel coordinates.
    /// Consider using [push] and `..default()` when using multiple default values.
    pub fn rect_2d(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        h_align: TextHAlign,
        v_align: TextVAlign,
    ) {
        self.group.render_messages.push(
            Rect2D {
                x,
                y,
                width,
                height,
                color,
                h_align,
                v_align,
            }
            .into(),
        );
    }

    /// Draws a rectangle anchored in 3d space.
    /// Width and height are screen-space sizes, i.e. 0.1 is 10% of the screen width/height.
    /// Use `set_resolution` to change to pixel coordinates.
    /// The size does not change based on distance to the camera.
    /// Consider using [push] and `..default()` when using multiple default values.
    pub fn rect_3d(
        &mut self,
        anchor: impl Into<RenderAnchor>,
        width: f32,
        height: f32,
        color: Color,
        h_align: TextHAlign,
        v_align: TextVAlign,
    ) {
        self.group.render_messages.push(
            Rect3D {
                anchor: Box::new(anchor.into()),
                width,
                height,
                color,
                h_align,
                v_align,
            }
            .into(),
        );
    }
}
