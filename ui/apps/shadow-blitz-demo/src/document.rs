use std::sync::Arc;

use blitz_dom::{DocGuard, DocGuardMut, Document, DocumentConfig};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_traits::events::UiEvent;

const FRAME_HTML: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Shadow Blitz Demo</title>
    <style id="shadow-blitz-style"></style>
  </head>
  <body>
    <main id="shadow-blitz-root"></main>
  </body>
</html>
"#;

pub struct StaticDocument {
    inner: HtmlDocument,
}

impl StaticDocument {
    pub fn new() -> Self {
        let mut document = Self {
            inner: template_document(),
        };
        document.apply_render();
        eprintln!("[shadow-blitz-demo] static-document-ready");
        document
    }

    fn apply_render(&mut self) {
        let style_id = self
            .inner
            .query_selector("#shadow-blitz-style")
            .expect("parse style selector")
            .expect("style node");
        let root_id = self
            .inner
            .query_selector("#shadow-blitz-root")
            .expect("parse root selector")
            .expect("root node");

        let mut mutator = self.inner.mutate();
        mutator.set_inner_html(style_id, static_css());
        mutator.set_inner_html(root_id, static_html());
    }
}

impl Document for StaticDocument {
    fn inner(&self) -> DocGuard<'_> {
        self.inner.inner()
    }

    fn inner_mut(&mut self) -> DocGuardMut<'_> {
        self.inner.inner_mut()
    }

    fn handle_ui_event(&mut self, _event: UiEvent) {}

    fn poll(&mut self, _task_context: Option<std::task::Context<'_>>) -> bool {
        false
    }
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

fn static_css() -> &'static str {
    r#"
:root {
  color-scheme: dark;
  --bg0: #050816;
  --bg1: #0e1630;
  --bg2: #141c2e;
  --card: rgba(10, 16, 28, 0.88);
  --border: rgba(125, 211, 252, 0.22);
  --text: #f8fafc;
  --muted: #cbd5e1;
  --accent: #7dd3fc;
  --accent-strong: #38bdf8;
  --glow: rgba(56, 189, 248, 0.24);
}
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; }
body {
  display: grid;
  place-items: center;
  padding: 24px;
  background:
    radial-gradient(circle at top, var(--glow), transparent 32%),
    linear-gradient(180deg, var(--bg0), var(--bg1) 50%, var(--bg2));
  color: var(--text);
  font: 500 16px/1.5 system-ui, sans-serif;
}
.frame {
  width: min(480px, 100%);
  padding: 26px;
  border-radius: 28px;
  border: 1px solid var(--border);
  background: var(--card);
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.38);
}
.eyebrow {
  margin: 0 0 10px;
  text-transform: uppercase;
  letter-spacing: 0.22em;
  color: var(--accent);
  font-size: 12px;
}
h1 {
  margin: 0;
  font-size: 42px;
  line-height: 0.96;
  letter-spacing: -0.06em;
}
.lede {
  margin: 14px 0 24px;
  color: var(--muted);
}
.status {
  margin: 0;
  font-size: 22px;
  line-height: 1.2;
}
.meta {
  margin: 12px 0 0;
  color: var(--muted);
}
"#
}

fn static_html() -> &'static str {
    r#"
<section class="frame">
  <p class="eyebrow">Shadow demo</p>
  <h1>Blitz rendered.<br>Static first.</h1>
  <p class="lede">
    This is the first bring-up rung in this worktree: prove the existing rooted
    guest compositor can present a native Blitz document on the Pixel panel
    before adding click handling or a JS runtime seam.
  </p>
  <p class="status">Static HTML and CSS are live inside Blitz.</p>
  <p class="meta">Next step: native click state. After that: dynamic HTML updates.</p>
</section>
"#
}
