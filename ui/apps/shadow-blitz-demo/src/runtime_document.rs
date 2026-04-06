use std::{
    env, fs,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    task::{Context, Waker},
    thread,
    time::Duration,
};

use blitz_dom::{DocGuard, DocGuardMut, Document};
use blitz_html::HtmlDocument;
use blitz_traits::events::UiEvent;
use serde::{Deserialize, Serialize};

use crate::frame::template_document;
use crate::log::runtime_log;
use crate::runtime_session::{RuntimeDispatchEvent, RuntimePointerEvent, RuntimeSession};

const STYLE_SELECTOR: &str = "#shadow-blitz-style";
const ROOT_SELECTOR: &str = "#shadow-blitz-root";
const DEBUG_SELECTOR: &str = "#shadow-blitz-debug";
const DEFAULT_SURFACE_WIDTH: u32 = 384;
const DEFAULT_SURFACE_HEIGHT: u32 = 720;
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeDocumentPayload {
    pub html: String,
    pub css: Option<String>,
}

pub struct RuntimeDocument {
    inner: HtmlDocument,
    payload: RuntimeDocumentPayload,
    frame_nodes: FrameNodes,
    debug_state: DebugOverlayState,
    debug_overlay_enabled: bool,
    touch_signal_path: Option<PathBuf>,
    last_touch_signal_token: Option<String>,
    runtime_session: Option<RuntimeSession>,
    pending_runtime_event: Option<RuntimeDispatchEvent>,
    touch_anywhere_target_id: Option<String>,
    armed_pointer_target_id: Option<String>,
    skip_next_raw_pointer_release: bool,
    skip_next_ui_pointer_release: bool,
    should_exit: bool,
    timer_started: bool,
    touch_signal_timer_started: bool,
    redraw_requested: bool,
    activate_on_pointer_down: bool,
    timer_tx: Sender<RuntimeTimerEvent>,
    timer_rx: Receiver<RuntimeTimerEvent>,
}

impl RuntimeDocument {
    pub fn from_env() -> Self {
        match RuntimeSession::from_env() {
            Ok(Some(mut runtime_session)) => {
                let payload = runtime_session
                    .render_document()
                    .unwrap_or_else(|error| panic!("render initial runtime document: {error}"));
                runtime_log("runtime-session-ready");
                Self::with_runtime(payload, Some(runtime_session))
            }
            Ok(None) => panic!(
                "configure runtime session: missing SHADOW_RUNTIME_APP_BUNDLE_PATH and SHADOW_RUNTIME_HOST_BINARY_PATH"
            ),
            Err(error) => panic!("configure runtime session: {error}"),
        }
    }

