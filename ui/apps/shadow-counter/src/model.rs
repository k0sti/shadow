#![allow(dead_code)]

use crate::layout::{self, CounterTarget};

#[derive(Clone, Copy, Debug)]
pub enum CounterAction {
    Close,
}

#[derive(Clone, Copy, Debug)]
pub enum CounterButtonState {
    Pressed,
    Released,
}

#[derive(Default)]
pub struct CounterModel {
    count: u64,
    cursor: Option<(f32, f32)>,
    hovered: Option<CounterTarget>,
    pressed: Option<CounterTarget>,
}

impl CounterModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn hovered_target(&self) -> Option<CounterTarget> {
        self.hovered
    }

    pub fn pressed_target(&self) -> Option<CounterTarget> {
        self.pressed
    }

    pub fn tap_pressed(&self) -> bool {
        self.pressed == Some(CounterTarget::Tap)
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.cursor = Some((x, y));
        self.hovered = layout::hit_target(width, height, x, y);
    }

    pub fn pointer_left(&mut self) {
        self.cursor = None;
        self.hovered = None;
        self.pressed = None;
    }

    pub fn pointer_button(
        &mut self,
        state: CounterButtonState,
        width: f32,
        height: f32,
    ) -> Option<CounterAction> {
        match state {
            CounterButtonState::Pressed => {
                self.pressed = self
                    .cursor
                    .and_then(|(x, y)| layout::hit_target(width, height, x, y));
                if self.pressed == Some(CounterTarget::Tap) {
                    self.count = self.count.saturating_add(1);
                }
                None
            }
            CounterButtonState::Released => {
                let hovered = self
                    .cursor
                    .and_then(|(x, y)| layout::hit_target(width, height, x, y));
                self.pressed = None;
                self.hovered = hovered;
                None
            }
        }
    }

    pub fn activate_pressed(&mut self) {
        self.pressed = Some(CounterTarget::Tap);
        self.count = self.count.saturating_add(1);
    }

    pub fn activate_released(&mut self) {
        self.pressed = None;
    }

    pub fn close_action(&mut self) -> CounterAction {
        self.cancel_press();
        CounterAction::Close
    }

    pub fn cancel_press(&mut self) {
        self.pressed = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{CounterButtonState, CounterModel};
    use crate::layout;

    #[test]
    fn tap_counts_on_press() {
        let mut model = CounterModel::new();
        let button =
            layout::tap_button_frame(layout::WINDOW_WIDTH as f32, layout::WINDOW_HEIGHT as f32);
        model.pointer_moved(
            button.x + button.w * 0.5,
            button.y + button.h * 0.5,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );

        model.pointer_button(
            CounterButtonState::Pressed,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );

        assert_eq!(model.count(), 1);
        assert!(model.tap_pressed());
    }

    #[test]
    fn release_does_not_double_count() {
        let mut model = CounterModel::new();
        let button =
            layout::tap_button_frame(layout::WINDOW_WIDTH as f32, layout::WINDOW_HEIGHT as f32);
        let center_x = button.x + button.w * 0.5;
        let center_y = button.y + button.h * 0.5;
        model.pointer_moved(
            center_x,
            center_y,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );
        model.pointer_button(
            CounterButtonState::Pressed,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );
        model.pointer_moved(
            center_x,
            center_y,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );

        model.pointer_button(
            CounterButtonState::Released,
            layout::WINDOW_WIDTH as f32,
            layout::WINDOW_HEIGHT as f32,
        );

        assert_eq!(model.count(), 1);
        assert!(!model.tap_pressed());
    }
}
