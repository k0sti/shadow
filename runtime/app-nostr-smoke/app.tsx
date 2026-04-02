import { For, createSignal } from "@shadow/app-runtime-solid";
import { listKind1, publishKind1 } from "@shadow/app-runtime-os";

const PUBLISH_CONTENT = "shadow says hello from the os";

type Kind1Event = {
  content: string;
  created_at: number;
  id: string;
  kind: number;
  pubkey: string;
};

function loadFeed(limit: number): Kind1Event[] {
  return listKind1({ limit });
}

export function renderApp() {
  const initialNotes = loadFeed(3);
  const [notes, setNotes] = createSignal(initialNotes);
  const [status, setStatus] = createSignal(`Loaded ${initialNotes.length} notes`);

  return (
    <main class="nostr-feed">
      <header class="feed-header">
        <h1>Shadow Nostr</h1>
        <button
          class="publish"
          data-shadow-id="publish"
          onClick={() => {
            const published = publishKind1({ content: PUBLISH_CONTENT });
            const nextNotes = loadFeed(4);
            setNotes(nextNotes);
            setStatus(`Posted ${published.id}`);
          }}
        >
          Post Kind 1
        </button>
      </header>
      <p class="feed-status">{status()}</p>
      <ol class="feed-list">
        <For each={notes()}>
          {(note) => (
            <li class="feed-item">
              <article class="feed-note">
                <p class="feed-meta">{note.id}:{note.pubkey}</p>
                <p class="feed-content">{note.content}</p>
              </article>
            </li>
          )}
        </For>
      </ol>
    </main>
  );
}
