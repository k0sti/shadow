use deno_core::Extension;
use deno_core::OpState;
use deno_core::extension;
use deno_core::op2;
use deno_error::JsErrorBox;
use serde::{Deserialize, Serialize};

const DEFAULT_PUBLISH_PUBKEY: &str = "npub-shadow-os";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Kind1Event {
    pub content: String,
    pub created_at: u64,
    pub id: String,
    pub kind: u32,
    pub pubkey: String,
}

#[derive(Debug, Default, Deserialize)]
struct ListKind1Query {
    authors: Option<Vec<String>>,
    since: Option<u64>,
    until: Option<u64>,
    limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct PublishKind1Request {
    content: Option<String>,
    pubkey: Option<String>,
}

#[derive(Debug)]
struct MockNostrService {
    events: Vec<Kind1Event>,
    latest_timestamp: u64,
    sequence: u64,
}

impl MockNostrService {
    fn new() -> Self {
        let events = vec![
            Kind1Event {
                content: String::from("local cache warmed from the system service"),
                created_at: 1_700_000_003,
                id: String::from("shadow-note-3"),
                kind: 1,
                pubkey: String::from("npub-feed-b"),
            },
            Kind1Event {
                content: String::from("relay subscriptions will live below app code"),
                created_at: 1_700_000_002,
                id: String::from("shadow-note-2"),
                kind: 1,
                pubkey: String::from("npub-feed-a"),
            },
            Kind1Event {
                content: String::from("shadow os owns nostr for tiny apps"),
                created_at: 1_700_000_001,
                id: String::from("shadow-note-1"),
                kind: 1,
                pubkey: String::from("npub-feed-a"),
            },
        ];
        let latest_timestamp = events
            .iter()
            .map(|event| event.created_at)
            .max()
            .unwrap_or(0);

        Self {
            sequence: events.len() as u64,
            events,
            latest_timestamp,
        }
    }

    fn list_kind1(&self, query: ListKind1Query) -> Vec<Kind1Event> {
        let authors = query.authors.map(|authors| authors.into_iter().collect::<std::collections::BTreeSet<_>>());
        let mut events: Vec<_> = self
            .events
            .iter()
            .filter(|event| event.kind == 1)
            .filter(|event| {
                authors
                    .as_ref()
                    .is_none_or(|authors| authors.contains(&event.pubkey))
            })
            .filter(|event| query.since.is_none_or(|since| event.created_at >= since))
            .filter(|event| query.until.is_none_or(|until| event.created_at <= until))
            .cloned()
            .collect();

        events.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        if let Some(limit) = query.limit {
            events.truncate(limit);
        }

        events
    }

    fn publish_kind1(&mut self, request: PublishKind1Request) -> Result<Kind1Event, JsErrorBox> {
        let content = request
            .content
            .as_deref()
            .map(str::trim)
            .filter(|content| !content.is_empty())
            .ok_or_else(|| JsErrorBox::type_error("nostr.publishKind1 requires non-empty content"))?
            .to_owned();

        self.sequence += 1;
        self.latest_timestamp += 1;

        let event = Kind1Event {
            content,
            created_at: self.latest_timestamp,
            id: format!("shadow-note-{}", self.sequence),
            kind: 1,
            pubkey: request
                .pubkey
                .as_deref()
                .filter(|pubkey| !pubkey.is_empty())
                .unwrap_or(DEFAULT_PUBLISH_PUBKEY)
                .to_owned(),
        };
        self.events.insert(0, event.clone());

        Ok(event)
    }
}

#[op2]
#[serde]
fn op_runtime_nostr_list_kind1(
    state: &mut OpState,
    #[serde] query: ListKind1Query,
) -> Result<Vec<Kind1Event>, JsErrorBox> {
    Ok(state.borrow::<MockNostrService>().list_kind1(query))
}

#[op2]
#[serde]
fn op_runtime_nostr_publish_kind1(
    state: &mut OpState,
    #[serde] request: PublishKind1Request,
) -> Result<Kind1Event, JsErrorBox> {
    state
        .borrow_mut::<MockNostrService>()
        .publish_kind1(request)
}

extension!(
    runtime_nostr_host_extension,
    ops = [op_runtime_nostr_list_kind1, op_runtime_nostr_publish_kind1],
    esm_entry_point = "ext:runtime_nostr_host_extension/bootstrap.js",
    esm = [dir "js", "bootstrap.js"],
    state = |state| {
        state.put(MockNostrService::new());
    },
);

pub fn init_extension() -> Extension {
    runtime_nostr_host_extension::init()
}
