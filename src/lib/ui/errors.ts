import { writable } from "svelte/store";

export interface AppErrorNotice {
	id: number;
	message: string;
	count: number;
}

let nextId = 1;

/// Surfaced errors. Failures used to go only to the webview console, which
/// the user can't see — every such failure looked like "nothing happened".
export const errorNotices = writable<AppErrorNotice[]>([]);

export function reportError(context: string, error: unknown) {
	const detail = error instanceof Error ? error.message : String(error);
	const message = `${context}: ${detail}`;
	console.error(context, error);

	errorNotices.update((list) => {
		// Repeats are counted rather than stacked: a failure that retriggers
		// itself (a bad render, a rejected refresh) would otherwise flood the
		// screen and keep the UI busy re-rendering.
		const existing = list.find((n) => n.message === message);
		if (existing) {
			return list.map((n) => (n === existing ? { ...n, count: n.count + 1 } : n));
		}
		return [...list, { id: nextId++, message, count: 1 }].slice(-4);
	});
}

export function dismissError(id: number) {
	errorNotices.update((list) => list.filter((e) => e.id !== id));
}

/// Catches anything that escapes a handler (a component that throws while
/// rendering, a rejected promise nobody awaited) so it becomes visible too.
export function installGlobalErrorReporting() {
	window.addEventListener("error", (e) => reportError("Ошибка", e.error ?? e.message));
	window.addEventListener("unhandledrejection", (e) => reportError("Ошибка", e.reason));
}
