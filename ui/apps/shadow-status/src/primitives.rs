#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    rgba: [u8; 4],
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    pub fn linear_rgba(self) -> [f32; 4] {
        [
            srgb_to_linear(self.rgba[0]),
            srgb_to_linear(self.rgba[1]),
            srgb_to_linear(self.rgba[2]),
            self.rgba[3] as f32 / 255.0,
        ]
    }

    pub fn rgba8(self) -> [u8; 4] {
        self.rgba
    }
}

fn srgb_to_linear(channel: u8) -> f32 {
    let srgb = channel as f32 / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub color: Color,
}

impl RoundedRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32, radius: f32, color: Color) -> Self {
        Self {
            x,
            y,
            width,
            height,
            radius,
            color,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TextAlign {
    Left,
    Center,
}

#[derive(Clone, Copy, Debug)]
pub enum TextWeight {
    Normal,
    Semibold,
    Bold,
}

#[derive(Clone, Debug)]
pub struct TextBlock {
    pub content: String,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub size: f32,
    pub line_height: f32,
    pub align: TextAlign,
    pub weight: TextWeight,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub clear_color: Color,
    pub rects: Vec<RoundedRect>,
    pub texts: Vec<TextBlock>,
}