    fn with_runtime(
        payload: RuntimeDocumentPayload,
        runtime_session: Option<RuntimeSession>,
    ) -> Self {
        let (timer_tx, timer_rx) = channel();
        let inner = template_document();
        let frame_nodes = FrameNodes::resolve(&inner);
        let pending_runtime_event = auto_click_event_from_env(runtime_session.is_some());
        let mut document = Self {
            inner,
            payload,
            frame_nodes,
            debug_state: DebugOverlayState::default(),
            debug_overlay_enabled: debug_overlay_enabled(),
            touch_signal_path: env::var_os("SHADOW_BLITZ_TOUCH_SIGNAL_PATH")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            last_touch_signal_token: None,
            pending_runtime_event,
            runtime_session,
            touch_anywhere_target_id: env::var("SHADOW_BLITZ_TOUCH_ANYWHERE_TARGET")
                .ok()
                .filter(|value| !value.is_empty()),
            armed_pointer_target_id: None,
            skip_next_raw_pointer_release: false,
            skip_next_ui_pointer_release: false,
            should_exit: false,
            timer_started: false,
            touch_signal_timer_started: false,
            redraw_requested: false,
            activate_on_pointer_down: env::var_os("SHADOW_BLITZ_TOUCH_ACTIVATE_ON_DOWN").is_some(),
            timer_tx,
            timer_rx,
        };
        document.apply_render();
        document.prime_touch_signal();
        if let Some(path) = document.touch_signal_path.as_ref() {
            runtime_log(format!("touch-signal-ready path={}", path.display()));
        }
        runtime_log("runtime-document-ready");
        document
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn replace_document(&mut self, payload: RuntimeDocumentPayload) {
        self.payload = payload;
        self.apply_render();
    }

    fn apply_render(&mut self) {
        let debug_overlay_html = self.debug_overlay_html();
        let mut mutator = self.inner.mutate();
        mutator.set_inner_html(
            self.frame_nodes.style_id,
            self.payload.css.as_deref().unwrap_or(""),
        );
        mutator.set_inner_html(self.frame_nodes.root_id, &self.payload.html);
        if self.debug_overlay_enabled {
            mutator.set_inner_html(self.frame_nodes.debug_id, &debug_overlay_html);
        } else {
            mutator.set_inner_html(self.frame_nodes.debug_id, "");
        }
        drop(mutator);
        self.log_target_hitmap(&self.touch_signal_target_id());
        self.debug_dump_render_state();
    }

    fn handle_runtime_ui_event(&mut self, event: UiEvent) {
        match &event {
            UiEvent::PointerDown(pointer) => {
                self.skip_next_raw_pointer_release = false;
                self.debug_state.ui_seen = true;
                eprintln!(
                    "[shadow-runtime-demo] ui-pointer-down x={} y={} primary={}",
                    pointer.client_x(),
                    pointer.client_y(),
                    pointer.is_primary
                );
            }
            UiEvent::PointerMove(pointer) => {
                self.debug_state.ui_seen = true;
                eprintln!(
                    "[shadow-runtime-demo] ui-pointer-move x={} y={} primary={}",
                    pointer.client_x(),
                    pointer.client_y(),
                    pointer.is_primary
                );
            }
            UiEvent::PointerUp(pointer) => {
                if self.skip_next_ui_pointer_release {
                    self.skip_next_ui_pointer_release = false;
                    self.skip_next_raw_pointer_release = true;
                    self.debug_state.ui_seen = true;
                    eprintln!(
                        "[shadow-runtime-demo] ui-pointer-up-skipped x={} y={} primary={}",
                        pointer.client_x(),
                        pointer.client_y(),
                        pointer.is_primary
                    );
                    return;
                }
                self.skip_next_raw_pointer_release = true;
                self.debug_state.ui_seen = true;
                eprintln!(
                    "[shadow-runtime-demo] ui-pointer-up x={} y={} primary={}",
                    pointer.client_x(),
                    pointer.client_y(),
                    pointer.is_primary
                );
            }
            _ => {}
        }
        let Some(runtime_event) = self.runtime_event_for_ui_event(&event) else {
            return;
        };
        if let Err(error) = self.dispatch_runtime_event(runtime_event, "ui") {
            eprintln!("[shadow-runtime-demo] runtime-event-error: {error}");
        }
    }

    fn runtime_event_for_ui_event(&mut self, event: &UiEvent) -> Option<RuntimeDispatchEvent> {
        match event {
            UiEvent::PointerDown(pointer) => {
                if pointer.is_primary {
                    self.armed_pointer_target_id =
                        self.arm_target_for_pointer(pointer.client_x(), pointer.client_y(), "ui");
                    if self.armed_pointer_target_id.is_some() {
                        self.debug_state.hit_seen = true;
                    }
                    eprintln!(
                        "[shadow-runtime-demo] ui-pointer-armed x={} y={} target={}",
                        pointer.client_x(),
                        pointer.client_y(),
                        self.armed_pointer_target_id.as_deref().unwrap_or("<none>")
                    );
                    if self.activate_on_pointer_down {
                        let Some(target_id) = self.armed_pointer_target_id.take() else {
                            return None;
                        };
                        self.skip_next_ui_pointer_release = true;
                        self.skip_next_raw_pointer_release = true;
                        eprintln!(
                            "[shadow-runtime-demo] ui-pointer-down-activate x={} y={} target={}",
                            pointer.client_x(),
                            pointer.client_y(),
                            target_id
                        );
                        return Some(RuntimeDispatchEvent {
                            target_id,
                            event_type: String::from("click"),
                            value: None,
                            checked: None,
                            selection: None,
                            pointer: Some(RuntimePointerEvent {
                                client_x: Some(pointer.client_x()),
                                client_y: Some(pointer.client_y()),
                                is_primary: Some(pointer.is_primary),
                            }),
                            keyboard: None,
                        });
                    }
                }
                None
            }
            UiEvent::PointerUp(pointer) => {
                if !pointer.is_primary {
                    return None;
                }

                let armed_target_id = self.armed_pointer_target_id.take();
                let released_target_id =
                    self.shadow_target_id_at(pointer.client_x(), pointer.client_y());
                let Some(target_id) =
                    self.resolve_click_target(armed_target_id.clone(), released_target_id.clone())
                else {
                    eprintln!(
                        "[shadow-runtime-demo] runtime-hit-miss x={} y={} armed={} target={}",
                        pointer.client_x(),
                        pointer.client_y(),
                        armed_target_id.as_deref().unwrap_or("<none>"),
                        released_target_id.as_deref().unwrap_or("<none>")
                    );
                    return None;
                };

                Some(RuntimeDispatchEvent {
                    target_id,
                    event_type: String::from("click"),
                    value: None,
                    checked: None,
                    selection: None,
                    pointer: Some(RuntimePointerEvent {
                        client_x: Some(pointer.client_x()),
                        client_y: Some(pointer.client_y()),
                        is_primary: Some(pointer.is_primary),
                    }),
                    keyboard: None,
                })
            }
            _ => None,
        }
    }

    pub fn handle_raw_pointer_button(
        &mut self,
        pressed: bool,
        is_primary: bool,
        client_x: f32,
        client_y: f32,
    ) {
        if !is_primary {
            return;
        }

        if pressed {
            self.skip_next_raw_pointer_release = false;
            self.debug_state.raw_seen = true;
            self.armed_pointer_target_id = self.arm_target_for_pointer(client_x, client_y, "raw");
            if self.armed_pointer_target_id.is_some() {
                self.debug_state.hit_seen = true;
            }
            eprintln!(
                "[shadow-runtime-demo] raw-pointer-down x={client_x} y={client_y} target={}",
                self.armed_pointer_target_id.as_deref().unwrap_or("<none>")
            );
            return;
        }

        if self.skip_next_raw_pointer_release {
            self.skip_next_raw_pointer_release = false;
            self.armed_pointer_target_id = None;
            eprintln!("[shadow-runtime-demo] raw-pointer-up-skipped x={client_x} y={client_y}");
            return;
        }

        let armed_target_id = self.armed_pointer_target_id.take();
        let released_target_id = self.shadow_target_id_at(client_x, client_y);
        eprintln!(
            "[shadow-runtime-demo] raw-pointer-up x={client_x} y={client_y} armed={} target={}",
            armed_target_id.as_deref().unwrap_or("<none>"),
            released_target_id.as_deref().unwrap_or("<none>")
        );

        let Some(target_id) =
            self.resolve_click_target(armed_target_id, released_target_id.clone())
        else {
            return;
        };

        let runtime_event = RuntimeDispatchEvent {
            target_id,
            event_type: String::from("click"),
            value: None,
            checked: None,
            selection: None,
            pointer: Some(RuntimePointerEvent {
                client_x: Some(client_x),
                client_y: Some(client_y),
                is_primary: Some(is_primary),
            }),
            keyboard: None,
        };

        if let Err(error) = self.dispatch_runtime_event(runtime_event, "raw") {
            eprintln!("[shadow-runtime-demo] runtime-event-error: {error}");
        }
    }

    fn shadow_target_id_at(&self, x: f32, y: f32) -> Option<String> {
        let hit = self.inner.hit(x, y)?;
        let mut node_id = Some(hit.node_id);

        while let Some(id) = node_id {
            let node = self.inner.get_node(id)?;
            if let Some(target_id) = node.attrs().and_then(|attrs| {
                attrs.iter().find_map(|attr| {
                    (attr.name.local.as_ref() == "data-shadow-id").then_some(attr.value.to_string())
                })
            }) {
                return Some(target_id);
            }
            node_id = node.parent;
        }

        None
    }

    fn dispatch_runtime_event(
        &mut self,
        event: RuntimeDispatchEvent,
        source: &str,
    ) -> Result<bool, String> {
        let Some(runtime_session) = self.runtime_session.as_mut() else {
            return Ok(false);
        };

        runtime_log(format!(
            "runtime-dispatch-start source={} type={} target={}",
            source, event.event_type, event.target_id
        ));
        let payload = runtime_session.dispatch(event.clone())?;
        self.replace_document(payload);
        self.debug_state.click_seen = true;
        self.refresh_debug_overlay();
        self.redraw_requested = true;
        runtime_log(format!(
            "runtime-event-dispatched source={} type={} target={}",
            source, event.event_type, event.target_id
        ));
        Ok(true)
    }

    fn arm_target_for_pointer(&self, client_x: f32, client_y: f32, source: &str) -> Option<String> {
        let hit_target_id = self.shadow_target_id_at(client_x, client_y);
        if hit_target_id.is_some() {
            return hit_target_id;
        }

        let fallback_target_id = self.touch_anywhere_target_id.clone();
        if let Some(target_id) = fallback_target_id.as_deref() {
            eprintln!(
                "[shadow-runtime-demo] {}-pointer-anywhere-fallback x={} y={} target={}",
                source, client_x, client_y, target_id
            );
        }
        fallback_target_id
    }

    fn resolve_click_target(
        &self,
        armed_target_id: Option<String>,
        released_target_id: Option<String>,
    ) -> Option<String> {
        let armed_target_id = armed_target_id?;
        if self.touch_anywhere_target_id.as_deref() == Some(armed_target_id.as_str()) {
            return Some(armed_target_id);
        }
        match released_target_id {
            Some(released_target_id) if released_target_id != armed_target_id => None,
            _ => Some(armed_target_id),
        }
    }

    fn refresh_debug_overlay(&mut self) {
        if !self.debug_overlay_enabled {
            return;
        }
        let debug_overlay_html = self.debug_overlay_html();
        let mut mutator = self.inner.mutate();
        mutator.set_inner_html(self.frame_nodes.debug_id, &debug_overlay_html);
    }

    fn debug_overlay_html(&self) -> String {
        let lane = |name: &str, enabled: bool| {
            format!(
                r#"<span class="shadow-debug-lane {name}{}"></span>"#,
                if enabled { " is-on" } else { "" }
            )
        };

        format!(
            "{}{}{}{}{}",
            lane("signal", self.debug_state.signal_seen),
            lane("raw", self.debug_state.raw_seen),
            lane("ui", self.debug_state.ui_seen),
            lane("hit", self.debug_state.hit_seen),
            lane("click", self.debug_state.click_seen),
        )
    }

    fn log_target_hitmap(&mut self, target_id: &str) {
        let (surface_width, surface_height) = runtime_surface_size();
        let mut inner = self.inner_mut();
        inner.set_viewport(blitz_traits::shell::Viewport::new(
            surface_width,
            surface_height,
            1.0,
            blitz_traits::shell::ColorScheme::Dark,
        ));
        inner.resolve(0.0);
        drop(inner);

        let mut hit_count = 0_u32;
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;
        let mut sample = None;

        for y in (0..surface_height).step_by(4) {
            for x in (0..surface_width).step_by(4) {
                if self.shadow_target_id_at(x as f32, y as f32).as_deref() == Some(target_id) {
                    hit_count += 1;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    sample.get_or_insert((x, y));
                }
            }
        }

        match sample {
            Some((sample_x, sample_y)) => eprintln!(
                "[shadow-runtime-demo] target-hitmap id={} hits={} bbox={}..{},{}..{} sample={},{}",
                target_id, hit_count, min_x, max_x, min_y, max_y, sample_x, sample_y
            ),
            None => eprintln!(
                "[shadow-runtime-demo] target-hitmap id={} hits=0",
                target_id
            ),
        }
    }

    fn ensure_exit_timer_started(&mut self, task_context: Option<&Context<'_>>) {
        let Some(task_context) = task_context else {
            return;
        };

        if self.timer_started {
            return;
        }

        self.timer_started = true;
        let Some(delay) = optional_duration_from_env("SHADOW_BLITZ_RUNTIME_EXIT_DELAY_MS") else {
            return;
        };

        spawn_timer(
            self.timer_tx.clone(),
            task_context.waker().clone(),
            delay,
            RuntimeTimerEvent::RequestExit,
        );
    }

    fn ensure_touch_signal_timer_started(&mut self, task_context: Option<&Context<'_>>) {
        if self.touch_signal_timer_started || self.touch_signal_path.is_none() {
            return;
        }
        let Some(task_context) = task_context else {
            return;
        };

        self.touch_signal_timer_started = true;
        spawn_repeating_timer(
            self.timer_tx.clone(),
            task_context.waker().clone(),
            Duration::from_millis(40),
            RuntimeTimerEvent::CheckTouchSignal,
        );
    }

    fn handle_timer_event(&mut self, event: RuntimeTimerEvent) -> bool {
        match event {
            RuntimeTimerEvent::RequestExit => {
                self.should_exit = true;
                eprintln!("[shadow-runtime-demo] exit-requested");
                false
            }
            RuntimeTimerEvent::CheckTouchSignal => self.handle_touch_signal_tick(),
        }
    }

    fn handle_touch_signal_tick(&mut self) -> bool {
        let Some(token) = self.read_touch_signal_token() else {
            return false;
        };
        if self.last_touch_signal_token.as_deref() == Some(token.as_str()) {
            return false;
        }

        self.last_touch_signal_token = Some(token.clone());
        self.debug_state.signal_seen = true;
        self.debug_state.hit_seen = true;
        self.refresh_debug_overlay();
        runtime_log(format!("touch-signal-detected token={token}"));

        let runtime_event = RuntimeDispatchEvent {
            target_id: self.touch_signal_target_id(),
            event_type: String::from("click"),
            value: None,
            checked: None,
            selection: None,
            pointer: None,
            keyboard: None,
        };

        match self.dispatch_runtime_event(runtime_event, "touch-signal") {
            Ok(did_update) => did_update,
            Err(error) => {
                eprintln!("[shadow-runtime-demo] runtime-event-error: {error}");
                false
            }
        }
    }

    pub fn check_touch_signal(&mut self) -> bool {
        self.handle_touch_signal_tick()
    }

    pub fn take_redraw_requested(&mut self) -> bool {
        let redraw_requested = self.redraw_requested;
        self.redraw_requested = false;
        redraw_requested
    }

    fn prime_touch_signal(&mut self) {
        self.last_touch_signal_token = self.read_touch_signal_token();
    }

    fn read_touch_signal_token(&self) -> Option<String> {
        let path = self.touch_signal_path.as_ref()?;
        let token = fs::read_to_string(path).ok()?;
        let token = token.trim();
        (!token.is_empty()).then(|| token.to_string())
    }

    fn touch_signal_target_id(&self) -> String {
        self.touch_anywhere_target_id
            .clone()
            .unwrap_or_else(|| String::from("counter"))
    }
    fn node_outer_html(&self, selector: &str) -> String {
        let node_id = self
            .inner
            .query_selector(selector)
            .expect("parse selector")
            .expect("matching node");
        self.inner
            .get_node(node_id)
            .expect("node by selector")
            .outer_html()
    }

    fn node_text_content(&self, selector: &str) -> String {
        let node_id = self
            .inner
            .query_selector(selector)
            .expect("parse selector")
            .expect("matching node");
        self.inner
            .get_node(node_id)
            .expect("node by selector")
            .text_content()
    }

    fn debug_dump_render_state(&self) {
        if env::var_os("SHADOW_BLITZ_RUNTIME_DEBUG_DUMP").is_none() {
            return;
        }

        let root_html = self.node_outer_html(ROOT_SELECTOR);
        let root_text = self.node_text_content(ROOT_SELECTOR);
        let style_text = self.node_text_content(STYLE_SELECTOR);

        eprintln!(
            "[shadow-runtime-demo] render-debug css_len={} root_html_len={} root_text_len={} root_text_excerpt={:?} root_html_excerpt={:?}",
            style_text.len(),
            root_html.len(),
            root_text.len(),
            truncate_debug(&root_text, 160),
            truncate_debug(&root_html, 200),
        );

        self.debug_dump_node_layout("h1");
        self.debug_dump_node_layout("p");
        self.debug_dump_node_layout("button");
    }

    fn debug_dump_node_layout(&self, selector: &str) {
        let Ok(Some(node_id)) = self.inner.query_selector(selector) else {
            eprintln!("[shadow-runtime-demo] node-layout selector={selector:?} missing");
            return;
        };
        eprintln!(
            "[shadow-runtime-demo] node-layout selector={selector:?} node_id={} text={:?}",
            node_id,
            self.inner.get_node(node_id).map(|node| node.text_content()),
        );
        self.inner.debug_log_node(node_id);
    }

    #[cfg(test)]
    fn point_for_target(&mut self, target_id: &str) -> (f32, f32) {
        let (surface_width, surface_height) = runtime_surface_size();
        let mut inner = self.inner_mut();
        inner.set_viewport(blitz_traits::shell::Viewport::new(
            surface_width,
            surface_height,
            1.0,
            blitz_traits::shell::ColorScheme::Dark,
        ));
        inner.resolve(0.0);
        drop(inner);

        for y in (0..surface_height).step_by(4) {
            for x in (0..surface_width).step_by(4) {
                if self.shadow_target_id_at(x as f32, y as f32).as_deref() == Some(target_id) {
                    return (x as f32, y as f32);
                }
            }
        }

        panic!("no hittable point found for target {target_id}");
    }

    #[cfg(test)]
    fn target_at(&mut self, x: f32, y: f32) -> Option<String> {
        let (surface_width, surface_height) = runtime_surface_size();
        let mut inner = self.inner_mut();
        inner.set_viewport(blitz_traits::shell::Viewport::new(
            surface_width,
            surface_height,
            1.0,
            blitz_traits::shell::ColorScheme::Dark,
        ));
        inner.resolve(0.0);
        drop(inner);

        self.shadow_target_id_at(x, y)
    }
}

impl Document for RuntimeDocument {
    fn inner(&self) -> DocGuard<'_> {
        self.inner.inner()
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        self.inner.inner_mut()
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        self.inner.handle_ui_event(event.clone());
        self.handle_runtime_ui_event(event);
    }

