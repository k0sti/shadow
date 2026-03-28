use crate::{
    app::{find_app, HOME_TILES},
    color::{
        BACKGROUND, SURFACE, SURFACE_ACCENT, SURFACE_GLASS, SURFACE_RAISED, TEXT_MUTED,
        TEXT_PRIMARY,
    },
    scene::{RoundedRect, Scene, TextAlign, TextBlock, TextWeight, HEIGHT, WIDTH},
};

use super::{Frame, Point, ShellModel, ShellStatus, Target};

const STATUS_BAR_HEIGHT: f32 = 54.0;
const CLOCK_CARD_Y: f32 = 124.0;
const CLOCK_CARD_HEIGHT: f32 = 250.0;
const APP_PANEL_Y: f32 = 420.0;
const APP_PANEL_HEIGHT: f32 = 640.0;
const APP_ICON_SIZE: f32 = 96.0;
const APP_LABEL_HEIGHT: f32 = 24.0;
const GRID_COLUMNS: usize = 4;

pub(super) fn build_scene(model: &ShellModel, status: &ShellStatus) -> Scene {
    let mut rects = Vec::new();
    let mut texts = Vec::new();

    rects.push(RoundedRect::new(
        0.0,
        0.0,
        WIDTH,
        HEIGHT,
        0.0,
        SURFACE.with_alpha(0.18),
    ));
    rects.push(RoundedRect::new(
        32.0,
        92.0,
        476.0,
        314.0,
        54.0,
        SURFACE.with_alpha(0.22),
    ));
    rects.push(RoundedRect::new(
        20.0,
        APP_PANEL_Y,
        500.0,
        APP_PANEL_HEIGHT,
        44.0,
        SURFACE_ACCENT.with_alpha(0.96),
    ));

    build_status_bar(&mut rects, &mut texts, status);
    build_clock(&mut rects, &mut texts, status, model.recent_app_titles());
    build_panel_header(&mut rects, &mut texts, model);
    build_app_grid(&mut rects, &mut texts, model);
    build_navigation_bar(&mut rects, model.home_indicator_active());

    Scene {
        clear_color: BACKGROUND,
        rects,
        texts,
    }
}

pub(super) fn hit_test(point: Point) -> Option<Target> {
    HOME_TILES
        .iter()
        .enumerate()
        .find_map(|(index, _)| {
            app_frame(index)
                .contains(point)
                .then_some(Target::App(index))
        })
        .or_else(|| {
            home_indicator_frame()
                .contains(point)
                .then_some(Target::HomeIndicator)
        })
}

pub(super) fn move_focus(current: usize, dx: isize, dy: isize) -> usize {
    let cols = GRID_COLUMNS as isize;
    let rows = (HOME_TILES.len() / GRID_COLUMNS) as isize;
    let col = current as isize % cols;
    let row = current as isize / cols;

    let next_col = (col + dx).clamp(0, cols - 1);
    let next_row = (row + dy).clamp(0, rows - 1);
    (next_row * cols + next_col) as usize
}

pub(super) fn wrap_index(current: usize, delta: isize) -> usize {
    let len = HOME_TILES.len() as isize;
    ((current as isize + delta).rem_euclid(len)) as usize
}

fn build_status_bar(
    rects: &mut Vec<RoundedRect>,
    texts: &mut Vec<TextBlock>,
    status: &ShellStatus,
) {
    rects.push(RoundedRect::new(
        16.0,
        14.0,
        508.0,
        STATUS_BAR_HEIGHT,
        24.0,
        SURFACE_GLASS,
    ));

    texts.push(TextBlock {
        content: status.time_label.clone(),
        left: 34.0,
        top: 27.0,
        width: 100.0,
        height: 24.0,
        size: 18.0,
        line_height: 20.0,
        align: TextAlign::Left,
        weight: TextWeight::Semibold,
        color: TEXT_PRIMARY,
    });

    let battery_fill = (status.battery_percent.min(100) as f32 / 100.0).max(0.12);
    rects.push(RoundedRect::new(
        450.0,
        29.0,
        22.0,
        12.0,
        3.0,
        TEXT_PRIMARY.with_alpha(0.18),
    ));
    rects.push(RoundedRect::new(
        473.0,
        32.0,
        4.0,
        6.0,
        2.0,
        TEXT_PRIMARY.with_alpha(0.65),
    ));
    rects.push(RoundedRect::new(
        452.0,
        31.0,
        18.0 * battery_fill,
        8.0,
        2.0,
        TEXT_PRIMARY.with_alpha(0.78),
    ));

    for index in 0..3 {
        let alpha = if index < status.wifi_strength.min(3) as usize {
            0.72
        } else {
            0.24 + index as f32 * 0.06
        };
        rects.push(RoundedRect::new(
            408.0 + index as f32 * 10.0,
            35.0 - index as f32 * 4.0,
            7.0,
            6.0 + index as f32 * 3.0,
            3.0,
            TEXT_PRIMARY.with_alpha(alpha),
        ));
    }
}

