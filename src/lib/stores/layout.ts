import { writable } from "svelte/store";

/// How the request editor and the response viewer are stacked.
/// `vertical` puts the response under the request, `horizontal` beside it.
/// Named after the axis the two panes are laid out along, which is also what
/// the settings labels say.
export type PaneOrientation = "vertical" | "horizontal";

export interface LayoutState {
	sidebarWidth: number;
	/// Two sizes rather than one "split position": the orientations want very
	/// different numbers, and sharing a value would resize the pane to
	/// something absurd every time the layout is flipped.
	editorHeight: number;
	editorWidth: number;
	orientation: PaneOrientation;
}

const STORAGE_KEY = "lokki.layout";
const DEFAULTS: LayoutState = {
	sidebarWidth: 260,
	editorHeight: 320,
	editorWidth: 520,
	orientation: "vertical",
};

function isOrientation(value: unknown): value is PaneOrientation {
	return value === "vertical" || value === "horizontal";
}

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
			editorWidth: Number(parsed.editorWidth) || DEFAULTS.editorWidth,
			// Anything else, including state written before orientation
			// existed, falls back to the layout the app has always had.
			orientation: isOrientation(parsed.orientation) ? parsed.orientation : DEFAULTS.orientation,
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
