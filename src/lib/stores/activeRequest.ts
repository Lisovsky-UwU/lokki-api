import { get, writable } from "svelte/store";
import type { RequestFile } from "../bindings/types";

export interface ActiveRequestState {
	path: string;
	request: RequestFile;
	dirty: boolean;
}

export const activeRequest = writable<ActiveRequestState | null>(null);

/// Rewrites a path that lived under `oldPrefix` to sit under `newPrefix`.
/// Used after a rename/move, where every path inside the moved subtree
/// changes.
export function rebasePath(path: string, oldPrefix: string, newPrefix: string): string {
	if (path === oldPrefix) return newPrefix;
	if (path.startsWith(oldPrefix + "\\") || path.startsWith(oldPrefix + "/")) {
		return newPrefix + path.slice(oldPrefix.length);
	}
	return path;
}

/// Keeps the open request pointing at the right file after it (or a folder
/// above it) was renamed or moved — otherwise a later save would write back
/// to a path that no longer exists.
export function rebaseActiveRequest(oldPrefix: string, newPrefix: string) {
	const current = get(activeRequest);
	if (!current) return;
	const rebased = rebasePath(current.path, oldPrefix, newPrefix);
	if (rebased !== current.path) activeRequest.set({ ...current, path: rebased });
}
