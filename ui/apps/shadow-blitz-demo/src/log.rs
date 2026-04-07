use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

fn start_instant() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

pub fn runtime_log(message: impl AsRef<str>) {
    let elapsed_ms = start_instant().elapsed().as_millis();
    let unix_ms = runtime_wall_ms();
    eprintln!(
        "[shadow-runtime-demo ts_ms={unix_ms} +{elapsed_ms:>6}ms] {}",
        message.as_ref()
    );
}

pub fn runtime_wall_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis()
}

pub fn runtime_log_json(tag: &str, value: &impl Serialize) {
    match serde_json::to_string(value) {
        Ok(payload) => runtime_log(format!("{tag} {payload}")),
        Err(error) => runtime_log(format!("{tag} serialization-error={error}")),
    }
}
