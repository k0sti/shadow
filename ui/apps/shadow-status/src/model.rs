use crate::layout;

#[derive(Clone, Copy, Debug)]
pub enum StatusAction {
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusButtonState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusToggle {
    Mesh,
    Relay,
    Quiet,
}

impl StatusToggle {
    pub const ALL: [Self; 3] = [Self::Mesh, Self::Relay, Self::Quiet];

    pub fn index(self) -> usize {
        match self {
            Self::Mesh => 0,
            Self::Relay => 1,
            Self::Quiet => 2,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Mesh => "Mesh radio",
            Self::Relay => "Relay push",
            Self::Quiet => "Quiet boot",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Mesh => "Peer discovery and local sync",
            Self::Relay => "Background relay wakeups",
            Self::Quiet => "Reduce screen and haptic noise",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileMode {
    Street,
    Travel,
    Offline,
}

impl ProfileMode {
    pub const ALL: [Self; 3] = [Self::Street, Self::Travel, Self::Offline];

    pub fn index(self) -> usize {
        match self {
            Self::Street => 0,
            Self::Travel => 1,
            Self::Offline => 2,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Street => "Street",
            Self::Travel => "Travel",
            Self::Offline => "Offline",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Street => "Balanced radios and live sync.",
            Self::Travel => "Battery bias with selective updates.",
            Self::Offline => "Local-first, radios mostly quiet.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTarget {
    Home,
    Toggle(StatusToggle),
    Profile(ProfileMode),
}

impl StatusTarget {
    pub const ORDERED: [Self; 7] = [
        Self::Home,
        Self::Toggle(StatusToggle::Mesh),
        Self::Toggle(StatusToggle::Relay),
        Self::Toggle(StatusToggle::Quiet),
        Self::Profile(ProfileMode::Street),
        Self::Profile(ProfileMode::Travel),
        Self::Profile(ProfileMode::Offline),
    ];
}

pub struct StatusModel {
    toggles: [bool; 3],
    profile: ProfileMode,
    cursor: Option<(f32, f32)>,
    hovered: Option<StatusTarget>,
    pressed: Option<StatusTarget>,
    focused: StatusTarget,
    banner: String,
}

impl Default for StatusModel {
    fn default() -> Self {
        Self {
            toggles: [true, true, false],
            profile: ProfileMode::Street,
            cursor: None,
            hovered: None,
            pressed: None,
            focused: StatusTarget::Toggle(StatusToggle::Mesh),
            banner: "Three channels checked. Street profile holding.".to_string(),
        }
    }
}

impl StatusModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn profile(&self) -> ProfileMode {
        self.profile
    }

    pub fn banner(&self) -> &str {
        &self.banner
    }

    pub fn hovered_target(&self) -> Option<StatusTarget> {
        self.hovered
    }

    pub fn pressed_target(&self) -> Option<StatusTarget> {
        self.pressed
    }

    pub fn focused_target(&self) -> StatusTarget {
        self.focused
    }

    pub fn toggle_enabled(&self, toggle: StatusToggle) -> bool {
        self.toggles[toggle.index()]
    }

    pub fn active_count(&self) -> usize {
        self.toggles.into_iter().filter(|enabled| *enabled).count()
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.cursor = Some((x, y));
        self.hovered = layout::hit_target(x, y, width, height);
    }

    pub fn pointer_left(&mut self) {
        self.cursor = None;
        self.hovered = None;
        self.pressed = None;
    }

    pub fn pointer_button(
        &mut self,
        state: StatusButtonState,
        width: f32,
        height: f32,
    ) -> Option<StatusAction> {
        match state {
            StatusButtonState::Pressed => {
                self.pressed = self
                    .cursor
                    .and_then(|(x, y)| layout::hit_target(x, y, width, height));
                if let Some(target) = self.pressed {
                    self.focused = target;
                }
                None
            }
            StatusButtonState::Released => {
                let hovered = self
                    .cursor
                    .and_then(|(x, y)| layout::hit_target(x, y, width, height));
                let pressed = self.pressed.take();
                self.hovered = hovered;

                match (pressed, hovered) {
                    (Some(lhs), Some(rhs)) if lhs == rhs => self.apply_target(rhs),
                    _ => None,
                }
            }
        }
    }

    pub fn focus_next(&mut self) {
        self.focused = shift_focus(self.focused, 1);
    }

    pub fn focus_previous(&mut self) {
        self.focused = shift_focus(self.focused, -1);
    }

    pub fn focus_horizontal(&mut self, delta: i32) {
        if let StatusTarget::Profile(profile) = self.focused {
            let index = profile.index() as i32 + delta;
            let index = index.clamp(0, ProfileMode::ALL.len() as i32 - 1) as usize;
            self.focused = StatusTarget::Profile(ProfileMode::ALL[index]);
        }
    }

    pub fn activate_pressed(&mut self) {
        self.pressed = Some(self.focused);
    }

    pub fn activate_released(&mut self) -> Option<StatusAction> {
        if let Some(target) = self.pressed.take() {
            return self.apply_target(target);
        }

        None
    }

    pub fn close_action(&mut self) -> StatusAction {
        self.cancel_press();
        StatusAction::Close
    }

    pub fn cancel_press(&mut self) {
        self.pressed = None;
    }

    fn apply_target(&mut self, target: StatusTarget) -> Option<StatusAction> {
        self.focused = target;

        match target {
            StatusTarget::Home => Some(StatusAction::Close),
            StatusTarget::Toggle(toggle) => {
                let slot = &mut self.toggles[toggle.index()];
                *slot = !*slot;
                self.banner = if *slot {
                    format!(
                        "{} enabled. {} systems live.",
                        toggle.title(),
                        self.active_count()
                    )
                } else {
                    format!(
                        "{} paused. {} systems live.",
                        toggle.title(),
                        self.active_count()
                    )
                };
                None
            }
            StatusTarget::Profile(profile) => {
                self.profile = profile;
                self.banner = format!("{} profile loaded. {}", profile.title(), profile.summary());
                None
            }
        }
    }
}

fn shift_focus(current: StatusTarget, delta: i32) -> StatusTarget {
    let index = StatusTarget::ORDERED
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0) as i32;
    let next = (index + delta).rem_euclid(StatusTarget::ORDERED.len() as i32) as usize;
    StatusTarget::ORDERED[next]
}
