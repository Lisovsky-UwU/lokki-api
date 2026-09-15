use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How requests are sent. Per-device rather than per-workspace: turning off
/// certificate checking or stretching a timeout describes the machine and
/// the environment being tested from, not the collection, and must not
/// travel to a teammate through a synced workspace folder.
///
/// Every timeout is in milliseconds, and **0 means no limit** - the value a
/// user reaches for when a slow endpoint has to be waited out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestSettings {
    /// Off makes the client accept any certificate. Deliberately explicit:
    /// the response viewer says so when a request went out unverified.
    pub verify_tls: bool,
    pub connect_timeout_ms: u64,
    /// Cap on the wait between two chunks of the response, not on the whole
    /// exchange - a streaming endpoint that keeps sending never trips it.
    pub read_timeout_ms: u64,
    /// Cap on the whole exchange, connection and body included.
    pub total_timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    /// Sent as `User-Agent` unless the request sets its own header. Empty
    /// leaves reqwest's default in place.
    pub user_agent: String,
}

impl Default for RequestSettings {
    fn default() -> Self {
        RequestSettings {
            verify_tls: true,
            connect_timeout_ms: 10_000,
            read_timeout_ms: 30_000,
            total_timeout_ms: 60_000,
            follow_redirects: true,
            max_redirects: 10,
            user_agent: String::new(),
        }
    }
}

/// `None` for 0 - the builder methods take "no timeout" as absence.
pub fn optional_duration(millis: u64) -> Option<Duration> {
    (millis > 0).then(|| Duration::from_millis(millis))
}

/// What the app shows when it starts. Per-device, next to `RequestSettings`
/// and for the same reason: which folder a given machine drops you into is
/// not something a workspace should carry to a teammate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupBehavior {
    /// Reopen whatever was open last - the behaviour the app has always had,
    /// and still the default: most launches continue yesterday's work.
    #[default]
    LastWorkspace,
    /// Always start on the welcome screen, recent workspaces included.
    Welcome,
}
