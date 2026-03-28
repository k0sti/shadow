use crate::{
    color, layout,
    model::{ProfileMode, StatusModel, StatusTarget, StatusToggle},
    primitives::{Color, RoundedRect, Scene, TextAlign, TextBlock, TextWeight},
};

pub fn build_scene(model: &StatusModel) -> Scene {
    let width = layout::WINDOW_WIDTH as f32;
    let height = layout::WINDOW_HEIGHT as f32;
    let top_bar = layout::top_bar_frame(width);
    let home_button = layout::home_button_frame();
    let status_card = layout::status_card_frame(width);
    let profile_card = layout::profile_card_frame(width, height);

    let mut rects = vec![
        RoundedRect::new(
            top_bar.x,
            top_bar.y,
            top_bar.w,
            top_bar.h,
            36.0,
            color::SURFACE_TOP,
        ),
        frame_rect(status_card, 44.0, color::SURFACE_CARD),
        frame_rect(profile_card, 44.0, color::SURFACE_CARD),
    ];

    let mut texts = vec![
        label_block(
            "HOME",
            home_button,
            18.0,
            12.0,
            TextWeight::Semibold,
            color::TEXT_PRIMARY,
        ),
        TextBlock {
            content: "Status".to_string(),
            left: 186.0,
            top: 44.0,
            width: width - 218.0,
            height: 34.0,
            size: 32.0,
            line_height: 34.0,
            align: TextAlign::Left,
            weight: TextWeight::Bold,
            color: color::TEXT_PRIMARY,
        },
        TextBlock {
            content: "Field controls and profile bias".to_string(),
            left: 186.0,
            top: 80.0,
            width: width - 218.0,
            height: 18.0,
            size: 15.0,
            line_height: 18.0,
            align: TextAlign::Left,
            weight: TextWeight::Normal,
            color: color::TEXT_MUTED,
        },
        TextBlock {
            content: "SYSTEM CHANNELS".to_string(),
            left: status_card.x + 24.0,
            top: status_card.y + 28.0,
            width: status_card.w - 48.0,
            height: 22.0,
            size: 18.0,
            line_height: 20.0,
            align: TextAlign::Left,
            weight: TextWeight::Semibold,
            color: color::TEXT_MUTED,
        },
        TextBlock {
            content: format!("{} of 3 active", model.active_count()),
            left: status_card.x + 24.0,
            top: status_card.y + 52.0,
            width: status_card.w - 48.0,
            height: 18.0,
            size: 15.0,
            line_height: 18.0,
            align: TextAlign::Left,
            weight: TextWeight::Normal,
            color: color::TEXT_SOFT,
        },
        TextBlock {
            content: "PROFILE".to_string(),
            left: profile_card.x + 24.0,
            top: profile_card.y + 28.0,
            width: profile_card.w - 48.0,
            height: 22.0,
            size: 18.0,
            line_height: 20.0,
            align: TextAlign::Left,
            weight: TextWeight::Semibold,
            color: color::TEXT_MUTED,
        },
        TextBlock {
            content: model.profile().summary().to_string(),
            left: profile_card.x + 24.0,
            top: profile_card.y + 58.0,
            width: profile_card.w - 48.0,
            height: 20.0,
            size: 16.0,
            line_height: 20.0,
            align: TextAlign::Left,
            weight: TextWeight::Normal,
            color: color::TEXT_SOFT,
        },
        TextBlock {
            content: "LAST CHANGE".to_string(),
            left: profile_card.x + 24.0,
            top: profile_card.y + 230.0,
            width: profile_card.w - 48.0,
            height: 18.0,
            size: 16.0,
            line_height: 18.0,
            align: TextAlign::Left,
            weight: TextWeight::Semibold,
            color: color::TEXT_MUTED,
        },
        TextBlock {
            content: model.banner().to_string(),
            left: profile_card.x + 24.0,
            top: profile_card.y + 260.0,
            width: profile_card.w - 48.0,
            height: 48.0,
            size: 18.0,
            line_height: 22.0,
            align: TextAlign::Left,
            weight: TextWeight::Normal,
            color: color::TEXT_PRIMARY,
        },
        TextBlock {
            content: "Tab or arrows move focus. Space toggles.".to_string(),
            left: 24.0,
            top: height - 82.0,
            width: width - 48.0,
            height: 18.0,
            size: 15.0,
            line_height: 18.0,
            align: TextAlign::Center,
            weight: TextWeight::Normal,
            color: color::TEXT_MUTED,
        },
    ];

    push_focus_ring(&mut rects, model, StatusTarget::Home, home_button, 24.0);
    rects.push(frame_rect(
        home_button,
        21.0,
        button_fill(
            model,
            StatusTarget::Home,
            color::SURFACE_ROW,
            color::SURFACE_ROW_HOVER,
            color::SURFACE_ROW_ACTIVE,
        ),
    ));

    for toggle in StatusToggle::ALL {
        let row = layout::status_row_frame(toggle.index(), width);
        let track = layout::toggle_track_frame(toggle.index(), width);
        let knob = layout::toggle_knob_frame(toggle.index(), width, model.toggle_enabled(toggle));
        let target = StatusTarget::Toggle(toggle);

        push_focus_ring(&mut rects, model, target, row, 34.0);
        rects.push(frame_rect(
            row,
            30.0,
            button_fill(
                model,
                target,
                color::SURFACE_ROW,
                color::SURFACE_ROW_HOVER,
                color::SURFACE_ROW_ACTIVE,
            ),
        ));
        rects.push(frame_rect(
            track,
            20.0,
            if model.toggle_enabled(toggle) {
                color::ACCENT
            } else {
                color::ACCENT_MUTED
            },
        ));
        rects.push(frame_rect(
            knob,
            14.0,
            if model.toggle_enabled(toggle) {
                color::SURFACE_TOP
            } else {
                color::TEXT_MUTED
            },
        ));

        texts.push(TextBlock {
            content: toggle.title().to_string(),
            left: row.x + 20.0,
            top: row.y + 18.0,
            width: row.w - 148.0,
            height: 24.0,
            size: 21.0,
            line_height: 24.0,
            align: TextAlign::Left,
            weight: TextWeight::Semibold,
            color: color::TEXT_PRIMARY,
        });
        texts.push(TextBlock {
            content: toggle.subtitle().to_string(),
            left: row.x + 20.0,
            top: row.y + 46.0,
            width: row.w - 148.0,
            height: 18.0,
            size: 15.0,
            line_height: 18.0,
            align: TextAlign::Left,
            weight: TextWeight::Normal,
            color: color::TEXT_MUTED,
        });
        texts.push(TextBlock {
            content: if model.toggle_enabled(toggle) {
                "ON".to_string()
            } else {
                "OFF".to_string()
            },
            left: track.x - 56.0,
            top: track.y + 9.0,
            width: 42.0,
            height: 20.0,
            size: 16.0,
            line_height: 18.0,
            align: TextAlign::Center,
            weight: TextWeight::Semibold,
            color: if model.toggle_enabled(toggle) {
                color::ACCENT
            } else {
                color::TEXT_SOFT
            },
        });
    }

    for profile in ProfileMode::ALL {
        let chip = layout::profile_chip_frame(profile.index(), width, height);
        let target = StatusTarget::Profile(profile);
        let selected = model.profile() == profile;
        let chip_fill = if model.pressed_target() == Some(target) {
            color::SURFACE_CHIP_ACTIVE
        } else if model.hovered_target() == Some(target) {
            color::SURFACE_CHIP_HOVER
        } else if selected {
            color::SURFACE_CHIP_ACTIVE
        } else {
            color::SURFACE_CHIP
        };

        push_focus_ring(&mut rects, model, target, chip, 24.0);
        rects.push(frame_rect(chip, 28.0, chip_fill));
        texts.push(TextBlock {
            content: profile.title().to_string(),
            left: chip.x,
            top: chip.y + 18.0,
            width: chip.w,
            height: 24.0,
            size: 22.0,
            line_height: 24.0,
            align: TextAlign::Center,
            weight: TextWeight::Semibold,
            color: color::TEXT_PRIMARY,
        });
        texts.push(TextBlock {
            content: if selected {
                "ACTIVE".to_string()
            } else {
                "READY".to_string()
            },
            left: chip.x,
            top: chip.y + 46.0,
            width: chip.w,
            height: 16.0,
            size: 14.0,
            line_height: 16.0,
            align: TextAlign::Center,
            weight: TextWeight::Normal,
            color: if selected {
                color::ACCENT
            } else {
                color::TEXT_SOFT
            },
        });
    }

    Scene {
        clear_color: color::BACKGROUND,
        rects,
        texts,
    }
}

