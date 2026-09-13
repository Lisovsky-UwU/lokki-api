import { derived, writable } from "svelte/store";

/// `system` follows the OS setting and is the default. The other two pin the
/// app regardless of it — a dark desktop with one light app on it is a
/// legitimate thing to want.
export type ThemePreference = "system" | "light" | "dark";

/// What the stylesheet actually keys off. `system` never reaches CSS: it is
/// resolved here and stamped on `<html>`, so there is one selector form to
/// write instead of a media query plus a duplicate of it for the pinned case.
export type ResolvedTheme = "light" | "dark";

export const THEME_PREFERENCES: ThemePreference[] = ["system", "light", "dark"];

/// Must stay in step with the inline script in `src/app.html`, which reads
/// the same key before the first paint so the window doesn't flash the wrong
/// theme while the bundle loads.
const STORAGE_KEY = "lokki.theme";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function isPreference(value: unknown): value is ThemePreference {
	return typeof value === "string" && (THEME_PREFERENCES as string[]).includes(value);
}

function stored(): ThemePreference {
	// A per-device preference with no bearing on the core, so it lives in the
	// webview's own storage rather than in app_state.json — and being
	// synchronous is what lets app.html apply it before anything is drawn.
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return isPreference(raw) ? raw : "system";
	} catch {
		return "system";
	}
}

function readSystemTheme(): ResolvedTheme {
	return typeof window !== "undefined" && window.matchMedia?.(DARK_QUERY).matches ? "dark" : "light";
}

/// What the OS is asking for right now. A store rather than a function
/// because, unlike the OS language, this one really does change under a
/// running app — a desktop that turns dark at sunset.
export const systemTheme = writable<ResolvedTheme>(readSystemTheme());

export const themePreference = writable<ThemePreference>("system");

/// The theme in force: the preference, with `system` resolved.
export const theme = derived([themePreference, systemTheme], ([$preference, $system]): ResolvedTheme =>
	$preference === "system" ? $system : $preference,
);

export function setThemePreference(preference: ThemePreference) {
	themePreference.set(preference);
	try {
		localStorage.setItem(STORAGE_KEY, preference);
	} catch {
		// Storage being unavailable must not stop the theme from changing for
		// this session; it just won't be remembered next launch.
	}
}

/// Reads the stored preference and keeps `<html data-theme>` in step with it
/// from then on. The attribute is already set — app.html does that before
/// the first paint — so this is taking ownership of it, not initialising it.
export function initTheme() {
	// Re-read rather than trust the value captured when this module was
	// evaluated: nothing guarantees the order of that against the rest of
	// start-up, and a stale answer here paints the wrong theme.
	systemTheme.set(readSystemTheme());
	themePreference.set(stored());
	// Never unsubscribed on purpose: both the subscription and the listener
	// live as long as the window does.
	theme.subscribe((resolved) => {
		document.documentElement.dataset.theme = resolved;
	});
	window.matchMedia?.(DARK_QUERY).addEventListener("change", (e) => {
		systemTheme.set(e.matches ? "dark" : "light");
	});
}
