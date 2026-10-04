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

/// Bytes as hex pairs, the way a binary WebSocket message is read. Takes
/// base64 because that is how binary crosses IPC, and stops after `limit`
/// bytes: a megabyte of hex is no more readable than the first kilobyte,
/// and far slower to render.
export function hexPreview(base64: string, limit = 1024): { text: string; truncated: boolean } {
	// Only as much is decoded as can be shown: four base64 characters carry
	// three bytes, and one byte past the limit tells whether there was more.
	// A multi-megabyte frame would otherwise be decoded on every render.
	const head = base64.slice(0, Math.ceil((limit + 1) / 3) * 4);
	let binary: string;
	try {
		binary = atob(head);
	} catch {
		return { text: "", truncated: false };
	}
	const shown = Math.min(binary.length, limit);
	const pairs: string[] = [];
	for (let i = 0; i < shown; i++) pairs.push(binary.charCodeAt(i).toString(16).padStart(2, "0"));
	// Sixteen bytes to a line, as every hex viewer has it.
	const lines: string[] = [];
	for (let i = 0; i < pairs.length; i += 16) lines.push(pairs.slice(i, i + 16).join(" "));
	return { text: lines.join("\n"), truncated: binary.length > limit || head.length < base64.length };
}

/// Live counter while a send is in flight. Kept at one decimal: at a 100 ms
/// tick, millisecond precision would just flicker.
export function formatElapsed(t: Translate, ms: number): string {
	if (ms < 60_000) return t("unit.seconds", { value: (ms / 1000).toFixed(1) });
	const minutes = Math.floor(ms / 60_000);
	const seconds = Math.floor((ms % 60_000) / 1000);
	return t("unit.minutesSeconds", { minutes, seconds: String(seconds).padStart(2, "0") });
}
