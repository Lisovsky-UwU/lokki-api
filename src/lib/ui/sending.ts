import { get } from "svelte/store";
import { api } from "../api/client";
import { newId } from "../bindings/types";
import type { ExecutionOutcome, SseEnd } from "../bindings/types";
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
} from "../stores/response";
import { workspacePath } from "../stores/workspace";
import { notifyResult, reportError, type NoticeKind } from "./notices";

/// Sends the open request - or, for an SSE request, opens its stream. Lives
/// outside any one component because both the header's button and the
/// response pane start and stop sends, and the two must not drift apart.
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
	if (request.meta.protocol === "sse") {
		markSending(path, sendId, true);
		try {
			const end = await api.openSseStream(request, workspace, collection, sendId, (message) => {
				if (message.kind === "open") markStreamOpen(path, message.status, message.status_text, message.headers);
				else appendStreamEntry(path, message);
			});
			({ summary, kind, inBackground } = recordStreamEnd(path, end));
		} catch (e) {
			// The stream never started, so there is nothing of it to keep.
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

/// Files a finished stream. An answer that was not a stream at all becomes
/// an ordinary response record, so the usual viewer shows it - a 401 with a
/// JSON error is better read as one than as an empty event list.
function recordStreamEnd(path: string, end: SseEnd) {
	const { outcome: result, trace, unresolved_variables } = end;
	const live = takeLiveStream(path);
	const stream = {
		status: result.status,
		status_text: result.status_text,
		headers: result.headers,
		entries: live?.entries ?? [],
		dropped: live?.dropped ?? 0,
		end: result.end.type === "not_a_stream" ? { type: "not_a_stream" as const, body_base64: "" } : result.end,
		unresolved_variables,
	};
	const at = Date.now();

	if (result.end.type === "not_a_stream") {
		const outcome: ExecutionOutcome = {
			status: result.status,
			status_text: result.status_text,
			headers: result.headers,
			body_base64: result.end.body_base64,
			trace,
			unresolved_variables,
		};
		return {
			summary: `${result.status} ${result.status_text}`,
			kind: "error" as NoticeKind,
			inBackground: recordResponse(path, { outcome, error: null, at, stream }),
		};
	}
	const error = result.end.type === "failed" ? result.end.message : null;
	return {
		summary: translate(error ? "stream.summaryFailed" : "stream.summaryClosed", { count: result.events }),
		kind: (error ? "error" : "success") as NoticeKind,
		inBackground: recordResponse(path, { outcome: null, error, at, stream }),
	};
}

/// Stops the open request's send. For a stream this is the ordinary way to
/// end it. The send stays "loading" until the backend comes back with the
/// cancellation - the connection is torn down there, and reporting it as
/// finished any earlier would let a second send start while the first is
/// still unwinding.
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
