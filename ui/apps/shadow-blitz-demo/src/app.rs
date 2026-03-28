use anyrender_vello::VelloWindowRenderer;
use blitz_shell::{
    create_default_event_loop, BlitzShellEvent, BlitzShellProxy, View, WindowConfig,
};
use std::sync::mpsc::Receiver;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::document::TsDocument;

#[cfg(target_os = "linux")]
use shadow_ui_core::app as shadow_app;
#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesWayland;

pub fn run() {
    let event_loop = create_default_event_loop();
    let (proxy, receiver) = BlitzShellProxy::new(event_loop.create_proxy());
    let window = WindowConfig::with_attributes(
        Box::new(TsDocument::new()),
        VelloWindowRenderer::new(),
        window_attributes(),
    );
    let application = BlitzTsApplication::new(proxy, receiver, window);
    event_loop.run_app(application).expect("run blitz app");
}

struct BlitzTsApplication {
    proxy: BlitzShellProxy,
    event_queue: Receiver<BlitzShellEvent>,
    pending_window: Option<WindowConfig<VelloWindowRenderer>>,
    window: Option<View<VelloWindowRenderer>>,
}

impl BlitzTsApplication {
    fn new(
        proxy: BlitzShellProxy,
        event_queue: Receiver<BlitzShellEvent>,
        window: WindowConfig<VelloWindowRenderer>,
    ) -> Self {
        Self {
            proxy,
            event_queue,
            pending_window: Some(window),
            window: None,
        }
    }

    fn handle_blitz_shell_event(
        &mut self,
        _event_loop: &dyn ActiveEventLoop,
        event: BlitzShellEvent,
    ) {
        let Some(window) = self.window.as_mut() else {
            return;
        };

        match event {
            BlitzShellEvent::Poll { window_id } if window.window_id() == window_id => {
                let _ = window.poll();
            }
            BlitzShellEvent::RequestRedraw { doc_id } if window.doc.id() == doc_id => {
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl ApplicationHandler for BlitzTsApplication {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if let Some(window) = self.window.as_mut() {
            window.resume();
            let _ = window.poll();
        }

        if let Some(config) = self.pending_window.take() {
            let mut window = View::init(config, event_loop, &self.proxy);
            window.resume();
            let _ = window.poll();
            self.window = Some(window);
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        if let Some(window) = self.window.as_mut() {
            window.suspend();
        }
    }

    fn resumed(&mut self, _event_loop: &dyn ActiveEventLoop) {}

    fn suspended(&mut self, _event_loop: &dyn ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.window.take();
            event_loop.exit();
            return;
        }

        if let Some(window) = self.window.as_mut() {
            window.handle_winit_event(event);
        }

        self.proxy.send_event(BlitzShellEvent::Poll { window_id });
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.event_queue.try_recv() {
            self.handle_blitz_shell_event(event_loop, event);
        }
    }
}

fn window_attributes() -> WindowAttributes {
    let attributes = WindowAttributes::default()
        .with_title("Shadow Blitz Demo")
        .with_resizable(false)
        .with_surface_size(LogicalSize::new(384.0, 720.0));

    #[cfg(target_os = "linux")]
    {
        let wayland_attributes = WindowAttributesWayland::default()
            .with_name(shadow_app::BLITZ_DEMO_WAYLAND_APP_ID, "shadow-blitz-demo");
        return attributes.with_platform_attributes(Box::new(wayland_attributes));
    }

    #[allow(unreachable_code)]
    attributes
}
