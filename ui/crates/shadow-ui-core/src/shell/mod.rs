mod view;

use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

use crate::{
    app::{find_app, AppId, HOME_TILES},
    scene::Scene,
};

const PRESS_FLASH: Duration = Duration::from_millis(160);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavAction {
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
    Activate,
    Home,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButtonState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShellEvent {
    PointerMoved { x: f32, y: f32 },
    PointerLeft,
    PointerButton(PointerButtonState),
    Navigate(NavAction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellAction {
    Launch { app_id: AppId },
    Home,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellStatus {
    pub time_label: String,
    pub date_label: String,
    pub battery_percent: u8,
    pub wifi_strength: u8,
}

impl ShellStatus {
    pub fn demo(now: DateTime<Local>) -> Self {
        Self {
            time_label: now.format("%H:%M").to_string(),
            date_label: now.format("%A, %B %-d").to_string(),
            battery_percent: 78,
            wifi_strength: 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    App(usize),
    HomeIndicator,
}

pub struct ShellModel {
    cursor: Option<Point>,
    hovered_target: Option<Target>,
    pressed_target: Option<Target>,
    focused_tile: usize,
    last_activated: Option<(Target, Instant)>,
    running_apps: Vec<AppId>,
    recent_apps: Vec<AppId>,
    foreground_app: Option<AppId>,
}

impl Default for ShellModel {
    fn default() -> Self {
        Self {
            cursor: None,
            hovered_target: None,
            pressed_target: None,
            focused_tile: first_launchable_tile(),
            last_activated: None,
            running_apps: Vec::new(),
            recent_apps: Vec::new(),
            foreground_app: None,
        }
    }
}

impl ShellModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&mut self, event: ShellEvent) -> Option<ShellAction> {
        match event {
            ShellEvent::PointerMoved { x, y } => {
                let point = Point { x, y };
                self.cursor = Some(point);
                self.hovered_target = view::hit_test(point);
                None
            }
            ShellEvent::PointerLeft => {
                self.cursor = None;
                self.hovered_target = None;
                self.pressed_target = None;
                None
            }
            ShellEvent::PointerButton(state) => self.pointer_button(state),
            ShellEvent::Navigate(action) => self.navigate(action),
        }
    }

    pub fn scene(&mut self, status: &ShellStatus) -> Scene {
        self.trim_expired_flash();
        view::build_scene(self, status)
    }

    pub fn set_app_running(&mut self, app_id: AppId, running: bool) {
        if running {
            if !self.running_apps.contains(&app_id) {
                self.running_apps.push(app_id);
            }
            self.touch_recent(app_id);
        } else {
            self.running_apps.retain(|candidate| *candidate != app_id);
            self.recent_apps.retain(|candidate| *candidate != app_id);
            if self.foreground_app == Some(app_id) {
                self.foreground_app = None;
            }
        }
    }

    pub fn set_foreground_app(&mut self, app_id: Option<AppId>) {
        self.foreground_app = app_id;
        if let Some(app_id) = app_id {
            self.set_app_running(app_id, true);
            if let Some(index) = tile_index_for_app(app_id) {
                self.focused_tile = index;
            }
        }
    }

    pub fn foreground_app(&self) -> Option<AppId> {
        self.foreground_app
    }

    pub fn running_apps(&self) -> &[AppId] {
        &self.running_apps
    }

    pub fn recent_apps(&self) -> &[AppId] {
        &self.recent_apps
    }

    pub(crate) fn focused_tile(&self) -> usize {
        self.focused_tile
    }

    pub(crate) fn hovered_target(&self) -> Option<Target> {
        self.hovered_target
    }

    pub(crate) fn pressed_target(&self) -> Option<Target> {
        self.pressed_target
    }

    pub(crate) fn last_activated(&self) -> Option<Target> {
        self.last_activated.map(|(target, _)| target)
    }

    pub(crate) fn app_is_running(&self, app_id: AppId) -> bool {
        self.running_apps.contains(&app_id)
    }

    pub(crate) fn app_is_foreground(&self, app_id: AppId) -> bool {
        self.foreground_app == Some(app_id)
    }

    pub(crate) fn home_indicator_active(&self) -> bool {
        self.foreground_app.is_some()
    }

    pub(crate) fn recent_app_titles(&self) -> Vec<&'static str> {
        self.recent_apps
            .iter()
            .filter_map(|app_id| find_app(*app_id).map(|app| app.title))
            .collect()
    }

    fn pointer_button(&mut self, state: PointerButtonState) -> Option<ShellAction> {
        match state {
            PointerButtonState::Pressed => {
                self.pressed_target = self.cursor.and_then(view::hit_test);
                if let Some(Target::App(index)) = self.pressed_target {
                    self.focused_tile = index;
                }
                None
            }
            PointerButtonState::Released => {
                let target = self.cursor.and_then(view::hit_test);
                let pressed = self.pressed_target.take();
                self.hovered_target = target;

                match (pressed, target) {
                    (Some(lhs), Some(rhs)) if lhs == rhs => self.activate_target(rhs),
                    _ => None,
                }
            }
        }
    }

    fn navigate(&mut self, action: NavAction) -> Option<ShellAction> {
        match action {
            NavAction::Left => self.focused_tile = view::move_focus(self.focused_tile, -1, 0),
            NavAction::Right => self.focused_tile = view::move_focus(self.focused_tile, 1, 0),
            NavAction::Up => self.focused_tile = view::move_focus(self.focused_tile, 0, -1),
            NavAction::Down => self.focused_tile = view::move_focus(self.focused_tile, 0, 1),
            NavAction::Next => self.focused_tile = view::wrap_index(self.focused_tile, 1),
            NavAction::Previous => self.focused_tile = view::wrap_index(self.focused_tile, -1),
            NavAction::Activate => return self.activate_target(Target::App(self.focused_tile)),
            NavAction::Home => return self.activate_target(Target::HomeIndicator),
        }
        None
    }

    fn activate_target(&mut self, target: Target) -> Option<ShellAction> {
        self.last_activated = Some((target, Instant::now()));

        match target {
            Target::App(index) => HOME_TILES[index].app_id.map(|app_id| {
                self.touch_recent(app_id);
                ShellAction::Launch { app_id }
            }),
            Target::HomeIndicator => self.foreground_app.is_some().then_some(ShellAction::Home),
        }
    }

    fn touch_recent(&mut self, app_id: AppId) {
        self.recent_apps.retain(|candidate| *candidate != app_id);
        self.recent_apps.insert(0, app_id);
        self.recent_apps.truncate(3);
    }

    fn trim_expired_flash(&mut self) {
        if let Some((_, instant)) = self.last_activated {
            if instant.elapsed() >= PRESS_FLASH {
                self.last_activated = None;
            }
        }
    }
}

fn first_launchable_tile() -> usize {
    HOME_TILES
        .iter()
        .enumerate()
        .find_map(|(index, tile)| tile.app_id.map(|_| index))
        .unwrap_or(0)
}

fn tile_index_for_app(app_id: AppId) -> Option<usize> {
    HOME_TILES
        .iter()
        .enumerate()
        .find_map(|(index, tile)| (tile.app_id == Some(app_id)).then_some(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{COUNTER_APP_ID, STATUS_APP_ID};

    #[test]
    fn tracks_running_and_recent_apps() {
        let mut shell = ShellModel::new();
        shell.set_app_running(COUNTER_APP_ID, true);
        shell.set_foreground_app(Some(STATUS_APP_ID));

        assert!(shell.app_is_running(COUNTER_APP_ID));
        assert!(shell.app_is_running(STATUS_APP_ID));
        assert_eq!(shell.foreground_app(), Some(STATUS_APP_ID));
        assert_eq!(shell.recent_apps(), &[STATUS_APP_ID, COUNTER_APP_ID]);
    }

    #[test]
    fn home_requires_foreground_app() {
        let mut shell = ShellModel::new();
        assert_eq!(shell.handle(ShellEvent::Navigate(NavAction::Home)), None);

        shell.set_foreground_app(Some(COUNTER_APP_ID));
        assert_eq!(
            shell.handle(ShellEvent::Navigate(NavAction::Home)),
            Some(ShellAction::Home)
        );
    }
}