    fn poll(&mut self, task_context: Option<std::task::Context<'_>>) -> bool {
        let task_context = task_context.as_ref();
        self.ensure_exit_timer_started(task_context);
        self.ensure_touch_signal_timer_started(task_context);

        let mut changed = false;
        if let Some(event) = self.pending_runtime_event.take() {
            match self.dispatch_runtime_event(event, "auto") {
                Ok(did_update) => {
                    changed |= did_update;
                }
                Err(error) => {
                    eprintln!("[shadow-runtime-demo] runtime-event-error: {error}");
                }
            }
        }
        while let Ok(event) = self.timer_rx.try_recv() {
            changed |= self.handle_timer_event(event);
        }
        let dirty_payload = if let Some(runtime_session) = self.runtime_session.as_mut() {
            match runtime_session.render_if_dirty() {
                Ok(payload) => payload,
                Err(error) => {
                    eprintln!("[shadow-runtime-demo] runtime-event-error: {error}");
                    None
                }
            }
        } else {
            None
        };
        if let Some(payload) = dirty_payload {
            self.replace_document(payload);
            self.refresh_debug_overlay();
            self.redraw_requested = true;
            runtime_log("runtime-dirty-render-applied");
            changed = true;
        }
        changed
    }
}

struct FrameNodes {
    style_id: usize,
    root_id: usize,
    debug_id: usize,
}

