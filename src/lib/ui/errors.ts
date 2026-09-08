import { writable } from "svelte/store";

export interface AppErrorNotice {
	id: number;
	message: string;
}

let nextId = 1;

/// Surfaced errors. Failures used to go only to the webview console, which
/// the user can't see — every such failure looked like "nothing happened".
export const errorNotices = writable<AppErrorNotice[]>([]);

export function reportError(context: string, error: unknown) {
	const detail = error instanceof Error ? error.message : String(error);
	console.error(context, error);
	errorNotices.update((list) => [...list, { id: nextId++, message: `${context}: ${detail}` }].slice(-4));
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
