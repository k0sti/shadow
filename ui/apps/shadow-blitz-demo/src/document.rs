use std::sync::Arc;

use blitz_dom::{
    DocGuard, DocGuardMut, Document, DocumentConfig, EventDriver, EventHandler, LocalName,
};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_traits::events::{DomEvent, DomEventData, EventState, UiEvent};
use shadow_ui_core::control;

use crate::{
    protocol::{HostMessage, RenderPayload},
    runtime::{RuntimeUpdate, TsRuntime},
};

const FRAME_HTML: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Shadow Blitz Demo</title>
    <style id="shadow-ts-style"></style>
  </head>
  <body>
    <main id="shadow-ts-root"></main>
  </body>
</html>
"#;

pub struct TsDocument {
    inner: HtmlDocument,
    runtime: Option<TsRuntime>,
}

impl TsDocument {
    pub fn new() -> Self {
        match TsRuntime::launch() {
            Ok((runtime, render)) => {
                let mut document = Self {
                    inner: template_document(),
                    runtime: Some(runtime),
                };
                document.apply_render(&render);
                document
            }
            Err(error) => {
                eprintln!("shadow-blitz-demo: runtime launch failed: {error}");
                let mut document = Self {
                    inner: template_document(),
                    runtime: None,
                };
                document.apply_render(&fallback_render(&error.to_string()));
                document
            }
        }
    }

    fn apply_render(&mut self, render: &RenderPayload) {
        let style_id = self
            .inner
            .query_selector("#shadow-ts-style")
            .expect("parse style selector")
            .expect("style node");
        let root_id = self
            .inner
            .query_selector("#shadow-ts-root")
            .expect("parse root selector")
            .expect("root node");

        let mut mutator = self.inner.mutate();
        mutator.set_inner_html(style_id, &render.css);
        mutator.set_inner_html(root_id, &render.html);
        drop(mutator);
    }
}

impl Document for TsDocument {
    fn inner(&self) -> DocGuard<'_> {
        self.inner.inner()
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        self.inner.inner_mut()
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        let mut runtime = self.runtime.take();
        let mut driver = EventDriver::new(
            self,
            TsEventHandler {
                runtime: runtime.as_mut(),
            },
        );
        driver.handle_ui_event(event);
        self.runtime = runtime;
    }

    fn poll(&mut self, _task_context: Option<std::task::Context<'_>>) -> bool {
        if let (Some(runtime), Some(task_context)) = (self.runtime.as_mut(), _task_context.as_ref())
        {
            runtime.register_waker(task_context.waker());
        }

        let Some(runtime) = self.runtime.as_mut() else {
            return false;
        };

        let Some(update) = runtime.drain_update() else {
            return false;
        };

        match update {
            RuntimeUpdate::Render(render) => self.apply_render(&render),
            RuntimeUpdate::Exited(reason) => {
                self.runtime = None;
                self.apply_render(&runtime_stopped_render(&reason));
            }
        }
        true
    }
}

struct TsEventHandler<'a> {
    runtime: Option<&'a mut TsRuntime>,
}

impl EventHandler for TsEventHandler<'_> {
    fn handle_event(
        &mut self,
        chain: &[usize],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        event_state: &mut EventState,
    ) {
        let DomEventData::Click(_) = &event.data else {
            return;
        };
        let Some(action) = find_action(chain, doc) else {
            return;
        };

        if action == "home" {
            let _ = control::request_home();
            event_state.prevent_default();
            return;
        }

        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };
        if let Err(error) = runtime.send(HostMessage::click(action)) {
            eprintln!("shadow-blitz-demo: failed to send event to Deno: {error}");
        }
        event_state.prevent_default();
    }
}

fn find_action(chain: &[usize], doc: &mut dyn Document) -> Option<String> {
    let inner = doc.inner();
    chain.iter().find_map(|node_id| {
        inner
            .get_node(*node_id)
            .and_then(|node| node.attr(LocalName::from("data-action")))
            .map(str::to_owned)
    })
}

fn template_document() -> HtmlDocument {
    HtmlDocument::from_html(
        FRAME_HTML,
        DocumentConfig {
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    )
}

fn fallback_render(message: &str) -> RenderPayload {
    RenderPayload {
        css: r#"
:root {
  color-scheme: dark;
  --bg: #071019;
  --card: rgba(16, 26, 39, 0.94);
  --border: rgba(148, 163, 184, 0.2);
  --text: #f8fafc;
  --muted: #94a3b8;
  --accent: #f59e0b;
}
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; }
body {
  display: grid;
  place-items: center;
  padding: 24px;
  background:
    radial-gradient(circle at top, rgba(245, 158, 11, 0.25), transparent 30%),
    linear-gradient(180deg, #020617, var(--bg));
  color: var(--text);
  font: 500 16px/1.5 system-ui, sans-serif;
}
.card {
  width: min(460px, 100%);
  padding: 24px;
  border-radius: 24px;
  border: 1px solid var(--border);
  background: var(--card);
}
.eyebrow {
  margin: 0 0 8px;
  text-transform: uppercase;
  letter-spacing: 0.22em;
  color: var(--accent);
  font-size: 12px;
}
h1 { margin: 0 0 12px; font-size: 32px; line-height: 1; }
p { margin: 0; color: var(--muted); }
"#
        .to_string(),
        html: format!(
            r#"
<section class="card">
  <p class="eyebrow">Blitz demo</p>
  <h1>Runtime unavailable</h1>
  <p>{message}</p>
</section>
"#
        ),
    }
}

fn runtime_stopped_render(message: &str) -> RenderPayload {
    RenderPayload {
        css: fallback_render(message).css,
        html: format!(
            r#"
<section class="card">
  <p class="eyebrow">Blitz demo</p>
  <h1>Runtime stopped</h1>
  <p>{message}</p>
</section>
"#
        ),
    }
}
