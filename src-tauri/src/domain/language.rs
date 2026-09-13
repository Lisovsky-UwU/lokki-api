use serde::{Deserialize, Serialize};

/// A UI language the app ships. Serialized as `"en"` / `"ru"` — it crosses
/// IPC and is stored in `app_state.json`, so the spelling is part of the
/// on-disk format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// The fallback: what an unknown OS language and an unreadable
    /// preference both resolve to.
    #[default]
    En,
    Ru,
}
