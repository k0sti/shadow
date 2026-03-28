use std::{convert::TryInto, time::Duration};

use font8x8::{UnicodeFonts, BASIC_FONTS};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::{calloop::EventLoop, calloop_wayland_source::WaylandSource},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::{
    color, layout,
    model::{
        ProfileMode, StatusAction, StatusButtonState, StatusModel, StatusTarget, StatusToggle,
    },
    primitives::Color,
};

const WINDOW_WIDTH: u32 = layout::WINDOW_WIDTH;
const WINDOW_HEIGHT: u32 = layout::WINDOW_HEIGHT;

pub fn run() {
    let connection = Connection::connect_to_env().expect("connect to wayland compositor");
    let (globals, event_queue) = registry_queue_init(&connection).expect("init registry");
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<StatusWaylandApp> =
        EventLoop::try_new().expect("create wayland event loop");
    let loop_handle = event_loop.handle();
    WaylandSource::new(connection, event_queue)
        .insert(loop_handle.clone())
        .expect("insert wayland source");

    let compositor = CompositorState::bind(&globals, &qh).expect("bind wl_compositor");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("bind xdg shell");
    let shm = Shm::bind(&globals, &qh).expect("bind wl_shm");

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
    window.set_title("Shadow Status");
    window.set_app_id("dev.shadow.status");
    window.set_min_size(Some((WINDOW_WIDTH, WINDOW_HEIGHT)));
    window.commit();

    let pool_size = buffer_len(WINDOW_WIDTH, WINDOW_HEIGHT);
    let pool = SlotPool::new(pool_size, &shm).expect("create shm slot pool");

    let mut app = StatusWaylandApp {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        pool,
        pool_size,
        width: WINDOW_WIDTH,
        height: WINDOW_HEIGHT,
        buffer: None,
        window,
        keyboard: None,
        pointer: None,
        loop_handle,
        model: StatusModel::new(),
        frame_pending: false,
        needs_redraw: false,
        exit: false,
    };

    while !app.exit {
        event_loop
            .dispatch(Duration::from_millis(16), &mut app)
            .expect("dispatch wayland events");
    }
}

struct StatusWaylandApp {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    pool: SlotPool,
    pool_size: usize,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
    window: Window,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    loop_handle: smithay_client_toolkit::reexports::calloop::LoopHandle<'static, Self>,
    model: StatusModel,
    frame_pending: bool,
    needs_redraw: bool,
    exit: bool,
}

impl StatusWaylandApp {
    fn request_redraw(&mut self, qh: &QueueHandle<Self>) {
        self.needs_redraw = true;
        if !self.frame_pending {
            self.draw(qh);
        }
    }

    fn ensure_pool_capacity(&mut self) {
        let needed = buffer_len(self.width, self.height);
        if needed > self.pool_size {
            self.pool = SlotPool::new(needed, &self.shm).expect("grow shm slot pool");
            self.pool_size = needed;
            self.buffer = None;
        }
    }

    fn draw(&mut self, qh: &QueueHandle<Self>) {
        if self.width == 0 || self.height == 0 {
            return;
        }

        self.ensure_pool_capacity();

        let stride = self.width as i32 * 4;
        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(
                    self.width as i32,
                    self.height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("create shm buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (next_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        self.width as i32,
                        self.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("create secondary shm buffer");
                *buffer = next_buffer;
                canvas
            }
        };

        render_ui(canvas, self.width, self.height, &self.model);

        self.window
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.window
            .wl_surface()
            .frame(qh, self.window.wl_surface().clone());
        buffer
            .attach_to(self.window.wl_surface())
            .expect("attach buffer");
        self.window.commit();
        self.frame_pending = true;
        self.needs_redraw = false;
    }
}

impl CompositorHandler for StatusWaylandApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.frame_pending = false;
        if self.needs_redraw {
            self.draw(qh);
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for StatusWaylandApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for StatusWaylandApp {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let new_width = configure
            .new_size
            .0
            .map(|value| value.get())
            .unwrap_or(WINDOW_WIDTH)
            .max(1);
        let new_height = configure
            .new_size
            .1
            .map(|value| value.get())
            .unwrap_or(WINDOW_HEIGHT)
            .max(1);

        if new_width != self.width || new_height != self.height {
            self.width = new_width;
            self.height = new_height;
            self.buffer = None;
        }

        self.request_redraw(qh);
    }
}

