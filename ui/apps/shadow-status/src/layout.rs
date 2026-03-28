use crate::model::{ProfileMode, StatusTarget, StatusToggle};

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

pub const WINDOW_WIDTH: u32 = 540;
pub const WINDOW_HEIGHT: u32 = 1170;

pub fn top_bar_frame(width: f32) -> Frame {
    Frame {
        x: 24.0,
        y: 24.0,
        w: width - 48.0,
        h: 86.0,
    }
}

pub fn home_button_frame() -> Frame {
    Frame {
        x: 38.0,
        y: 46.0,
        w: 112.0,
        h: 42.0,
    }
}

pub fn status_card_frame(width: f32) -> Frame {
    Frame {
        x: 24.0,
        y: 142.0,
        w: width - 48.0,
        h: 410.0,
    }
}

pub fn status_row_frame(index: usize, width: f32) -> Frame {
    let card = status_card_frame(width);
    Frame {
        x: card.x + 24.0,
        y: card.y + 82.0 + index as f32 * 102.0,
        w: card.w - 48.0,
        h: 82.0,
    }
}

pub fn toggle_track_frame(index: usize, width: f32) -> Frame {
    let row = status_row_frame(index, width);
    Frame {
        x: row.x + row.w - 116.0,
        y: row.y + 21.0,
        w: 76.0,
        h: 40.0,
    }
}

pub fn toggle_knob_frame(index: usize, width: f32, enabled: bool) -> Frame {
    let track = toggle_track_frame(index, width);
    let knob_x = if enabled {
        track.x + track.w - 34.0
    } else {
        track.x + 6.0
    };
    Frame {
        x: knob_x,
        y: track.y + 6.0,
        w: 28.0,
        h: 28.0,
    }
}

pub fn profile_card_frame(width: f32, height: f32) -> Frame {
    Frame {
        x: 24.0,
        y: 582.0,
        w: width - 48.0,
        h: height - 684.0,
    }
}

pub fn profile_chip_frame(index: usize, width: f32, height: f32) -> Frame {
    let card = profile_card_frame(width, height);
    let gap = 12.0;
    let chip_width = (card.w - 48.0 - gap * 2.0) / 3.0;
    Frame {
        x: card.x + 24.0 + index as f32 * (chip_width + gap),
        y: card.y + 118.0,
        w: chip_width,
        h: 76.0,
    }
}

pub fn hit_target(x: f32, y: f32, width: f32, height: f32) -> Option<StatusTarget> {
    if home_button_frame().contains(x, y) {
        return Some(StatusTarget::Home);
    }

    for toggle in StatusToggle::ALL {
        if status_row_frame(toggle.index(), width).contains(x, y) {
            return Some(StatusTarget::Toggle(toggle));
        }
    }

    for profile in ProfileMode::ALL {
        if profile_chip_frame(profile.index(), width, height).contains(x, y) {
            return Some(StatusTarget::Profile(profile));
        }
    }

    None
}