fn build_clock(
    rects: &mut Vec<RoundedRect>,
    texts: &mut Vec<TextBlock>,
    status: &ShellStatus,
    recent_titles: Vec<&'static str>,
) {
    rects.push(RoundedRect::new(
        42.0,
        CLOCK_CARD_Y,
        456.0,
        CLOCK_CARD_HEIGHT,
        46.0,
        SURFACE_RAISED.with_alpha(0.92),
    ));

    texts.push(TextBlock {
        content: status.time_label.clone(),
        left: 66.0,
        top: 168.0,
        width: 408.0,
        height: 90.0,
        size: 78.0,
        line_height: 82.0,
        align: TextAlign::Center,
        weight: TextWeight::Bold,
        color: TEXT_PRIMARY,
    });

    texts.push(TextBlock {
        content: status.date_label.clone(),
        left: 86.0,
        top: 270.0,
        width: 368.0,
        height: 28.0,
        size: 24.0,
        line_height: 28.0,
        align: TextAlign::Center,
        weight: TextWeight::Normal,
        color: TEXT_MUTED,
    });

    if !recent_titles.is_empty() {
        texts.push(TextBlock {
            content: format!("Warm apps: {}", recent_titles.join("  •  ")),
            left: 78.0,
            top: 322.0,
            width: 384.0,
            height: 22.0,
            size: 14.0,
            line_height: 18.0,
            align: TextAlign::Center,
            weight: TextWeight::Normal,
            color: TEXT_MUTED,
        });
    }
}

fn build_panel_header(
    rects: &mut Vec<RoundedRect>,
    texts: &mut Vec<TextBlock>,
    model: &ShellModel,
) {
    let (headline, detail) = match model.foreground_app() {
        Some(app_id) => {
            let app = find_app(app_id).expect("foreground app metadata");
            (
                format!("{} live", app.title),
                format!("Tap the pill to shelf it. {}.", app.subtitle),
            )
        }
        None if model.running_apps().is_empty() => (
            "Home stack".to_string(),
            "Launch an app or keep iterating with keyboard and pointer.".to_string(),
        ),
        None => (
            "Home stack".to_string(),
            format!(
                "{} app(s) warm in the background.",
                model.running_apps().len()
            ),
        ),
    };

    rects.push(RoundedRect::new(
        44.0,
        446.0,
        452.0,
        82.0,
        28.0,
        SURFACE_GLASS.with_alpha(0.78),
    ));

    texts.push(TextBlock {
        content: headline,
        left: 68.0,
        top: 464.0,
        width: 408.0,
        height: 24.0,
        size: 22.0,
        line_height: 26.0,
        align: TextAlign::Left,
        weight: TextWeight::Semibold,
        color: TEXT_PRIMARY,
    });
    texts.push(TextBlock {
        content: detail,
        left: 68.0,
        top: 494.0,
        width: 408.0,
        height: 20.0,
        size: 14.0,
        line_height: 18.0,
        align: TextAlign::Left,
        weight: TextWeight::Normal,
        color: TEXT_MUTED,
    });
}