fn push_focus_ring(
    rects: &mut Vec<RoundedRect>,
    model: &StatusModel,
    target: StatusTarget,
    frame: layout::Frame,
    radius: f32,
) {
    if model.focused_target() == target {
        rects.push(frame_rect(
            layout::Frame {
                x: frame.x - 4.0,
                y: frame.y - 4.0,
                w: frame.w + 8.0,
                h: frame.h + 8.0,
            },
            radius + 4.0,
            color::FOCUS_RING,
        ));
    }
}

fn button_fill(
    model: &StatusModel,
    target: StatusTarget,
    base: Color,
    hover: Color,
    pressed: Color,
) -> Color {
    if model.pressed_target() == Some(target) {
        pressed
    } else if model.hovered_target() == Some(target) {
        hover
    } else {
        base
    }
}

fn label_block(
    content: &str,
    frame: layout::Frame,
    size: f32,
    top_offset: f32,
    weight: TextWeight,
    color: Color,
) -> TextBlock {
    TextBlock {
        content: content.to_string(),
        left: frame.x,
        top: frame.y + top_offset,
        width: frame.w,
        height: size + 4.0,
        size,
        line_height: size + 4.0,
        align: TextAlign::Center,
        weight,
        color,
    }
}

fn frame_rect(frame: layout::Frame, radius: f32, color: Color) -> RoundedRect {
    RoundedRect::new(frame.x, frame.y, frame.w, frame.h, radius, color)
}