impl FrameNodes {
    fn resolve(document: &HtmlDocument) -> Self {
        let style_id = document
            .query_selector(STYLE_SELECTOR)
            .expect("parse style selector")
            .expect("style node");
        let root_id = document
            .query_selector(ROOT_SELECTOR)
            .expect("parse root selector")
            .expect("root node");
        let debug_id = document
            .query_selector(DEBUG_SELECTOR)
            .expect("parse debug selector")
            .expect("debug node");
        Self {
            style_id,
            root_id,
            debug_id,
        }
    }
}

#[derive(Debug, Default)]
struct DebugOverlayState {
    signal_seen: bool,
    raw_seen: bool,
    ui_seen: bool,
    hit_seen: bool,
    click_seen: bool,
}

#[derive(Clone, Copy, Debug)]
enum RuntimeTimerEvent {
    RequestExit,
    CheckTouchSignal,
}

fn spawn_timer(
    timer_tx: Sender<RuntimeTimerEvent>,
    waker: Waker,
    delay: Duration,
    event: RuntimeTimerEvent,
) {
    thread::spawn(move || {
        thread::sleep(delay);
        let _ = timer_tx.send(event);
        waker.wake_by_ref();
    });
}

fn spawn_repeating_timer(
    timer_tx: Sender<RuntimeTimerEvent>,
    waker: Waker,
    delay: Duration,
    event: RuntimeTimerEvent,
) {
    thread::spawn(move || loop {
        thread::sleep(delay);
        if timer_tx.send(event).is_err() {
            break;
        }
        waker.wake_by_ref();
    });
}

