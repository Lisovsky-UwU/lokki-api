//! The language every user-facing string produced by the core is written in.
//!
//! Error text from `store`, `exec` and the commands reaches the user verbatim
//! (see `AppError::Message`), so the core has to know the UI language rather
//! than hand back keys for the frontend to render — that would mean reshaping
//! every error into a code plus arguments, and the trace log along with it.
//!
//! The language is process-wide because it is a property of the person using
//! the app, not of any one request: a single atomic, written once at startup
//! and again whenever the setting changes.

use crate::domain::Language;
use std::sync::atomic::{AtomicU8, Ordering};

pub mod messages;

const EN: u8 = 0;
const RU: u8 = 1;

/// English is the fallback, and therefore the value before the frontend has
/// reported what the OS asks for.
static CURRENT: AtomicU8 = AtomicU8::new(EN);

pub fn current() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        RU => Language::Ru,
        _ => Language::En,
    }
}

pub fn set_current(language: Language) {
    CURRENT.store(
        match language {
            Language::En => EN,
            Language::Ru => RU,
        },
        Ordering::Relaxed,
    );
}

/// Picks one of two `format!` templates by the active language. Written as a
/// macro so the inline captures (`{host}`, `{path}`) keep working, which is
/// what keeps the two wordings readable side by side in `messages`.
#[macro_export]
macro_rules! tr {
    ($en:literal, $ru:literal $(,)?) => {
        match $crate::i18n::current() {
            $crate::domain::Language::En => format!($en),
            $crate::domain::Language::Ru => format!($ru),
        }
    };
}

/// Runs `f` with the language pinned, for tests that assert on message text.
/// The language is process-wide and `cargo test` runs the suite in parallel
/// threads, so the lock is what keeps two such tests from reading each
/// other's setting.
#[cfg(test)]
pub fn with_language<T>(language: Language, f: impl FnOnce() -> T) -> T {
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());

    // A panicking test would otherwise poison the lock and fail every other
    // test that needs it, hiding the one real failure.
    let _guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous = current();
    set_current(language);
    let result = f();
    set_current(previous);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// English is the fallback the whole feature rests on: the language
    /// before the frontend has said anything, and the one an unreadable
    /// preference falls back to.
    #[test]
    fn default_language_is_english() {
        assert_eq!(Language::default(), Language::En);
    }

    #[test]
    fn tr_picks_the_active_language() {
        let host = "example.com";
        assert_eq!(with_language(Language::En, || tr!("at {host}", "у {host}")), "at example.com");
        assert_eq!(with_language(Language::Ru, || tr!("at {host}", "у {host}")), "у example.com");
    }
}
