use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    layout,
    model::{StatusAction, StatusButtonState, StatusModel},
    renderer::Renderer,
    scene,
    text::TextSystem,
};

struct AppState {
    renderer: Renderer,
    text_system: TextSystem,
    window: Arc<Window>,
    model: StatusModel,
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
            .with_title("Shadow Status")
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(
                layout::WINDOW_WIDTH as f64,
                layout::WINDOW_HEIGHT as f64,
            ));
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
            window: window.clone(),
            model: StatusModel::new(),
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
                state.model.pointer_moved(
                    logical.x,
                    logical.y,
                    layout::WINDOW_WIDTH as f32,
                    layout::WINDOW_HEIGHT as f32,
                );
                state.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                state.model.pointer_left();
                state.window.request_redraw();
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: button_state,
                ..
            } => {
                let action = state.model.pointer_button(
                    match button_state {
                        ElementState::Pressed => StatusButtonState::Pressed,
                        ElementState::Released => StatusButtonState::Released,
                    },
                    layout::WINDOW_WIDTH as f32,
                    layout::WINDOW_HEIGHT as f32,
                );
                if let Some(action) = action {
                    handle_action(event_loop, action);
                }
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }

                match (event.state, event.physical_key) {
                    (ElementState::Pressed, PhysicalKey::Code(KeyCode::Escape)) => {
                        handle_action(event_loop, state.model.close_action());
                    }
                    (
                        ElementState::Pressed,
                        PhysicalKey::Code(KeyCode::Tab | KeyCode::ArrowDown),
                    ) => {
                        state.model.focus_next();
                        state.window.request_redraw();
                    }
                    (ElementState::Pressed, PhysicalKey::Code(KeyCode::ArrowUp)) => {
                        state.model.focus_previous();
                        state.window.request_redraw();
                    }
                    (ElementState::Pressed, PhysicalKey::Code(KeyCode::ArrowLeft)) => {
                        state.model.focus_horizontal(-1);
                        state.window.request_redraw();
                    }
                    (ElementState::Pressed, PhysicalKey::Code(KeyCode::ArrowRight)) => {
                        state.model.focus_horizontal(1);
                        state.window.request_redraw();
                    }
                    (ElementState::Pressed, PhysicalKey::Code(KeyCode::Enter | KeyCode::Space)) => {
                        state.model.activate_pressed();
                        state.window.request_redraw();
                    }
                    (
                        ElementState::Released,
                        PhysicalKey::Code(KeyCode::Enter | KeyCode::Space),
                    ) => {
                        if let Some(action) = state.model.activate_released() {
                            handle_action(event_loop, action);
                        }
                        state.window.request_redraw();
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let scene = scene::build_scene(&state.model);
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
                    Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other) => {}
                }
            }
            _ => {}
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run app");
}

fn handle_action(event_loop: &ActiveEventLoop, action: StatusAction) {
    let StatusAction::Close = action;
    event_loop.exit();
}