fn optional_duration_from_env(key: &str) -> Option<Duration> {
    let value = env::var(key).ok()?;
    if value.is_empty() {
        return None;
    }

    value.parse::<u64>().ok().map(Duration::from_millis)
}

fn debug_overlay_enabled() -> bool {
    !matches!(
        env::var("SHADOW_BLITZ_DEBUG_OVERLAY").ok().as_deref(),
        Some("0") | Some("false") | Some("off")
    )
}

fn auto_click_event_from_env(runtime_session_enabled: bool) -> Option<RuntimeDispatchEvent> {
    if !runtime_session_enabled {
        return None;
    }

    let target_id = env::var("SHADOW_BLITZ_RUNTIME_AUTO_CLICK_TARGET").ok()?;
    if target_id.is_empty() {
        return None;
    }

    Some(RuntimeDispatchEvent {
        target_id,
        event_type: String::from("click"),
        value: None,
        checked: None,
        selection: None,
        pointer: None,
        keyboard: None,
    })
}

fn truncate_debug(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn runtime_surface_size() -> (u32, u32) {
    (
        runtime_surface_dimension("SHADOW_BLITZ_SURFACE_WIDTH", DEFAULT_SURFACE_WIDTH),
        runtime_surface_dimension("SHADOW_BLITZ_SURFACE_HEIGHT", DEFAULT_SURFACE_HEIGHT),
    )
}

fn runtime_surface_dimension(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}
#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::{RuntimeDocument, RuntimeDocumentPayload};

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_MUTEX: Mutex<()> = Mutex::new(());
        TEST_MUTEX.lock().expect("lock runtime document test mutex")
    }

    #[test]
    fn runtime_document_renders_initial_payload_into_fixed_frame() {
        let _guard = test_guard();
        let payload = RuntimeDocumentPayload {
            html: String::from(r#"<section class="screen"><h1>Hello</h1></section>"#),
            css: Some(String::from("body { color: red; }")),
        };
        let document = RuntimeDocument::with_runtime(payload.clone(), None);

        assert_eq!(
            document.node_text_content("#shadow-blitz-style"),
            "body { color: red; }"
        );
        assert_eq!(
            document.node_outer_html("#shadow-blitz-root"),
            format!(r#"<main id="shadow-blitz-root">{}</main>"#, payload.html)
        );
    }

    #[test]
    fn runtime_document_replaces_style_and_root_content() {
        let _guard = test_guard();
        let mut document = RuntimeDocument::with_runtime(
            RuntimeDocumentPayload {
                html: String::from("<p>Before</p>"),
                css: Some(String::from("body { color: red; }")),
            },
            None,
        );

        document.replace_document(RuntimeDocumentPayload {
            html: String::from(r#"<article data-app="next">After</article>"#),
            css: None,
        });

        assert_eq!(document.node_text_content("#shadow-blitz-style"), "");
        assert_eq!(
            document.node_outer_html("#shadow-blitz-root"),
            r#"<main id="shadow-blitz-root"><article data-app="next">After</article></main>"#
        );
    }

    #[test]
    fn raw_pointer_click_arms_on_press_and_disarms_on_release() {
        let _guard = test_guard();
        let mut document = RuntimeDocument::with_runtime(
            RuntimeDocumentPayload {
                html: String::from(r#"<button data-shadow-id="counter">Count 1</button>"#),
                css: None,
            },
            None,
        );
        let (client_x, client_y) = document.point_for_target("counter");

        document.handle_raw_pointer_button(true, true, client_x, client_y);
        assert_eq!(document.armed_pointer_target_id.as_deref(), Some("counter"));

        document.handle_raw_pointer_button(false, true, client_x, client_y);
        assert_eq!(document.armed_pointer_target_id, None);
    }

    #[test]
    fn release_target_none_still_clicks_armed_target() {
        let _guard = test_guard();
        assert_eq!(
            RuntimeDocument::with_runtime(
                RuntimeDocumentPayload {
                    html: String::new(),
                    css: None,
                },
                None
            )
            .resolve_click_target(Some(String::from("counter")), None),
            Some(String::from("counter"))
        );
    }

    #[test]
    fn release_target_mismatch_cancels_click() {
        let _guard = test_guard();
        assert_eq!(
            RuntimeDocument::with_runtime(
                RuntimeDocumentPayload {
                    html: String::new(),
                    css: None,
                },
                None
            )
            .resolve_click_target(Some(String::from("counter")), Some(String::from("other"))),
            None
        );
    }

    #[test]
    fn card_target_hits_multiple_points_inside_card() {
        let _guard = test_guard();
        let mut document = RuntimeDocument::with_runtime(
            RuntimeDocumentPayload {
                html: String::from(
                    r#"<main style="width:100%;height:100%;display:flex;justify-content:center;align-items:center;background:#10293a"><section data-shadow-id="counter" style="display:block;width:280px;height:240px;background:#2fb8ff"></section></main>"#,
                ),
                css: None,
            },
            None,
        );

        for (x, y) in [(120.0, 260.0), (192.0, 360.0), (264.0, 460.0)] {
            assert_eq!(document.target_at(x, y).as_deref(), Some("counter"));
        }
    }
}
