import { derived, writable } from "svelte/store";
import type { ExecutionOutcome } from "../bindings/types";
import { activeRequest, rebasePath } from "./activeRequest";

export interface ResponseRecord {
	outcome: ExecutionOutcome | null;
	error: string | null;
	at: number;
}

export interface RequestResponses {
	loading: boolean;
	/// Newest first. Capped at HISTORY_LIMIT, which is 1 today — the shape is
	/// already a list so turning on real response history later is just
	/// raising the cap and adding a picker, not reworking the store.
	history: ResponseRecord[];
}

const HISTORY_LIMIT = 1;
const EMPTY: RequestResponses = { loading: false, history: [] };

/// Responses are cached per request path, so switching requests shows that
/// request's own last response instead of leaving the previous one on screen.
export const responsesByRequest = writable<Record<string, RequestResponses>>({});

export function markSending(path: string) {
	responsesByRequest.update((map) => ({
		...map,
		[path]: { loading: true, history: map[path]?.history ?? [] },
	}));
}

export function recordResponse(path: string, record: ResponseRecord) {
	responsesByRequest.update((map) => ({
		...map,
		[path]: {
			loading: false,
			history: [record, ...(map[path]?.history ?? [])].slice(0, HISTORY_LIMIT),
		},
	}));
}

/// Moves cached responses along with a renamed/moved request or folder.
export function rekeyResponses(oldPrefix: string, newPrefix: string) {
	responsesByRequest.update((map) =>
		Object.fromEntries(Object.entries(map).map(([path, value]) => [rebasePath(path, oldPrefix, newPrefix), value])),
	);
}

export function forgetResponses(path: string) {
	responsesByRequest.update((map) => {
		const next = { ...map };
		delete next[path];
		return next;
	});
}

export const activeResponses = derived([responsesByRequest, activeRequest], ([$map, $active]) =>
	$active ? ($map[$active.path] ?? EMPTY) : EMPTY,
);
