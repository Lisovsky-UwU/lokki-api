import { writable } from "svelte/store";

/// Settings are reachable from the workspace menu in the sidebar, which is a
/// different component from the one that renders the dialog — hence a store
/// rather than a prop.
export const settingsOpen = writable(false);

export function openSettings() {
	settingsOpen.set(true);
}

export function closeSettings() {
	settingsOpen.set(false);
}
