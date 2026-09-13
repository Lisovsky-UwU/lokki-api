import { writable } from "svelte/store";

export interface MenuItem {
	label: string;
	action: () => void;
	danger?: boolean;
}

export interface ContextMenuState {
	x: number;
	y: number;
	items: MenuItem[];
}

/// The one open context menu, if any. Kept in a store and rendered at the
/// app root rather than inside each row: the sidebar scrolls and clips its
/// overflow, so a menu positioned inside it would be cut off near the
/// bottom.
export const contextMenu = writable<ContextMenuState | null>(null);

/// Opens the menu at the pointer and suppresses the webview's own one.
export function openContextMenu(event: MouseEvent, items: MenuItem[]) {
	event.preventDefault();
	event.stopPropagation();
	contextMenu.set({ x: event.clientX, y: event.clientY, items });
}

export function closeContextMenu() {
	contextMenu.set(null);
}
