import { writable } from "svelte/store";
import { translate } from "../i18n";

export type NoticeKind = "error" | "success";

export interface AppNotice {
	id: number;
	kind: NoticeKind;
	message: string;
	count: number;
}

let nextId = 1;

/// Surfaced errors and background results. Failures used to go only to the
/// webview console, which the user can't see - every such failure looked
/// like "nothing happened".
export const notices = writable<AppNotice[]>([]);

function push(kind: NoticeKind, message: string, autoDismissMs?: number) {
	let id = 0;
	notices.update((list) => {
		// Repeats are counted rather than stacked: a failure that retriggers
		// itself (a bad render, a rejected refresh) would otherwise flood the
		// screen and keep the UI busy re-rendering.
		const existing = list.find((n) => n.message === message && n.kind === kind);
		if (existing) {
			id = existing.id;
			return list.map((n) => (n === existing ? { ...n, count: n.count + 1 } : n));
		}
		id = nextId++;
		return [...list, { id, kind, message, count: 1 }].slice(-4);
	});
	if (autoDismissMs) setTimeout(() => dismissNotice(id), autoDismissMs);
}

export function reportError(context: string, error: unknown) {
	const detail = error instanceof Error ? error.message : String(error);
	console.error(context, error);
	// Errors stay until dismissed: they may need reading and acting on,
	// unlike a "your request finished" note.
	push("error", `${context}: ${detail}`);
}

const RESULT_DISMISS_MS = 8000;

/// A result the user should know about but doesn't have to act on - it
/// clears itself, since the matching sidebar marker keeps the information
/// available after the toast is gone. `kind` only picks the colour: a 500
/// is a real answer, not a failure of the app.
export function notifyResult(message: string, kind: NoticeKind = "success") {
	push(kind, message, RESULT_DISMISS_MS);
}

export function dismissNotice(id: number) {
	notices.update((list) => list.filter((e) => e.id !== id));
}

/// Catches anything that escapes a handler (a component that throws while
/// rendering, a rejected promise nobody awaited) so it becomes visible too.
export function installGlobalErrorReporting() {
	// Translated at throw time rather than captured once: the listeners are
	// installed before the language is settled.
	window.addEventListener("error", (e) => reportError(translate("common.error"), e.error ?? e.message));
	window.addEventListener("unhandledrejection", (e) => reportError(translate("common.error"), e.reason));
}
