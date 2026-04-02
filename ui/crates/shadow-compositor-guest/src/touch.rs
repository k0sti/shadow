use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use evdev::{AbsoluteAxisCode, Device, PropType};
use smithay::reexports::calloop::channel::Sender;

#[derive(Clone, Debug)]
pub struct TouchDeviceInfo {
    pub path: PathBuf,
    pub name: String,
    pub x_min: i32,
    pub x_max: i32,
    pub y_min: i32,
    pub y_max: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchInputEvent {
    pub phase: TouchPhase,
    pub normalized_x: f64,
    pub normalized_y: f64,
    pub time_msec: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ContactSnapshot {
    tracking_id: Option<i32>,
    x: i32,
    y: i32,
}

impl ContactSnapshot {
    fn is_active(self) -> bool {
        self.tracking_id.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TouchFrameState {
    current_slot: i32,
    slot0: ContactSnapshot,
    committed: ContactSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RawTouchEvent {
    event_type: u16,
    code: u16,
    value: i32,
    time_msec: u32,
}

impl TouchFrameState {
    fn handle_event(
        &mut self,
        event: RawTouchEvent,
        device: &TouchDeviceInfo,
    ) -> Option<TouchInputEvent> {
        match (event.event_type, event.code) {
            (EV_ABS, ABS_MT_SLOT) => {
                self.current_slot = event.value;
                None
            }
            (EV_ABS, code) => {
                if self.current_slot != 0 {
                    return None;
                }

                match code {
                    ABS_MT_TRACKING_ID => {
                        self.slot0.tracking_id = (event.value >= 0).then_some(event.value);
                    }
                    ABS_MT_POSITION_X => {
                        self.slot0.x = event.value;
                    }
                    ABS_MT_POSITION_Y => {
                        self.slot0.y = event.value;
                    }
                    _ => {}
                }
                None
            }
            (EV_SYN, SYN_REPORT) => self.flush(device, event.time_msec),
            (EV_SYN, SYN_DROPPED) => {
                self.slot0 = self.committed;
                None
            }
            _ => None,
        }
    }

    fn flush(&mut self, device: &TouchDeviceInfo, time_msec: u32) -> Option<TouchInputEvent> {
        let current = self.slot0;
        let previous = self.committed;
        let (phase, x, y) = match (previous.is_active(), current.is_active()) {
            (false, true) => (TouchPhase::Down, current.x, current.y),
            (true, true) if previous.x != current.x || previous.y != current.y => {
                (TouchPhase::Move, current.x, current.y)
            }
            (true, false) => (TouchPhase::Up, previous.x, previous.y),
            _ => {
                self.committed = current;
                return None;
            }
        };

        self.committed = current;
        Some(TouchInputEvent {
            phase,
            normalized_x: normalize_axis(x, device.x_min, device.x_max),
            normalized_y: normalize_axis(y, device.y_min, device.y_max),
            time_msec,
        })
    }
}

pub fn detect_touch_device() -> Result<TouchDeviceInfo> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir("/dev/input").context("read /dev/input")? {
        let entry = entry.context("read /dev/input entry")?;
        let path = entry.path();
        if !is_event_path(&path) {
            continue;
        }
        candidates.push(path);
    }
    candidates.sort();

    for path in candidates {
        if let Ok(device) = Device::open(&path) {
            if let Some(info) = touch_device_info(&path, &device) {
                return Ok(info);
            }
        }
    }

    Err(anyhow!(
        "no direct-touch evdev node with multitouch X/Y axes was found under /dev/input"
    ))
}

pub fn spawn_touch_reader(info: TouchDeviceInfo, sender: Sender<TouchInputEvent>) {
    let thread_info = info.clone();
    thread::Builder::new()
        .name(String::from("shadow-guest-touch"))
        .spawn(move || {
            if let Err(error) = run_touch_reader(thread_info, sender) {
                tracing::warn!("[shadow-guest-compositor] touch-reader failed: {error}");
            }
        })
        .expect("spawn touch reader");
}

pub fn map_normalized_touch_to_frame(
    normalized_x: f64,
    normalized_y: f64,
    panel_width: u32,
    panel_height: u32,
    frame_width: u32,
    frame_height: u32,
) -> Option<(f64, f64)> {
    if panel_width == 0 || panel_height == 0 || frame_width == 0 || frame_height == 0 {
        return None;
    }

    let copy_width = frame_width.min(panel_width);
    let copy_height = frame_height.min(panel_height);
    let dst_x = if panel_width > copy_width {
        (panel_width - copy_width) / 2
    } else {
        0
    };
    let dst_y = if panel_height > copy_height {
        (panel_height - copy_height) / 2
    } else {
        0
    };
    let src_x = if frame_width > panel_width {
        (frame_width - copy_width) / 2
    } else {
        0
    };
    let src_y = if frame_height > panel_height {
        (frame_height - copy_height) / 2
    } else {
        0
    };

    let panel_x = normalized_x.clamp(0.0, 1.0) * f64::from(panel_width.saturating_sub(1));
    let panel_y = normalized_y.clamp(0.0, 1.0) * f64::from(panel_height.saturating_sub(1));
    let copy_width_f = f64::from(copy_width);
    let copy_height_f = f64::from(copy_height);
    let dst_x_f = f64::from(dst_x);
    let dst_y_f = f64::from(dst_y);

    if panel_x < dst_x_f
        || panel_x >= dst_x_f + copy_width_f
        || panel_y < dst_y_f
        || panel_y >= dst_y_f + copy_height_f
    {
        return None;
    }

    Some((
        f64::from(src_x) + (panel_x - dst_x_f),
        f64::from(src_y) + (panel_y - dst_y_f),
    ))
}

fn run_touch_reader(info: TouchDeviceInfo, sender: Sender<TouchInputEvent>) -> Result<()> {
    let touch_path = info.path.to_string_lossy().to_string();
    let mut child = Command::new("/system/bin/getevent")
        .arg("-lt")
        .arg(&touch_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn /system/bin/getevent for {}", info.path.display()))?;
    let stdout = child
        .stdout
        .take()
        .context("capture getevent stdout for touch reader")?;
    let reader = BufReader::new(stdout);
    let mut frame_state = TouchFrameState::default();

    for line in reader.lines() {
        let line =
            line.with_context(|| format!("read touch events from {}", info.path.display()))?;
        let Some(event) = parse_getevent_line(&line)? else {
            continue;
        };
        if let Some(pointer_event) = frame_state.handle_event(event, &info) {
            tracing::info!(
                "[shadow-guest-compositor] touch-reader-event phase={:?} normalized={:.3},{:.3}",
                pointer_event.phase,
                pointer_event.normalized_x,
                pointer_event.normalized_y
            );
            if sender.send(pointer_event).is_err() {
                return Ok(());
            }
        }
    }

    let status = child.wait().context("wait for getevent touch reader")?;
    Err(anyhow!(
        "touch reader exited unexpectedly with status {status}"
    ))
}

const EV_SYN: u16 = 0x00;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;
const SYN_DROPPED: u16 = 0x03;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

fn parse_getevent_line(line: &str) -> Result<Option<RawTouchEvent>> {
    let Some(close_index) = line.find(']') else {
        return Ok(None);
    };
    let timestamp = line[1..close_index].trim();
    let mut fields = line[close_index + 1..].split_whitespace();
    let Some(event_kind) = fields.next() else {
        return Ok(None);
    };
    let Some(event_code) = fields.next() else {
        return Ok(None);
    };
    let Some(event_value) = fields.next() else {
        return Ok(None);
    };

    let time_msec = parse_getevent_time_msec(timestamp)?;
    let parsed = match (event_kind, event_code) {
        ("EV_ABS", "ABS_MT_SLOT") => Some(RawTouchEvent {
            event_type: EV_ABS,
            code: ABS_MT_SLOT,
            value: parse_getevent_hex_i32(event_value)?,
            time_msec,
        }),
        ("EV_ABS", "ABS_MT_TRACKING_ID") => Some(RawTouchEvent {
            event_type: EV_ABS,
            code: ABS_MT_TRACKING_ID,
            value: parse_getevent_hex_i32(event_value)?,
            time_msec,
        }),
        ("EV_ABS", "ABS_MT_POSITION_X") => Some(RawTouchEvent {
            event_type: EV_ABS,
            code: ABS_MT_POSITION_X,
            value: parse_getevent_hex_i32(event_value)?,
            time_msec,
        }),
        ("EV_ABS", "ABS_MT_POSITION_Y") => Some(RawTouchEvent {
            event_type: EV_ABS,
            code: ABS_MT_POSITION_Y,
            value: parse_getevent_hex_i32(event_value)?,
            time_msec,
        }),
        ("EV_SYN", "SYN_REPORT") => Some(RawTouchEvent {
            event_type: EV_SYN,
            code: SYN_REPORT,
            value: 0,
            time_msec,
        }),
        ("EV_SYN", "SYN_DROPPED") => Some(RawTouchEvent {
            event_type: EV_SYN,
            code: SYN_DROPPED,
            value: 0,
            time_msec,
        }),
        _ => None,
    };

    Ok(parsed)
}

fn parse_getevent_time_msec(timestamp: &str) -> Result<u32> {
    let (seconds, fraction) = timestamp
        .split_once('.')
        .ok_or_else(|| anyhow!("parse getevent timestamp: {timestamp}"))?;
    let seconds: i64 = seconds.trim().parse()?;
    let micros: i64 = fraction.trim().parse()?;
    Ok(seconds
        .saturating_mul(1_000)
        .saturating_add(micros.div_euclid(1_000))
        .clamp(0, i64::from(u32::MAX)) as u32)
}

fn parse_getevent_hex_i32(value: &str) -> Result<i32> {
    Ok(u32::from_str_radix(value, 16)? as i32)
}

fn touch_device_info(path: &Path, device: &Device) -> Option<TouchDeviceInfo> {
    let properties = device.properties();
    if !properties.contains(PropType::DIRECT) {
        return None;
    }

    let mut x_range = None;
    let mut y_range = None;
    let absinfo = device.get_absinfo().ok()?;
    for (axis, info) in absinfo {
        match axis {
            AbsoluteAxisCode::ABS_MT_POSITION_X => {
                x_range = Some((info.minimum(), info.maximum()));
            }
            AbsoluteAxisCode::ABS_MT_POSITION_Y => {
                y_range = Some((info.minimum(), info.maximum()));
            }
            _ => {}
        }
    }

    let ((x_min, x_max), (y_min, y_max)) = (x_range?, y_range?);
    Some(TouchDeviceInfo {
        path: path.to_path_buf(),
        name: device.name().unwrap_or("unknown").to_string(),
        x_min,
        x_max,
        y_min,
        y_max,
    })
}

fn is_event_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("event"))
        .unwrap_or(false)
}

fn normalize_axis(value: i32, min: i32, max: i32) -> f64 {
    if max <= min {
        return 0.0;
    }

    ((value - min) as f64 / (max - min) as f64).clamp(0.0, 1.0)
}

#[allow(dead_code)]
fn time_msec(timestamp: SystemTime) -> u32 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

#[cfg(test)]
mod tests {
    use super::{
        map_normalized_touch_to_frame, ContactSnapshot, TouchDeviceInfo, TouchFrameState,
        TouchPhase,
    };

    fn test_device() -> TouchDeviceInfo {
        TouchDeviceInfo {
            path: "/dev/input/event-test".into(),
            name: String::from("test-touch"),
            x_min: 0,
            x_max: 1079,
            y_min: 0,
            y_max: 2339,
        }
    }

    #[test]
    fn touch_state_emits_down_move_up_for_slot_zero() {
        let device = test_device();
        let mut state = TouchFrameState::default();

        state.slot0 = ContactSnapshot {
            tracking_id: Some(1),
            x: 540,
            y: 1200,
        };
        let down = state.flush(&device, 10).expect("down event");
        assert_eq!(down.phase, TouchPhase::Down);

        state.slot0.x = 600;
        let moved = state.flush(&device, 20).expect("move event");
        assert_eq!(moved.phase, TouchPhase::Move);

        state.slot0.tracking_id = None;
        let up = state.flush(&device, 30).expect("up event");
        assert_eq!(up.phase, TouchPhase::Up);

        assert!(state.flush(&device, 40).is_none());
    }

    #[test]
    fn centered_touch_mapping_ignores_letterbox_and_maps_inside_content() {
        assert_eq!(
            map_normalized_touch_to_frame(0.5, 0.5, 1080, 2340, 384, 720),
            Some((191.5, 359.5))
        );
        assert_eq!(
            map_normalized_touch_to_frame(0.0, 0.0, 1080, 2340, 384, 720),
            None
        );
    }
}
