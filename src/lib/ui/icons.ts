/// The app's icon set, as bare path data.
///
/// One shape vocabulary for every icon: a 24x24 box, outlined, stroked in
/// the current text colour and never filled - the same drawing `GhostIcon`
/// does by hand. Keeping the paths here rather than one component per icon
/// is what lets a menu item name its icon in the same object literal that
/// names its label (see `ui/contextMenu.ts`), with an unknown name a type
/// error.
export const ICONS = {
	/// Creating things.
	"request-add": ["M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z", "M14 2v6h6", "M9 15h6", "M12 12v6"],
	"folder-add": [
		"M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.6a2 2 0 0 1-1.7-.9l-.8-1.2a2 2 0 0 0-1.7-.9H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z",
		"M9 13h6",
		"M12 10v6",
	],
	/// Acting on what is already there.
	clone: [
		"M20 9h-9a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h9a2 2 0 0 0 2-2v-9a2 2 0 0 0-2-2z",
		"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1",
	],
	rename: ["M12 20h9", "M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z"],
	environments: ["M12 3 3 8l9 5 9-5-9-5z", "M3 13l9 5 9-5"],
	switch: ["M16 3l4 4-4 4", "M20 7H4", "M8 21l-4-4 4-4", "M4 17h16"],
	/// Folding the tree.
	"expand-all": ["M7 6l5 5 5-5", "M7 13l5 5 5-5"],
	"collapse-all": ["M17 11l-5-5-5 5", "M17 18l-5-5-5 5"],
	/// Destructive.
	delete: ["M3 6h18", "M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2", "M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6", "M10 11v6", "M14 11v6"],
	/// Chrome: the "+" in the sidebar header and the "⋯" that opens a menu.
	/// The dots are zero-length segments with round caps, so they follow the
	/// same stroke settings as every other icon instead of being filled
	/// circles.
	plus: ["M12 5v14", "M5 12h14"],
	more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
} as const;

export type IconName = keyof typeof ICONS;
