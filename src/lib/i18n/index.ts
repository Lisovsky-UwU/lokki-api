import { derived, get, writable } from "svelte/store";
import { en } from "./en";
import { ru } from "./ru";

export type Locale = "en" | "ru";

/// English is the fallback everywhere: an unknown OS language, a missing key
/// in another catalogue, and the language the app falls back to when the
/// stored preference can't be read.
export const FALLBACK_LOCALE: Locale = "en";

export const LOCALES: Locale[] = ["en", "ru"];

/// What the language picker shows. Each name is written in its own language,
/// so it is readable whatever the UI is currently set to.
export const LOCALE_NAMES: Record<Locale, string> = {
	en: "English",
	ru: "Русский",
};

/// `en` is the reference catalogue — `ru` is typed against it, so a key added
/// to one and forgotten in the other is a type error rather than a blank
/// label at runtime.
export type TranslationKey = keyof typeof en;

const CATALOGUES: Record<Locale, Record<TranslationKey, string>> = { en, ru };

export type TranslationParams = Record<string, string | number>;

/// The locale in force. Written once at startup by `initLocale` and again
/// whenever the user changes it in settings.
export const locale = writable<Locale>(FALLBACK_LOCALE);

/// Whether a string names one of the languages we ship, used to sift both
/// the stored preference and whatever the OS reports.
export function isLocale(value: unknown): value is Locale {
	return typeof value === "string" && (LOCALES as string[]).includes(value);
}

/// The language the OS asks for. `navigator.language` inside the webview is
/// the system display language (WebView2 on Windows, WKWebView on macOS,
/// LANG on Linux), so it needs no extra plumbing through Rust. Only the
/// primary subtag matters: `ru-RU` and `ru` are the same catalogue here.
export function detectLocale(): Locale {
	const candidates = [...(navigator.languages ?? []), navigator.language];
	for (const tag of candidates) {
		const primary = (tag ?? "").toLowerCase().split("-")[0];
		if (isLocale(primary)) return primary;
	}
	return FALLBACK_LOCALE;
}

/// `{name}` is a placeholder; `{{name}}` is not. Several strings talk about
/// the app's own `{{variable}}` syntax, so a doubled brace has to survive
/// interpolation untouched.
const PLACEHOLDER = /(?<!\{)\{(\w+)\}(?!\})/g;

function interpolate(template: string, params?: TranslationParams): string {
	if (!params) return template;
	return template.replace(PLACEHOLDER, (match, name: string) =>
		name in params ? String(params[name]) : match,
	);
}

/// Non-reactive lookup, for the stores and helpers that translate outside a
/// component. Components use `$t` instead so the text follows a language
/// change without a reload.
export function translate(key: TranslationKey, params?: TranslationParams): string {
	const active = CATALOGUES[get(locale)] ?? CATALOGUES[FALLBACK_LOCALE];
	const template = active[key] ?? CATALOGUES[FALLBACK_LOCALE][key] ?? key;
	return interpolate(template, params);
}

/// `$t("some.key")` in a component. Derived from `locale`, so every rendered
/// string re-evaluates when the language changes.
export const t = derived(
	locale,
	() =>
		(key: TranslationKey, params?: TranslationParams): string =>
			translate(key, params),
);
