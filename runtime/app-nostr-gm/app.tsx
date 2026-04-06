import { publishEphemeralKind1 } from "@shadow/app-runtime-os";

const GM_CONTENT = "GM";
export const runtimeDocumentCss = `
.gm-shell {
  width: 100%;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-items: stretch;
  padding: 28px;
  box-sizing: border-box;
  background: linear-gradient(180deg, #02111b 0%, #05334d 45%, #0f766e 100%);
  font-family: "Google Sans", "Roboto", "Droid Sans", "Noto Sans", sans-serif;
}

.gm-card {
  flex: 1;
  width: 100%;
  min-height: 100%;
  display: flex;
  flex-direction: column;
  gap: 22px;
  padding: 34px 30px;
  border-radius: 36px;
  box-sizing: border-box;
  box-shadow: 0 28px 90px rgba(0, 0, 0, 0.32);
}

.gm-card-error {
  background: #fef2f2;
  border: 2px solid #ffe4e6;
  color: #991b1b;
}

.gm-card-success {
  background: #f8fafc;
  border: 2px solid #082f49;
  color: #0f172a;
}

.gm-eyebrow {
  margin: 0;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.gm-eyebrow-error {
  color: #b91c1c;
}

.gm-eyebrow-success {
  color: #0369a1;
}

.gm-headline {
  margin: 0;
  font-size: 58px;
  line-height: 0.96;
  letter-spacing: -0.05em;
}

.gm-headline-error {
  color: #7f1d1d;
}

.gm-headline-success {
  color: #020617;
}

.gm-body {
  margin: 0;
  font-size: 28px;
  line-height: 1.32;
}

.gm-body-error {
  color: #7f1d1d;
}

.gm-body-success {
  color: #334155;
}

.gm-qr-frame {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 14px;
  border-radius: 24px;
  background: #ecfeff;
  border: 1px solid rgba(8, 47, 73, 0.16);
}

.gm-qr-grid {
  display: grid;
  gap: 0;
  width: 228px;
  height: 228px;
  background: #ffffff;
  padding: 10px;
  box-sizing: border-box;
}

.gm-meta {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 22px;
  border-radius: 28px;
  background: #f0fdfa;
  border: 1px solid rgba(13, 148, 136, 0.18);
}

.gm-label-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.gm-label {
  margin: 0;
  color: #0f766e;
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.gm-value {
  margin: 0;
  color: #0f172a;
  font-family: "Droid Sans Mono", "Cutive Mono", monospace;
  font-size: 19px;
  line-height: 1.42;
  overflow-wrap: anywhere;
}
`;

type PublishReceipt = {
  content: string;
  createdAt: number;
  failedRelays: Array<{ relayUrl: string; error: string }>;
  id: string;
  noteId: string;
  npub: string;
  primalUrl: string;
  publishedRelays: string[];
  qrRows: string[];
  relayUrls: string[];
};

type AppState =
  | {
    kind: "error";
    message: string;
  }
  | {
    kind: "ready";
    receipt: PublishReceipt;
  };

const appState: AppState = await publishGmPost();

export function renderApp() {
  if (appState.kind === "error") {
    return (
      <main class="gm-shell">
        <section class="gm-card gm-card-error">
          <p class="gm-eyebrow gm-eyebrow-error">Shadow GM</p>
          <h1 class="gm-headline gm-headline-error">Relay publish failed</h1>
          <p class="gm-body gm-body-error">{appState.message}</p>
        </section>
      </main>
    );
  }

  const { receipt } = appState;
  return (
    <main class="gm-shell">
      <section class="gm-card gm-card-success">
        <p class="gm-eyebrow gm-eyebrow-success">Shadow GM</p>
        <h1 class="gm-headline gm-headline-success">GM sent</h1>
        <p class="gm-body gm-body-success">
          Fresh key, one note, Primal link ready to scan.
        </p>
        <section class="gm-meta">
          <LabelValue label="Primal" value={receipt.primalUrl} />
          <LabelValue label="Note" value={receipt.noteId} />
          <LabelValue label="Pubkey" value={receipt.npub} />
          <LabelValue
            label="Relays"
            value={`${receipt.publishedRelays.length}/${receipt.relayUrls.length} ok`}
          />
        </section>
      </section>
    </main>
  );
}

async function publishGmPost(): Promise<AppState> {
  try {
    const receipt = await publishEphemeralKind1({
      content: GM_CONTENT,
    });
    return { kind: "ready", receipt };
  } catch (error) {
    return {
      kind: "error",
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

function LabelValue(props: { label: string; value: string }) {
  return (
    <div class="gm-label-group">
      <p class="gm-label">{props.label}</p>
      <p class="gm-value">{props.value}</p>
    </div>
  );
}
