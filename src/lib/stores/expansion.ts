import { writable } from "svelte/store";
import { isUnder, rebasePath } from "./activeRequest";

/// Which collections and folders are open in the sidebar, keyed by path.
///
/// Persisted, because the tree used to come back fully reset on every
/// launch: collections closed, and every folder the user had collapsed open
/// again. Rebuilding that by hand is the first thing you do in a workspace
/// of any size.
///
/// A path is absent until it is toggled, which is what lets the two kinds
/// keep different defaults (see `isExpanded`).
const STORAGE_KEY = "lokki.expanded";

/// Collections start closed, folders start open - the behaviour the tree has
/// always had, kept so a stored state that says nothing about a path reads
/// the same as it did before this was persisted.
export const COLLECTION_DEFAULT_EXPANDED = false;
export const FOLDER_DEFAULT_EXPANDED = true;

function load(): Record<string, boolean> {
	// Per-device UI state with no bearing on the workspace, so it lives in
	// the webview's own storage next to the pane sizes.
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return {};
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
		// Only the booleans: anything else in there is not ours, and a
		// truthy string would silently read as "expanded".
		return Object.fromEntries(
			Object.entries(parsed as Record<string, unknown>).filter(([, v]) => typeof v === "boolean"),
		) as Record<string, boolean>;
	} catch {
		return {};
	}
}

export const expandedPaths = writable<Record<string, boolean>>(load());

function persist(next: Record<string, boolean>) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
	} catch {
		// Storage being unavailable must not stop the tree from opening; the
		// state just won't outlive the session.
	}
}

function write(update: (current: Record<string, boolean>) => Record<string, boolean>) {
	expandedPaths.update((current) => {
		const next = update(current);
		persist(next);
		return next;
	});
}

export function isExpanded(map: Record<string, boolean>, path: string, fallback: boolean): boolean {
	return map[path] ?? fallback;
}

export function setExpanded(path: string, expanded: boolean) {
	write((current) => ({ ...current, [path]: expanded }));
}

export function toggleExpanded(path: string, fallback: boolean) {
	write((current) => ({ ...current, [path]: !isExpanded(current, path, fallback) }));
}

/// Takes the paths rather than a root, because "expand all" has to resolve
/// them from the loaded tree anyway.
export function expandAll(paths: string[]) {
	write((current) => {
		const next = { ...current };
		for (const path of paths) next[path] = true;
		return next;
	});
}

/// Closes each root and everything under it. Path-prefix matching rather
/// than a tree walk, so it reaches folders whose parent is collapsed and
/// therefore never rendered.
///
/// Several roots in one pass because the sidebar folds a whole workspace by
/// naming its collections: one call per collection would rewrite and persist
/// the map once per collection, and the tree would fold a step at a time.
export function collapseAll(rootPaths: string[]) {
	write((current) => {
		const next = { ...current };
		for (const path of Object.keys(next)) {
			if (rootPaths.some((root) => isUnder(path, root))) next[path] = false;
		}
		for (const root of rootPaths) next[root] = false;
		return next;
	});
}

export function collapseAllUnder(rootPath: string) {
	collapseAll([rootPath]);
}

/// Moves the state of a renamed or moved subtree to its new paths. Without
/// this a renamed folder snaps shut, since its entry still answers to a path
/// that no longer exists.
export function rebaseExpanded(oldPrefix: string, newPrefix: string) {
	write((current) =>
		Object.fromEntries(Object.entries(current).map(([path, value]) => [rebasePath(path, oldPrefix, newPrefix), value])),
	);
}

/// Drops the state for a deleted subtree, so the map does not grow a tail of
/// paths that will never exist again.
export function forgetExpandedUnder(rootPath: string) {
	write((current) => Object.fromEntries(Object.entries(current).filter(([path]) => !isUnder(path, rootPath))));
}

