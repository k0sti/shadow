type HostMessage = {
  type: "click";
  action: string;
};

type RenderMessage = {
  type: "render";
  css: string;
  html: string;
};

type State = {
  count: number;
  toneIndex: number;
  lastEvent: string;
  heartbeat: string;
};

const tones = [
  { accent: "#7dd3fc", accentStrong: "#38bdf8", glow: "rgba(56, 189, 248, 0.24)" },
  { accent: "#fca5a5", accentStrong: "#ef4444", glow: "rgba(239, 68, 68, 0.24)" },
  { accent: "#86efac", accentStrong: "#22c55e", glow: "rgba(34, 197, 94, 0.24)" },
];
const eventFilePath = Deno.args[0];
const renderFilePath = Deno.args[1];
if (!eventFilePath || !renderFilePath) {
  throw new Error("missing event mailbox path");
}

const state: State = {
  count: 0,
  toneIndex: 0,
  lastEvent: "Booted in Deno",
  heartbeat: heartbeatLabel(),
};

let renderInFlight = false;
let renderPending = false;
let eventPollInFlight = false;
let eventCursor = 0;
let eventBuffer = "";

await flushRenderQueue();
setInterval(() => {
  state.heartbeat = heartbeatLabel();
  requestRender();
}, 1000);
setInterval(() => {
  void pollEvents();
}, 50);
await new Promise(() => {});

async function pollEvents() {
  if (eventPollInFlight) {
    return;
  }

  eventPollInFlight = true;

  try {
    const snapshot = await Deno.readTextFile(eventFilePath);
    if (snapshot.length <= eventCursor) {
      return;
    }

    eventBuffer += snapshot.slice(eventCursor);
    eventCursor = snapshot.length;

    const lines = eventBuffer.split("\n");
    eventBuffer = lines.pop() ?? "";

    let handledMessage = false;
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) {
        continue;
      }

      const message = JSON.parse(trimmed) as HostMessage;
      handleMessage(message);
      handledMessage = true;
    }

    if (handledMessage) {
      await flushRenderQueue();
    }
  } finally {
    eventPollInFlight = false;
  }
}

function handleMessage(message: HostMessage) {
  if (message.type !== "click") {
    return;
  }

  switch (message.action) {
    case "increment":
      state.count += 1;
      state.lastEvent = "Incremented from native click handling";
      break;
    case "reset":
      state.count = 0;
      state.lastEvent = "Reset from native click handling";
      break;
    case "cycle-tone":
      state.toneIndex = (state.toneIndex + 1) % tones.length;
      state.lastEvent = "Shifted theme from TypeScript state";
      break;
    default:
      state.lastEvent = `Unhandled action: ${message.action}`;
      break;
  }

  state.heartbeat = heartbeatLabel();
}

function requestRender() {
  renderPending = true;
  if (!renderInFlight) {
    void flushRenderQueue();
  }
}

async function flushRenderQueue() {
  if (renderInFlight) {
    return;
  }

  renderInFlight = true;

  try {
    do {
      renderPending = false;
      await emitRender(buildRenderMessage());
    } while (renderPending);
  } finally {
    renderInFlight = false;
  }
}

function buildRenderMessage(): RenderMessage {
  const tone = tones[state.toneIndex];
  return {
    type: "render",
    css: `
:root {
  color-scheme: dark;
  --bg0: #04070d;
  --bg1: #0f172a;
  --bg2: #172033;
  --card: rgba(10, 15, 24, 0.88);
  --border: rgba(148, 163, 184, 0.18);
  --text: #f8fafc;
  --muted: #94a3b8;
  --accent: ${tone.accent};
  --accent-strong: ${tone.accentStrong};
  --glow: ${tone.glow};
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
.count {
  margin: 0;
  font-size: 108px;
  line-height: 0.92;
  letter-spacing: -0.08em;
  color: var(--accent);
}
.meta {
  margin: 12px 0 24px;
  color: var(--muted);
}
.meta strong {
  color: var(--text);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
button {
  appearance: none;
  border: 0;
  border-radius: 999px;
  padding: 14px 18px;
  font: inherit;
  font-weight: 700;
  color: #03111c;
  background: linear-gradient(180deg, var(--accent), var(--accent-strong));
  cursor: pointer;
}
button.secondary {
  color: var(--text);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
}
.footer {
  margin: 22px 0 0;
  color: var(--muted);
}
`,
    html: `
<section class="frame">
  <p class="eyebrow">Shadow demo</p>
  <h1>Blitz rendered.<br>Deno owns state.</h1>
  <p class="lede">
    The Rust host forwards coarse click actions into TypeScript, then applies
    returned HTML and CSS into a Blitz document.
  </p>
  <p class="count">${state.count}</p>
  <p class="meta"><strong>${state.lastEvent}</strong><br>Heartbeat: ${state.heartbeat}</p>
  <div class="actions">
    <button type="button" data-action="increment">Increment</button>
    <button type="button" data-action="cycle-tone" class="secondary">Shift tone</button>
    <button type="button" data-action="reset" class="secondary">Reset</button>
    <button type="button" data-action="home" class="secondary">Home</button>
  </div>
  <p class="footer">This is intentionally pre-incremental today: the TS side polls a coarse event mailbox and emits full HTML/CSS swaps over stdout.</p>
</section>
`,
  };
}

async function emitRender(message: RenderMessage) {
  await Deno.writeTextFile(renderFilePath, `${JSON.stringify(message)}\n`);
}

function heartbeatLabel() {
  return new Date().toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  });
}