fn build_app_grid(rects: &mut Vec<RoundedRect>, texts: &mut Vec<TextBlock>, model: &ShellModel) {
    for (index, tile) in HOME_TILES.iter().enumerate() {
        let frame = app_frame(index);
        let target = Target::App(index);
        let is_focused = model.focused_tile() == index;
        let is_hovered = model.hovered_target() == Some(target);
        let is_pressed = model.pressed_target() == Some(target);
        let is_active = model.last_activated() == Some(target);
        let app_id = tile.app_id;
        let is_running = app_id.is_some_and(|app_id| model.app_is_running(app_id));
        let is_foreground = app_id.is_some_and(|app_id| model.app_is_foreground(app_id));

        let halo_alpha = if is_pressed {
            0.34
        } else if is_foreground {
            0.26
        } else if is_active {
            0.22
        } else if is_focused {
            0.18
        } else if is_hovered {
            0.12
        } else {
            0.0
        };

        if halo_alpha > 0.0 {
            rects.push(RoundedRect::new(
                frame.x - 10.0,
                frame.y - 10.0,
                frame.w + 20.0,
                frame.h + 20.0,
                28.0,
                TEXT_PRIMARY.with_alpha(halo_alpha),
            ));
        }

        rects.push(RoundedRect::new(
            frame.x,
            frame.y,
            frame.w,
            frame.h,
            28.0,
            SURFACE_GLASS,
        ));

        let icon_scale = if is_pressed { 0.94 } else { 1.0 };
        let icon_size = APP_ICON_SIZE * icon_scale;
        let icon_x = frame.x + (frame.w - icon_size) * 0.5;
        let icon_y = frame.y + 10.0 + (APP_ICON_SIZE - icon_size) * 0.5;

        rects.push(RoundedRect::new(
            icon_x, icon_y, icon_size, icon_size, 26.0, tile.color,
        ));
        rects.push(RoundedRect::new(
            icon_x + 16.0,
            icon_y + 20.0,
            icon_size - 32.0,
            10.0,
            5.0,
            TEXT_PRIMARY.with_alpha(0.16),
        ));

        if is_running {
            rects.push(RoundedRect::new(
                frame.x + frame.w * 0.5 - 18.0,
                frame.y + APP_ICON_SIZE + 10.0,
                36.0,
                5.0,
                2.5,
                if is_foreground {
                    TEXT_PRIMARY.with_alpha(0.92)
                } else {
                    TEXT_PRIMARY.with_alpha(0.44)
                },
            ));
        }

        texts.push(TextBlock {
            content: tile.label.to_string(),
            left: frame.x,
            top: frame.y + APP_ICON_SIZE + 22.0,
            width: frame.w,
            height: APP_LABEL_HEIGHT,
            size: 17.0,
            line_height: 20.0,
            align: TextAlign::Center,
            weight: if is_foreground || is_focused {
                TextWeight::Semibold
            } else {
                TextWeight::Normal
            },
            color: TEXT_PRIMARY,
        });
    }

    texts.push(TextBlock {
        content: "Mouse, arrows, Tab, Enter. Esc/Home comes from the app side.".to_string(),
        left: 52.0,
        top: APP_PANEL_Y + APP_PANEL_HEIGHT - 42.0,
        width: 460.0,
        height: 24.0,
        size: 15.0,
        line_height: 18.0,
        align: TextAlign::Center,
        weight: TextWeight::Normal,
        color: TEXT_MUTED,
    });
}

fn build_navigation_bar(rects: &mut Vec<RoundedRect>, active: bool) {
    rects.push(RoundedRect::new(
        186.0,
        1106.0,
        168.0,
        14.0,
        7.0,
        SURFACE_GLASS.with_alpha(if active { 0.96 } else { 0.88 }),
    ));
    rects.push(RoundedRect::new(
        222.0,
        1110.0,
        96.0,
        6.0,
        3.0,
        TEXT_PRIMARY.with_alpha(if active { 0.96 } else { 0.76 }),
    ));
}

fn grid_origin() -> Point {
    Point { x: 52.0, y: 546.0 }
}

fn app_frame(index: usize) -> Frame {
    let origin = grid_origin();
    let col = index % GRID_COLUMNS;
    let row = index / GRID_COLUMNS;

    Frame {
        x: origin.x + col as f32 * 110.0,
        y: origin.y + row as f32 * 164.0,
        w: 104.0,
        h: 142.0,
    }
}

fn home_indicator_frame() -> Frame {
    Frame {
        x: 186.0,
        y: 1098.0,
        w: 168.0,
        h: 26.0,
    }
}
