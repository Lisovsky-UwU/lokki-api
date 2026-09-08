import { writable } from "svelte/store";

export interface LayoutState {
	sidebarWidth: number;
	editorHeight: number;
}

const STORAGE_KEY = "lokki.layout";
const DEFAULTS: LayoutState = { sidebarWidth: 260, editorHeight: 320 };

function load(): LayoutState {
	// Pane sizes are a per-device preference, so they live in the webview's
	// own storage rather than in the (shareable) workspace folder.
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return DEFAULTS;
		const parsed = JSON.parse(raw) as Partial<LayoutState>;
		return {
			sidebarWidth: Number(parsed.sidebarWidth) || DEFAULTS.sidebarWidth,
			editorHeight: Number(parsed.editorHeight) || DEFAULTS.editorHeight,
		};
	} catch {
		return DEFAULTS;
	}
}

export const layout = writable<LayoutState>(load());

export function updateLayout(patch: Partial<LayoutState>) {
	layout.update((current) => {
		const next = { ...current, ...patch };
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
		} catch {
			// Storage being unavailable must not break resizing.
		}
		return next;
	});
}
