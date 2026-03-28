use chrono::Local;
use shadow_ui_core::scene::{RoundedRect, Scene, TextAlign, TextBlock, TextWeight};

use crate::{color, layout, layout::CounterTarget, model::CounterModel};

pub fn build_scene(model: &CounterModel) -> Scene {
    let width = layout::WINDOW_WIDTH as f32;
    let height = layout::WINDOW_HEIGHT as f32;
    let top_bar = layout::top_bar_frame(width);
    let home_button = layout::home_button_frame();
    let body_card = layout::body_card_frame(width, height);
    let accent_card = layout::accent_card_frame(width);
    let tap_button = layout::tap_button_frame(width, height);
    let home_color = match button_fill(model, CounterTarget::Home) {
        ButtonFill::Base => color::SURFACE_BUTTON,
        ButtonFill::Hover => color::SURFACE_BUTTON_HOVER,
        ButtonFill::Pressed => color::SURFACE_BUTTON_ACTIVE,
    };
    let tap_color = match button_fill(model, CounterTarget::Tap) {
        ButtonFill::Base => color::ACCENT,
        ButtonFill::Hover => color::ACCENT_HOVER,
        ButtonFill::Pressed => color::ACCENT_PRESSED,
    };
    let footer = if model.tap_pressed() {
        "Release to increment"
    } else {
        "Tap the button to count"
    };
    let subtitle = Local::now().format("%A, %B %-d").to_string();
    let count_scale = if model.count() >= 100 { 0.82 } else { 1.0 };

    Scene {
        clear_color: color::BACKGROUND,
        rects: vec![
            RoundedRect::new(
                top_bar.x,
                top_bar.y,
                top_bar.w,
                top_bar.h,
                34.0,
                color::SURFACE_TOP,
            ),
            RoundedRect::new(
                home_button.x,
                home_button.y,
                home_button.w,
                home_button.h,
                21.0,
                home_color,
            ),
            RoundedRect::new(
                body_card.x,
                body_card.y,
                body_card.w,
                body_card.h,
                46.0,
                color::SURFACE_CARD,
            ),
            RoundedRect::new(
                accent_card.x,
                accent_card.y,
                accent_card.w,
                accent_card.h,
                52.0,
                color::ACCENT_PANEL,
            ),
            RoundedRect::new(
                tap_button.x,
                tap_button.y,
                tap_button.w,
                tap_button.h,
                42.0,
                tap_color,
            ),
        ],
        texts: vec![
            TextBlock {
                content: "HOME".to_string(),
                left: home_button.x,
                top: home_button.y + 9.0,
                width: home_button.w,
                height: 24.0,
                size: 18.0,
                line_height: 22.0,
                align: TextAlign::Center,
                weight: TextWeight::Semibold,
                color: color::TEXT_PRIMARY,
            },
            TextBlock {
                content: "Counter".to_string(),
                left: 190.0,
                top: 44.0,
                width: width - 230.0,
                height: 30.0,
                size: 30.0,
                line_height: 32.0,
                align: TextAlign::Left,
                weight: TextWeight::Semibold,
                color: color::TEXT_PRIMARY,
            },
            TextBlock {
                content: "Demo app inside Shadow".to_string(),
                left: 190.0,
                top: 76.0,
                width: width - 230.0,
                height: 18.0,
                size: 15.0,
                line_height: 18.0,
                align: TextAlign::Left,
                weight: TextWeight::Normal,
                color: color::TEXT_MUTED,
            },
            TextBlock {
                content: "LIVE COUNT".to_string(),
                left: body_card.x,
                top: 164.0,
                width: body_card.w,
                height: 22.0,
                size: 18.0,
                line_height: 20.0,
                align: TextAlign::Center,
                weight: TextWeight::Semibold,
                color: color::TEXT_MUTED,
            },
            TextBlock {
                content: model.count().to_string(),
                left: accent_card.x,
                top: 278.0,
                width: accent_card.w,
                height: 118.0,
                size: 118.0 * count_scale,
                line_height: 118.0 * count_scale,
                align: TextAlign::Center,
                weight: TextWeight::Bold,
                color: color::TEXT_PRIMARY,
            },
            TextBlock {
                content: footer.to_string(),
                left: body_card.x + 40.0,
                top: 560.0,
                width: body_card.w - 80.0,
                height: 26.0,
                size: 22.0,
                line_height: 24.0,
                align: TextAlign::Center,
                weight: TextWeight::Normal,
                color: color::TEXT_MUTED,
            },
            TextBlock {
                content: subtitle,
                left: body_card.x + 40.0,
                top: 622.0,
                width: body_card.w - 80.0,
                height: 20.0,
                size: 16.0,
                line_height: 20.0,
                align: TextAlign::Center,
                weight: TextWeight::Normal,
                color: color::TEXT_MUTED,
            },
            TextBlock {
                content: if model.tap_pressed() {
                    "RELEASE".to_string()
                } else {
                    "TAP".to_string()
                },
                left: tap_button.x,
                top: tap_button.y + 26.0,
                width: tap_button.w,
                height: 30.0,
                size: 28.0,
                line_height: 30.0,
                align: TextAlign::Center,
                weight: TextWeight::Bold,
                color: color::TEXT_PRIMARY,
            },
            TextBlock {
                content: "Esc also returns home".to_string(),
                left: 24.0,
                top: height - 88.0,
                width: width - 48.0,
                height: 18.0,
                size: 15.0,
                line_height: 18.0,
                align: TextAlign::Center,
                weight: TextWeight::Normal,
                color: color::TEXT_MUTED,
            },
        ],
    }
}

enum ButtonFill {
    Base,
    Hover,
    Pressed,
}

fn button_fill(model: &CounterModel, target: CounterTarget) -> ButtonFill {
    if model.pressed_target() == Some(target) {
        ButtonFill::Pressed
    } else if model.hovered_target() == Some(target) {
        ButtonFill::Hover
    } else {
        ButtonFill::Base
    }
}
