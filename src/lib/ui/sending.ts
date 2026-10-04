import { get } from "svelte/store";
import { api } from "../api/client";
import { newId } from "../bindings/types";
import type { ExecutionOutcome, ExecutionTrace, KeyValue, SseEnd, WsEnd } from "../bindings/types";
import { translate } from "../i18n";
import { activeRequest } from "../stores/activeRequest";
import { activeCollection } from "../stores/collectionTree";
import {
	appendStreamEntry,
	markCancelling,
	markSending,
	markStreamOpen,
	recordResponse,
	responsesByRequest,
	takeLiveStream,
	type StreamRecord,
} from "../stores/response";
import { workspacePath } from "../stores/workspace";
import { notifyResult, reportError, type NoticeKind } from "./notices";

/// Sends the open request - or, for an SSE or WebSocket request, opens its
/// connection. Lives outside any one component because both the header's
/// button and the response pane start and stop sends, and the two must not
/// drift apart.
export async function sendActiveRequest() {
	const active = get(activeRequest);
	if (!active) return;
	// Results are stored against the request's path, so each request keeps
	// its own last response instead of sharing one global slot.
	const { path, request } = active;
	if (get(responsesByRequest)[path]?.loading) return;
	const name = request.meta.name;
	const sendId = newId();
	const workspace = get(workspacePath);
	const collection = get(activeCollection)?.path ?? null;

	let summary: string;
	let kind: NoticeKind;
	let inBackground: boolean;
	const protocol = request.meta.protocol;
	if (protocol === "sse" || protocol === "websocket") {
		markSending(path, sendId, protocol);
		try {
			const end =
				protocol === "sse"
					? await api.openSseStream(request, workspace, collection, sendId, (message) => {
							if (message.kind === "open") markStreamOpen(path, message.status, message.status_text, message.headers);
							else appendStreamEntry(path, message);
						})
					: await api.openWebSocket(request, workspace, collection, sendId, (message) => {
							if (message.kind === "open") markStreamOpen(path, message.status, message.status_text, message.headers);
							else appendStreamEntry(path, message);
						});
			({ summary, kind, inBackground } = recordConnectionEnd(path, end));
		} catch (e) {
			// The connection never opened, so there is nothing of it to keep.
			takeLiveStream(path);
			summary = translate("request.sendFailedShort");
			kind = "error";
			inBackground = recordResponse(path, { outcome: null, error: String(e), at: Date.now() });
		}
	} else {
		markSending(path, sendId);
		try {
			const outcome = await api.sendRequest(request, workspace, collection, sendId);
			summary = `${outcome.status} ${outcome.status_text}`;
			kind = outcome.status >= 400 ? "error" : "success";
			inBackground = recordResponse(path, { outcome, error: null, at: Date.now() });
		} catch (e) {
			summary = translate("request.sendFailedShort");
			kind = "error";
			inBackground = recordResponse(path, { outcome: null, error: String(e), at: Date.now() });
		}
	}
	// The user moved on to another request while this one was in flight,
	// so the response panel they are looking at won't show the result -
	// the toast and the sidebar marker are the only way they learn of it.
	if (inBackground) notifyResult(translate("request.backgroundDone", { name, summary }), kind);
}

/// Files a finished stream or socket. An answer that never became one - not
/// an event stream, or an upgrade the server refused - becomes an ordinary
/// response record, so the usual viewer shows it: a 401 with a JSON error is
/// better read as one than as an empty log.
function recordConnectionEnd(path: string, end: SseEnd | WsEnd) {
	const { outcome: result, trace, unresolved_variables } = end;
	const live = takeLiveStream(path);
	const sse = "events" in result;
	const stream: StreamRecord = {
		protocol: sse ? "sse" : "websocket",
		status: result.status,
		status_text: result.status_text,
		headers: result.headers,
		entries: live?.entries ?? [],
		dropped: live?.dropped ?? 0,
		sent: sse ? 0 : result.sent,
		received: sse ? 0 : result.received,
		// The body moves into the record's outcome below; keeping a second
		// copy of it here would only double what the store holds.
		end:
			result.end.type === "not_a_stream" || result.end.type === "rejected"
				? { ...result.end, body_base64: "" }
				: result.end,
		unresolved_variables,
	};
	const at = Date.now();

	if (result.end.type === "not_a_stream" || result.end.type === "rejected") {
		return {
			summary: `${result.status} ${result.status_text}`,
			kind: "error" as NoticeKind,
			inBackground: recordResponse(path, {
				outcome: answerAsOutcome(result, result.end.body_base64, trace, unresolved_variables),
				error: null,
				at,
				stream,
			}),
		};
	}
	const error = result.end.type === "failed" ? result.end.message : null;
	const summary = sse
		? translate(error ? "stream.summaryFailed" : "stream.summaryClosed", { count: result.events })
		: translate(error ? "socket.summaryFailed" : "socket.summaryClosed", {
				sent: result.sent,
				received: result.received,
			});
	return {
		summary,
		kind: (error ? "error" : "success") as NoticeKind,
		inBackground: recordResponse(path, { outcome: null, error, at, stream }),
	};
}

function answerAsOutcome(
	head: { status: number; status_text: string; headers: KeyValue[] },
	body_base64: string,
	trace: ExecutionTrace,
	unresolved_variables: string[],
): ExecutionOutcome {
	return {
		status: head.status,
		status_text: head.status_text,
		headers: head.headers,
		body_base64,
		trace,
		unresolved_variables,
	};
}

/// Whether the open request is a WebSocket that is open right now - past
/// the handshake, not merely connecting.
export function activeSocketIsOpen(): boolean {
	const active = get(activeRequest);
	if (active?.request.meta.protocol !== "websocket") return false;
	const live = get(responsesByRequest)[active.path]?.live;
	return live?.protocol === "websocket" && live.status === 101;
}

/// Sends the open WebSocket request's message over its socket. The message
/// is the one in the editor, unsaved edits included - what the user sees is
/// what goes out.
export async function sendSocketMessage() {
	const active = get(activeRequest);
	if (!active || !activeSocketIsOpen()) return;
	const sendId = get(responsesByRequest)[active.path]?.sendId;
	const message = active.request.websocket?.message ?? "";
	if (!sendId) return;
	try {
		await api.sendWebSocketMessage(
			sendId,
			message,
			get(workspacePath),
			get(activeCollection)?.path ?? null,
		);
	} catch (e) {
		reportError(translate("socket.sendFailed"), e);
	}
}

/// Stops the open request's send. For a stream or a socket this is the
/// ordinary way to end it. The send stays "loading" until the backend comes
/// back with the cancellation - the connection is torn down there, and
/// reporting it as finished any earlier would let a second send start while
/// the first is still unwinding.
export async function cancelActiveSend() {
	const path = get(activeRequest)?.path;
	const sendId = path ? get(responsesByRequest)[path]?.sendId : null;
	if (!path || !sendId) return;
	markCancelling(path);
	try {
		await api.cancelSend(sendId);
	} catch (e) {
		reportError(translate("response.cancelFailed"), e);
	}
}
