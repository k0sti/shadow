import { createSignal } from "@shadow/app-runtime-solid";

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
      onClick={() => setCount((value) => value + 1)}
    >
      {props.title} {count()}
    </button>
  );
}

export function renderApp() {
  return (
    <main class="shell">
      <h1>Shadow Runtime Smoke</h1>
      <Counter title="Count" initialCount={1} />
    </main>
  );
}
