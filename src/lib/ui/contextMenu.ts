import { writable } from "svelte/store";
import type { IconName } from "./icons";

export interface MenuItem {
	label: string;
	action: () => void;
	/// Required, not optional: an item without an icon would sit half an
	/// icon's width left of its neighbours, and nothing in these menus is
	/// unnameable in the icon set.
	icon: IconName;
	danger?: boolean;
}

/// Items that belong together - creating things, acting on the entry,
/// destroying it. Drawn with a hairline between groups.
export type MenuGroup = MenuItem[];

/// A menu is its groups, in the order they are shown. Both ways of opening
/// one - the ⋯ button (`NodeMenu`) and right-click (`ContextMenu`) - take
/// this same shape, so a menu is defined once per entity.
export type Menu = MenuGroup[];

export interface ContextMenuState {
	x: number;
	y: number;
	menu: Menu;
}

/// The one open context menu, if any. Kept in a store and rendered at the
/// app root rather than inside each row: the sidebar scrolls and clips its
/// overflow, so a menu positioned inside it would be cut off near the
/// bottom.
export const contextMenu = writable<ContextMenuState | null>(null);

/// Opens the menu at the pointer and suppresses the webview's own one.
export function openContextMenu(event: MouseEvent, menu: Menu) {
	event.preventDefault();
	event.stopPropagation();
	contextMenu.set({ x: event.clientX, y: event.clientY, menu });
}

export function closeContextMenu() {
	contextMenu.set(null);
}

/// Drops groups that ended up empty, so a menu can be assembled
/// conditionally without leaving a separator with nothing on one side.
export function menuGroups(menu: Menu): MenuGroup[] {
	return menu.filter((group) => group.length > 0);
}
