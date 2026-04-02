import { createSignal } from "@shadow/app-runtime-solid";

function describeEvent(event: {
  currentTarget: { checked: boolean; name: string; type: string };
  targetId: string;
  type: string;
}) {
  return `${event.type}:${event.targetId}:${event.currentTarget.name}:${event.currentTarget.type}:${event.currentTarget.checked}`;
}

export function renderApp() {
  const [alertsEnabled, setAlertsEnabled] = createSignal(false);
  const [lastEvent, setLastEvent] = createSignal("idle");

  return (
    <main class="compose">
      <label class="toggle">
        <input
          data-shadow-id="alerts"
          name="alerts"
          type="checkbox"
          checked={alertsEnabled()}
          onChange={(event) => {
            setAlertsEnabled(event.currentTarget.checked);
            setLastEvent(describeEvent(event));
          }}
        />
        <span>Alerts enabled</span>
      </label>
      <p class="status">Enabled: {alertsEnabled() ? "yes" : "no"}</p>
      <p class="status">Last: {lastEvent()}</p>
    </main>
  );
}
