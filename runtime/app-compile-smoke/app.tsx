import { createSignal } from "@shadow/app-runtime-solid";

const SHELL_STYLE =
  "width:384px;height:720px;display:flex;flex-direction:column;justify-content:center;align-items:center;gap:20px;padding:32px;box-sizing:border-box;background:linear-gradient(180deg,#09131c,#10293a);color:#f4fbff;font-family:system-ui,sans-serif";
const TITLE_STYLE =
  "margin:0;font-size:42px;line-height:0.95;letter-spacing:-0.05em;text-align:center";
const LEDE_STYLE =
  "margin:0;max-width:280px;color:#bfd5df;font-size:18px;line-height:1.35;text-align:center";
const BUTTON_STYLE =
  "width:100%;max-width:280px;min-height:96px;border:none;border-radius:28px;background:linear-gradient(135deg,#79d4ff,#2fb8ff);color:#04212d;font-size:36px;font-weight:800;line-height:1;padding:20px 24px;box-shadow:0 24px 72px rgba(0,0,0,0.35)";

type CounterProps = {
  initialCount: number;
  title: string;
};

function Counter(props: CounterProps) {
  const [count, setCount] = createSignal(props.initialCount);

  return (
    <button
      class="primary"
      data-shadow-id="counter"
      style={BUTTON_STYLE}
      onClick={() => setCount((value) => value + 1)}
    >
      {props.title} {count()}
    </button>
  );
}

export function renderApp() {
  return (
    <main class="shell" style={SHELL_STYLE}>
      <h1 style={TITLE_STYLE}>Shadow Runtime Smoke</h1>
      <p style={LEDE_STYLE}>Tap the button on the phone screen.</p>
      <Counter title="Count" initialCount={1} />
    </main>
  );
}