impl SeatHandler for StatusWaylandApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_, _, _| {}),
                )
                .expect("create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }

        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for StatusWaylandApp {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.model.cancel_press();
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        match event.keysym {
            Keysym::Escape => self.exit = true,
            Keysym::Tab | Keysym::Down => {
                self.model.focus_next();
                self.request_redraw(qh);
            }
            Keysym::Up => {
                self.model.focus_previous();
                self.request_redraw(qh);
            }
            Keysym::Left => {
                self.model.focus_horizontal(-1);
                self.request_redraw(qh);
            }
            Keysym::Right => {
                self.model.focus_horizontal(1);
                self.request_redraw(qh);
            }
            Keysym::space | Keysym::Return | Keysym::KP_Enter => {
                self.model.activate_pressed();
                self.request_redraw(qh);
            }
            _ => {}
        }
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        match event.keysym {
            Keysym::space | Keysym::Return | Keysym::KP_Enter => {
                if let Some(action) = self.model.activate_released() {
                    handle_action(self, action);
                }
                self.request_redraw(qh);
            }
            _ => {}
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
    }
}

impl PointerHandler for StatusWaylandApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.window.wl_surface() {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { .. }
                | PointerEventKind::Motion { .. }
                | PointerEventKind::Press { .. }
                | PointerEventKind::Release { .. } => {
                    self.model.pointer_moved(
                        event.position.0 as f32,
                        event.position.1 as f32,
                        self.width as f32,
                        self.height as f32,
                    );
                }
                PointerEventKind::Leave { .. } | PointerEventKind::Axis { .. } => {}
            }

            match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    self.request_redraw(qh);
                }
                PointerEventKind::Press { .. } => {
                    let action = self.model.pointer_button(
                        StatusButtonState::Pressed,
                        self.width as f32,
                        self.height as f32,
                    );
                    if let Some(action) = action {
                        handle_action(self, action);
                    }
                    self.request_redraw(qh);
                }
                PointerEventKind::Release { .. } => {
                    let action = self.model.pointer_button(
                        StatusButtonState::Released,
                        self.width as f32,
                        self.height as f32,
                    );
                    if let Some(action) = action {
                        handle_action(self, action);
                    }
                    self.request_redraw(qh);
                }
                PointerEventKind::Leave { .. } => {
                    self.model.pointer_left();
                    self.request_redraw(qh);
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for StatusWaylandApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(StatusWaylandApp);
delegate_output!(StatusWaylandApp);
delegate_shm!(StatusWaylandApp);
delegate_seat!(StatusWaylandApp);
delegate_keyboard!(StatusWaylandApp);
delegate_pointer!(StatusWaylandApp);
delegate_xdg_shell!(StatusWaylandApp);
delegate_xdg_window!(StatusWaylandApp);
delegate_registry!(StatusWaylandApp);

impl ProvidesRegistryState for StatusWaylandApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

fn handle_action(app: &mut StatusWaylandApp, action: StatusAction) {
    let StatusAction::Close = action;
    app.exit = true;
}

fn buffer_len(width: u32, height: u32) -> usize {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
        .expect("buffer size overflow")
}

fn render_ui(canvas: &mut [u8], width: u32, height: u32, model: &StatusModel) {
    let width_f = width as f32;
    let height_f = height as f32;
    let top_bar = layout::top_bar_frame(width_f);
    let home = layout::home_button_frame();
    let status_card = layout::status_card_frame(width_f);
    let profile_card = layout::profile_card_frame(width_f, height_f);

    fill(canvas, pack_color(color::BACKGROUND));
    fill_rect(canvas, width, height, top_bar, color::SURFACE_TOP);
    fill_rect(
        canvas,
        width,
        height,
        home,
        home_fill(model, StatusTarget::Home),
    );
    draw_text_centered(canvas, width, height, home, 2, "HOME", color::TEXT_PRIMARY);
    draw_text(
        canvas,
        width,
        height,
        188,
        50,
        4,
        "STATUS",
        color::TEXT_PRIMARY,
    );
    draw_text(
        canvas,
        width,
        height,
        188,
        90,
        2,
        "FIELD CONTROLS AND PROFILE BIAS",
        color::TEXT_MUTED,
    );

    fill_rect(canvas, width, height, status_card, color::SURFACE_CARD);
    draw_text(
        canvas,
        width,
        height,
        48,
        176,
        2,
        "SYSTEM CHANNELS",
        color::TEXT_MUTED,
    );
    draw_text(
        canvas,
        width,
        height,
        48,
        206,
        2,
        &format!("{} OF 3 ACTIVE", model.active_count()),
        color::TEXT_SOFT,
    );

    for toggle in StatusToggle::ALL {
        let row = layout::status_row_frame(toggle.index(), width_f);
        let track = layout::toggle_track_frame(toggle.index(), width_f);
        let knob = layout::toggle_knob_frame(toggle.index(), width_f, model.toggle_enabled(toggle));
        let target = StatusTarget::Toggle(toggle);

        draw_focus_ring(canvas, width, height, model, target, row, 4);
        fill_rect(
            canvas,
            width,
            height,
            row,
            row_fill(model, target, color::SURFACE_ROW),
        );
        fill_rect(
            canvas,
            width,
            height,
            track,
            if model.toggle_enabled(toggle) {
                color::ACCENT
            } else {
                color::ACCENT_MUTED
            },
        );
        fill_rect(
            canvas,
            width,
            height,
            knob,
            if model.toggle_enabled(toggle) {
                color::SURFACE_TOP
            } else {
                color::TEXT_MUTED
            },
        );
        draw_text(
            canvas,
            width,
            height,
            row.x as i32 + 20,
            row.y as i32 + 17,
            2,
            &uppercase(toggle.title()),
            color::TEXT_PRIMARY,
        );
        draw_text(
            canvas,
            width,
            height,
            row.x as i32 + 20,
            row.y as i32 + 47,
            1,
            &uppercase(toggle.subtitle()),
            color::TEXT_MUTED,
        );
        draw_text(
            canvas,
            width,
            height,
            track.x as i32 - 50,
            track.y as i32 + 12,
            2,
            if model.toggle_enabled(toggle) {
                "ON"
            } else {
                "OFF"
            },
            if model.toggle_enabled(toggle) {
                color::ACCENT
            } else {
                color::TEXT_SOFT
            },
        );
    }

    fill_rect(canvas, width, height, profile_card, color::SURFACE_CARD);
    draw_text(
        canvas,
        width,
        height,
        48,
        616,
        2,
        "PROFILE",
        color::TEXT_MUTED,
    );
    draw_text(
        canvas,
        width,
        height,
        48,
        646,
        1,
        &uppercase(model.profile().summary()),
        color::TEXT_SOFT,
    );

    for profile in ProfileMode::ALL {
        let chip = layout::profile_chip_frame(profile.index(), width_f, height_f);
        let target = StatusTarget::Profile(profile);
        draw_focus_ring(canvas, width, height, model, target, chip, 4);
        fill_rect(
            canvas,
            width,
            height,
            chip,
            profile_fill(model, profile, target),
        );
        draw_text_centered(
            canvas,
            width,
            height,
            layout::Frame {
                x: chip.x,
                y: chip.y + 18.0,
                w: chip.w,
                h: 20.0,
            },
            2,
            &uppercase(profile.title()),
            color::TEXT_PRIMARY,
        );
        draw_text_centered(
            canvas,
            width,
            height,
            layout::Frame {
                x: chip.x,
                y: chip.y + 50.0,
                w: chip.w,
                h: 14.0,
            },
            1,
            if model.profile() == profile {
                "ACTIVE"
            } else {
                "READY"
            },
            if model.profile() == profile {
                color::ACCENT
            } else {
                color::TEXT_SOFT
            },
        );
    }

    draw_text(
        canvas,
        width,
        height,
        48,
        842,
        2,
        "LAST CHANGE",
        color::TEXT_MUTED,
    );
    draw_wrapped_text(
        canvas,
        width,
        height,
        48,
        878,
        1,
        model.banner(),
        color::TEXT_PRIMARY,
        54,
    );
    draw_text_centered(
        canvas,
        width,
        height,
        layout::Frame {
            x: 24.0,
            y: height as f32 - 84.0,
            w: width as f32 - 48.0,
            h: 20.0,
        },
        1,
        "TAB OR ARROWS MOVE FOCUS. SPACE TOGGLES.",
        color::TEXT_MUTED,
    );
}

fn row_fill(model: &StatusModel, target: StatusTarget, base: Color) -> Color {
    if model.pressed_target() == Some(target) {
        color::SURFACE_ROW_ACTIVE
    } else if model.hovered_target() == Some(target) {
        color::SURFACE_ROW_HOVER
    } else {
        base
    }
}

fn home_fill(model: &StatusModel, target: StatusTarget) -> Color {
    row_fill(model, target, color::SURFACE_ROW)
}

fn profile_fill(model: &StatusModel, profile: ProfileMode, target: StatusTarget) -> Color {
    if model.pressed_target() == Some(target) {
        color::SURFACE_CHIP_ACTIVE
    } else if model.hovered_target() == Some(target) {
        color::SURFACE_CHIP_HOVER
    } else if model.profile() == profile {
        color::SURFACE_CHIP_ACTIVE
    } else {
        color::SURFACE_CHIP
    }
}

fn draw_focus_ring(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    model: &StatusModel,
    target: StatusTarget,
    frame: layout::Frame,
    inset: i32,
) {
    if model.focused_target() != target {
        return;
    }

    fill_rect(
        canvas,
        width,
        height,
        layout::Frame {
            x: frame.x - inset as f32,
            y: frame.y - inset as f32,
            w: frame.w + inset as f32 * 2.0,
            h: frame.h + inset as f32 * 2.0,
        },
        color::FOCUS_RING,
    );
}

fn fill(canvas: &mut [u8], color: u32) {
    for chunk in canvas.chunks_exact_mut(4) {
        let array: &mut [u8; 4] = chunk.try_into().expect("pixel chunk");
        *array = color.to_le_bytes();
    }
}

fn fill_rect(canvas: &mut [u8], width: u32, height: u32, frame: layout::Frame, color: Color) {
    let x0 = frame.x.max(0.0) as u32;
    let y0 = frame.y.max(0.0) as u32;
    let x1 = (frame.x + frame.w).min(width as f32).max(0.0) as u32;
    let y1 = (frame.y + frame.h).min(height as f32).max(0.0) as u32;
    let color = pack_color(color);

    for row in y0..y1 {
        for col in x0..x1 {
            let index = ((row * width + col) * 4) as usize;
            canvas[index..index + 4].copy_from_slice(&color.to_le_bytes());
        }
    }
}

fn draw_text_centered(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    frame: layout::Frame,
    scale: i32,
    text: &str,
    color: Color,
) {
    let glyph_width = 8 * scale;
    let spacing = scale.max(1);
    let total_width = text.chars().count() as i32 * glyph_width
        + (text.chars().count() as i32 - 1).max(0) * spacing;
    let start_x = frame.x as i32 + ((frame.w as i32 - total_width) / 2).max(0);
    draw_text(
        canvas,
        width,
        height,
        start_x,
        frame.y as i32,
        scale,
        text,
        color,
    );
}

fn draw_wrapped_text(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: i32,
    text: &str,
    color: Color,
    max_chars: usize,
) {
    let mut line = String::new();
    let mut line_y = y;

    for word in uppercase(text).split(' ') {
        let projected = if line.is_empty() {
            word.len()
        } else {
            line.len() + 1 + word.len()
        };
        if projected > max_chars && !line.is_empty() {
            draw_text(canvas, width, height, x, line_y, scale, &line, color);
            line.clear();
            line_y += 12 * scale;
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }

    if !line.is_empty() {
        draw_text(canvas, width, height, x, line_y, scale, &line, color);
    }
}

fn draw_text(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: i32,
    text: &str,
    color: Color,
) {
    let mut cursor_x = x;
    let spacing = scale.max(1);

    for character in text.chars() {
        if character == ' ' {
            cursor_x += 8 * scale + spacing;
            continue;
        }

        let Some(glyph) = BASIC_FONTS.get(character) else {
            cursor_x += 8 * scale + spacing;
            continue;
        };

        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = cursor_x + col * scale + dx;
                        let py = y + (7 - row) as i32 * scale + dy;
                        if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                            continue;
                        }
                        let index = ((py as u32 * width + px as u32) * 4) as usize;
                        canvas[index..index + 4].copy_from_slice(&pack_color(color).to_le_bytes());
                    }
                }
            }
        }

        cursor_x += 8 * scale + spacing;
    }
}

fn uppercase(text: &str) -> String {
    text.to_ascii_uppercase()
}

fn pack_color(color: Color) -> u32 {
    let [r, g, b, a] = color.rgba8();
    u32::from_le_bytes([b, g, r, a])
}
