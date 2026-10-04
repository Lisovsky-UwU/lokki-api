import { get, writable } from "svelte/store";
import { newHttpRequestSpec, newWebSocketSpec } from "../bindings/types";
import type { HttpRequestSpec, Protocol, RequestFile, WebSocketSpec } from "../bindings/types";

export interface ActiveRequestState {
	path: string;
	request: RequestFile;
	dirty: boolean;
}

export const activeRequest = writable<ActiveRequestState | null>(null);

/// Edits the open request's HTTP spec and marks it unsaved.
///
/// It lives here rather than in a component because two of them edit the
/// same request: the header owns the method and the URL, the tabs below own
/// the parameters, headers, body and auth.
export function mutateHttp(patch: Partial<HttpRequestSpec>) {
	const current = get(activeRequest);
	if (!current) return;
	const http = current.request.http ?? newHttpRequestSpec();
	activeRequest.set({
		...current,
		request: { ...current.request, http: { ...http, ...patch } },
		dirty: true,
	});
}

/// Edits the open WebSocket request's message and marks it unsaved.
export function mutateWebSocket(patch: Partial<WebSocketSpec>) {
	const current = get(activeRequest);
	if (!current) return;
	const websocket = current.request.websocket ?? newWebSocketSpec();
	activeRequest.set({
		...current,
		request: { ...current.request, websocket: { ...websocket, ...patch } },
		dirty: true,
	});
}

/// Switches the open request between HTTP, SSE and WebSocket. All three
/// start from the same HTTP spec, so nothing is lost either way: a
/// WebSocket's message is kept when switching away from it, and found again
/// on the way back.
export function setProtocol(protocol: Protocol) {
	const current = get(activeRequest);
	if (!current || current.request.meta.protocol === protocol) return;
	const websocket =
		protocol === "websocket" ? (current.request.websocket ?? newWebSocketSpec()) : current.request.websocket;
	activeRequest.set({
		...current,
		request: { ...current.request, meta: { ...current.request.meta, protocol }, websocket },
		dirty: true,
	});
}

/// Whether `path` is `prefix` itself or sits inside it. Paths come from the
/// Rust side as native (Windows) paths, so both separators are accepted.
export function isUnder(path: string, prefix: string): boolean {
	return path === prefix || path.startsWith(prefix + "\\") || path.startsWith(prefix + "/");
}

/// Rewrites a path that lived under `oldPrefix` to sit under `newPrefix`.
/// Used after a rename/move, where every path inside the moved subtree
/// changes.
export function rebasePath(path: string, oldPrefix: string, newPrefix: string): string {
	return isUnder(path, oldPrefix) ? newPrefix + path.slice(oldPrefix.length) : path;
}

/// Keeps the open request pointing at the right file after it (or a folder
/// above it) was renamed or moved - otherwise a later save would write back
/// to a path that no longer exists.
export function rebaseActiveRequest(oldPrefix: string, newPrefix: string) {
	const current = get(activeRequest);
	if (!current) return;
	const rebased = rebasePath(current.path, oldPrefix, newPrefix);
	if (rebased !== current.path) activeRequest.set({ ...current, path: rebased });
}
