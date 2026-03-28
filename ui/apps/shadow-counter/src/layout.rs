use shadow_ui_core::scene::{HEIGHT, WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterTarget {
    Home,
    Tap,
}

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

pub const WINDOW_WIDTH: u32 = WIDTH as u32;
pub const WINDOW_HEIGHT: u32 = HEIGHT as u32;

pub fn top_bar_frame(width: f32) -> Frame {
    Frame {
        x: 24.0,
        y: 24.0,
        w: width - 48.0,
        h: 74.0,
    }
}

pub fn home_button_frame() -> Frame {
    Frame {
        x: 38.0,
        y: 40.0,
        w: 128.0,
        h: 42.0,
    }
}

pub fn body_card_frame(width: f32, height: f32) -> Frame {
    Frame {
        x: 24.0,
        y: 132.0,
        w: width - 48.0,
        h: height - 228.0,
    }
}

pub fn accent_card_frame(width: f32) -> Frame {
    Frame {
        x: 58.0,
        y: 214.0,
        w: width - 116.0,
        h: 284.0,
    }
}

pub fn tap_button_frame(width: f32, height: f32) -> Frame {
    let button_width = (width - 200.0).clamp(220.0, 292.0);
    Frame {
        x: (width - button_width) * 0.5,
        y: height - 228.0,
        w: button_width,
        h: 88.0,
    }
}

pub fn hit_target(width: f32, height: f32, x: f32, y: f32) -> Option<CounterTarget> {
    if home_button_frame().contains(x, y) {
        return Some(CounterTarget::Home);
    }

    tap_button_frame(width, height)
        .contains(x, y)
        .then_some(CounterTarget::Tap)
}
