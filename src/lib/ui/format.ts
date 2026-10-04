import type { translate } from "../i18n";

/// Components pass their `$t`, so the text re-renders when the language
/// changes; `translate` itself would be read once and go stale.
type Translate = typeof translate;

export function formatSize(t: Translate, bytes: number): string {
	if (bytes < 1024) return t("unit.bytes", { value: bytes });
	if (bytes < 1024 * 1024) return t("unit.kilobytes", { value: (bytes / 1024).toFixed(1) });
	return t("unit.megabytes", { value: (bytes / (1024 * 1024)).toFixed(2) });
}

export function formatDuration(t: Translate, ms: number): string {
	if (ms < 1000) return t("unit.milliseconds", { value: ms });
	if (ms < 60_000) return t("unit.seconds", { value: (ms / 1000).toFixed(2) });
	const minutes = Math.floor(ms / 60_000);
	const seconds = Math.round((ms % 60_000) / 1000);
	return t("unit.minutesSeconds", { minutes, seconds });
}

/// Live counter while a send is in flight. Kept at one decimal: at a 100 ms
/// tick, millisecond precision would just flicker.
export function formatElapsed(t: Translate, ms: number): string {
	if (ms < 60_000) return t("unit.seconds", { value: (ms / 1000).toFixed(1) });
	const minutes = Math.floor(ms / 60_000);
	const seconds = Math.floor((ms % 60_000) / 1000);
	return t("unit.minutesSeconds", { minutes, seconds: String(seconds).padStart(2, "0") });
}
