import { writable } from "svelte/store";
import { api } from "../api/client";
import { detectLocale, isLocale, locale, type Locale } from "./index";

/// What the user chose in settings, or `null` for "follow the OS" - the
/// default, and what the app stays on until someone picks a language by
/// hand. Kept separate from `locale` because the two differ in exactly that
/// case, and the settings dialog has to show the choice, not the outcome.
export const languagePreference = writable<Locale | null>(null);

function effective(preference: Locale | null): Locale {
	return preference ?? detectLocale();
}

/// Reads the stored preference and puts the resulting language in force -
/// both in the UI and in the core, whose error messages reach the user
/// verbatim.
///
/// Called before anything else on start-up: a failure while restoring the
/// last workspace is the first thing a user can see, and it should already
/// be in their language.
export async function initLocale(): Promise<void> {
	let preference: Locale | null = null;
	try {
		const stored = await api.getLanguagePreference();
		preference = isLocale(stored) ? stored : null;
	} catch {
		// An unreadable preference is not worth a toast the user can't act
		// on: the OS language, and English behind it, are both still there.
	}
	languagePreference.set(preference);
	await applyLocale(preference);
}

/// Changes the language and remembers the choice. Throws if it can't be
/// stored - the caller reports that, since the language did change on screen
/// and a silent failure would come back at the next launch.
export async function setLanguagePreference(preference: Locale | null): Promise<void> {
	languagePreference.set(preference);
	await applyLocale(preference);
}

async function applyLocale(preference: Locale | null): Promise<void> {
	const active = effective(preference);
	locale.set(active);
	// The core words its own messages, so it needs the resolved language and
	// the preference both: the first to speak, the second to remember.
	await api.setLanguage(preference, active);
}

/// The language "follow the OS" currently resolves to, for the settings
/// dialog to name it. A plain call rather than a store: the OS language
/// cannot change under a running webview.
export function systemLocale(): Locale {
	return detectLocale();
}
