import { For } from "@shadow/app-runtime-solid";
import { publishEphemeralKind1 } from "@shadow/app-runtime-os";

const GM_CONTENT = "GM";

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
      <main style={shellStyle()}>
        <section style={cardStyle("#ffe4e6", "#fef2f2", "#991b1b")}>
          <p style={eyebrowStyle("#b91c1c")}>Shadow GM</p>
          <h1 style={headlineStyle("#7f1d1d")}>Relay publish failed</h1>
          <p style={bodyStyle("#7f1d1d")}>{appState.message}</p>
        </section>
      </main>
    );
  }

  const { receipt } = appState;
  return (
    <main style={shellStyle()}>
      <section style={cardStyle("#082f49", "#f8fafc", "#0f172a")}>
        <p style={eyebrowStyle("#0369a1")}>Shadow GM</p>
        <h1 style={headlineStyle("#020617")}>GM sent</h1>
        <p style={bodyStyle("#334155")}>
          Fresh key, one note, Primal link ready to scan.
        </p>
        <div style={qrFrameStyle()}>
          <div
            aria-label="GM event QR"
            style={{
              display: "grid",
              gridTemplateColumns: `repeat(${
                receipt.qrRows[0]?.length ?? 0
              }, 1fr)`,
              gap: "0px",
              width: "228px",
              height: "228px",
              background: "#ffffff",
              padding: "10px",
              boxSizing: "border-box",
            }}
          >
            <For each={receipt.qrRows}>
              {(row) => (
                <For each={row.split("")}>
                  {(cell) => (
                    <span
                      style={{
                        display: "block",
                        width: "100%",
                        height: "100%",
                        background: cell === "1" ? "#020617" : "#ffffff",
                      }}
                    />
                  )}
                </For>
              )}
            </For>
          </div>
        </div>
        <section style={metaSectionStyle()}>
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
    <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
      <p
        style={{
          margin: "0",
          color: "#0f766e",
          fontSize: "12px",
          fontWeight: "700",
          letterSpacing: "0.08em",
          textTransform: "uppercase",
        }}
      >
        {props.label}
      </p>
      <p
        style={{
          margin: "0",
          color: "#0f172a",
          fontFamily: "monospace",
          fontSize: "13px",
          lineHeight: "1.4",
          overflowWrap: "anywhere",
        }}
      >
        {props.value}
      </p>
    </div>
  );
}

function shellStyle() {
  return {
    width: "384px",
    height: "720px",
    display: "flex",
    flexDirection: "column",
    justifyContent: "center",
    alignItems: "center",
    padding: "28px",
    boxSizing: "border-box",
    background:
      "linear-gradient(180deg, #02111b 0%, #05334d 45%, #0f766e 100%)",
  };
}

function cardStyle(borderColor: string, background: string, textColor: string) {
  return {
    width: "100%",
    display: "flex",
    flexDirection: "column",
    gap: "14px",
    padding: "22px",
    borderRadius: "32px",
    background,
    border: `2px solid ${borderColor}`,
    color: textColor,
    boxSizing: "border-box",
    boxShadow: "0 28px 90px rgba(0, 0, 0, 0.32)",
  };
}

function eyebrowStyle(color: string) {
  return {
    margin: "0",
    color,
    fontSize: "13px",
    fontWeight: "700",
    letterSpacing: "0.14em",
    textTransform: "uppercase",
  };
}

function headlineStyle(color: string) {
  return {
    margin: "0",
    color,
    fontSize: "38px",
    lineHeight: "0.96",
    letterSpacing: "-0.05em",
  };
}

function bodyStyle(color: string) {
  return {
    margin: "0",
    color,
    fontSize: "16px",
    lineHeight: "1.45",
  };
}

function qrFrameStyle() {
  return {
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    padding: "14px",
    borderRadius: "24px",
    background: "#ecfeff",
    border: "1px solid rgba(8, 47, 73, 0.16)",
  };
}

function metaSectionStyle() {
  return {
    display: "flex",
    flexDirection: "column",
    gap: "14px",
    padding: "16px",
    borderRadius: "24px",
    background: "#f0fdfa",
    border: "1px solid rgba(13, 148, 136, 0.18)",
  };
}
