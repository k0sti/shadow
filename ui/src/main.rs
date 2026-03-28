use std::sync::Arc;

use shadow_ui_core::{
    app, control,
    scene::{HEIGHT, WIDTH},
    shell::{NavAction, ShellAction, ShellEvent, ShellModel, ShellStatus},
};
use shadow_ui_wgpu::{renderer::Renderer, text::TextSystem};
#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowAttributes, WindowId},
};

struct AppState {
    renderer: Renderer,
    text_system: TextSystem,
    shell: ShellModel,
    window: Arc<Window>,
}

#[derive(Default)]
struct App {
    state: Option<AppState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = WindowAttributes::default()
            .with_title("Shadow")
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(WIDTH as f64, HEIGHT as f64));
        #[cfg(target_os = "linux")]
        let attributes = attributes.with_name("dev.shadow.desktop", "shadow-home");
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));

        let renderer = pollster::block_on(Renderer::new(window.clone()));
        let text_system = TextSystem::new(
            renderer.device(),
            renderer.queue(),
            renderer.surface_format(),
        );

        self.state = Some(AppState {
            renderer,
            text_system,
            shell: ShellModel::new(),
            window: window.clone(),
        });

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.renderer.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.renderer.resize(state.window.inner_size());
                state.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical::<f32>(state.window.scale_factor());
                state.shell.handle(ShellEvent::PointerMoved {
                    x: logical.x,
                    y: logical.y,
                });
                state.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                state.shell.handle(ShellEvent::PointerLeft);
                state.window.request_redraw();
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: button_state,
                ..
            } => {
                if let Some(action) =
                    state
                        .shell
                        .handle(ShellEvent::PointerButton(match button_state {
                            ElementState::Pressed => {
                                shadow_ui_core::shell::PointerButtonState::Pressed
                            }
                            ElementState::Released => {
                                shadow_ui_core::shell::PointerButtonState::Released
                            }
                        }))
                {
                    dispatch_shell_action(action);
                }
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        let action =
                            match code {
                                winit::keyboard::KeyCode::ArrowLeft => Some(NavAction::Left),
                                winit::keyboard::KeyCode::ArrowRight => Some(NavAction::Right),
                                winit::keyboard::KeyCode::ArrowUp => Some(NavAction::Up),
                                winit::keyboard::KeyCode::ArrowDown => Some(NavAction::Down),
                                winit::keyboard::KeyCode::Enter
                                | winit::keyboard::KeyCode::Space => Some(NavAction::Activate),
                                winit::keyboard::KeyCode::Tab => Some(NavAction::Next),
                                winit::keyboard::KeyCode::Escape
                                | winit::keyboard::KeyCode::Home => Some(NavAction::Home),
                                _ => None,
                            };
                        if let Some(action) = action {
                            if let Some(shell_action) =
                                state.shell.handle(ShellEvent::Navigate(action))
                            {
                                dispatch_shell_action(shell_action);
                            }
                            state.window.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let scene = state.shell.scene(&ShellStatus::demo(chrono::Local::now()));
                match state.renderer.render(
                    &scene,
                    &mut state.text_system,
                    state.window.scale_factor() as f32,
                ) {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.renderer.reconfigure();
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Timeout) => {}
                    Err(wgpu::SurfaceError::Other) => {}
                }
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run app");
}

fn dispatch_shell_action(action: ShellAction) {
    match action {
        ShellAction::Launch { app_id } => launch_app(app_id),
        ShellAction::Home => {
            let _ = control::request_home();
        }
    }
}

fn launch_app(app_id: app::AppId) {
    if control::request(control::ControlRequest::Launch { app_id }).unwrap_or(false) {
        return;
    }

    let Some(binary_name) = app::binary_name_for(app_id) else {
        return;
    };

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(bin_dir) = current_exe.parent() {
            let sibling_binary = bin_dir.join(binary_name);
            if sibling_binary.exists() {
                let _ = std::process::Command::new(sibling_binary).spawn();
                return;
            }
        }
    }

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = std::process::Command::new("cargo")
        .current_dir(workspace_root)
        .args(["run", "-p", binary_name])
        .spawn();
}
