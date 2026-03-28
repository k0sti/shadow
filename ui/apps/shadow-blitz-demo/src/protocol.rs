use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HostMessage {
    Click { action: String },
}

impl HostMessage {
    pub fn click(action: impl Into<String>) -> Self {
        Self::Click {
            action: action.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RenderPayload {
    pub css: String,
    pub html: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RuntimeMessage {
    Log { message: String },
    Render(RenderPayload),
}
