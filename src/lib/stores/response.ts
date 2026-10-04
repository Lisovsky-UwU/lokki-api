import { derived, get, writable } from "svelte/store";
import type { ExecutionOutcome, KeyValue, SseComment, SseEvent, StreamEnd } from "../bindings/types";
import { activeRequest, isUnder, rebasePath } from "./activeRequest";

/// An SSE stream, live or finished.
export interface StreamRecord {
	/// Null until the response head arrives.
	status: number | null;
	status_text: string;
	headers: KeyValue[];
	/// Oldest first, capped at STREAM_ENTRY_LIMIT - the oldest go first.
	entries: (SseEvent | SseComment)[];
	/// How many entries the cap pushed out.
	dropped: number;
	/// Null while the stream is open. A `not_a_stream` end carries no body
	/// here: that response is filed as the record's `outcome` instead.
	end: StreamEnd | null;
	/// Known once the stream has ended.
	unresolved_variables: string[];
}

export interface ResponseRecord {
	/// For a stream, set only when the server answered with an ordinary
	/// response instead (`stream.end` is then `not_a_stream`).
	outcome: ExecutionOutcome | null;
	error: string | null;
	at: number;
	/// Wall-clock time from send to result, measured in the UI and filled in
	/// by recordResponse. Slightly longer than outcome.trace.total_ms (which
	/// times only the HTTP exchange, not the IPC round trip), and the only
	/// duration available at all when the request failed.
	elapsedMs?: number;
	/// The user stopped this send. Not a failure, and shown as neither. For
	/// a stream, the ordinary way it ends.
	cancelled?: boolean;
	/// Set for an SSE send whose stream got as far as a response head.
	stream?: StreamRecord;
}

export interface RequestResponses {
	loading: boolean;
	/// When the in-flight send started, so the viewer can tick a live timer
	/// while waiting. Null whenever nothing is in flight.
	startedAt: number | null;
	/// Identifies the in-flight send to the backend, which is what cancelling
	/// refers to. Null whenever nothing is in flight.
	sendId: string | null;
	/// Cancel was asked for and the send hasn't come back yet. The error that
	/// follows is the cancellation landing, not a failure to report.
	cancelling: boolean;
	/// The result arrived while the user was looking at some other request,
	/// and they haven't opened this one since - the sidebar marks it so a
	/// background result isn't lost.
	unseen: boolean;
	/// The stream being received right now, for an SSE send in flight. Moves
	/// into the record when the stream ends.
	live: StreamRecord | null;
	/// Newest first. Capped at HISTORY_LIMIT, which is 1 today - the shape is
	/// already a list so turning on real response history later is just
	/// raising the cap and adding a picker, not reworking the store.
	history: ResponseRecord[];
}

const HISTORY_LIMIT = 1;
/// A stream can run for hours; past this many entries the oldest are let
/// go, so an open stream can't grow the webview without bound.
export const STREAM_ENTRY_LIMIT = 5000;
const EMPTY: RequestResponses = {
	loading: false,
	startedAt: null,
	sendId: null,
	cancelling: false,
	unseen: false,
	live: null,
	history: [],
};

/// Responses are cached per request path, so switching requests shows that
/// request's own last response instead of leaving the previous one on screen.
export const responsesByRequest = writable<Record<string, RequestResponses>>({});

/// `stream` starts an empty live stream alongside, for an SSE send.
export function markSending(path: string, sendId: string, stream = false) {
	responsesByRequest.update((map) => ({
		...map,
		[path]: {
			loading: true,
			startedAt: Date.now(),
			sendId,
			cancelling: false,
			unseen: false,
			live: stream
				? { status: null, status_text: "", headers: [], entries: [], dropped: 0, end: null, unresolved_variables: [] }
				: null,
			history: map[path]?.history ?? [],
		},
	}));
}

/// Applies a change to the live stream of `path`, if it still has one - the
/// request may have been deleted, or the send finished, in the meantime.
function updateLive(path: string, change: (live: StreamRecord) => StreamRecord) {
	responsesByRequest.update((map) => {
		const current = map[path];
		if (!current?.live) return map;
		return { ...map, [path]: { ...current, live: change(current.live) } };
	});
}

export function markStreamOpen(path: string, status: number, status_text: string, headers: KeyValue[]) {
	updateLive(path, (live) => ({ ...live, status, status_text, headers }));
}

/// Entries waiting to be written to the store, per path. A fast stream (a
/// model streaming tokens) sends dozens of events a second; copying the
/// entry list and re-rendering for every one of them would be the slowest
/// part of the app, so they are gathered and written a few times a second.
const pendingEntries = new Map<string, (SseEvent | SseComment)[]>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const FLUSH_INTERVAL_MS = 100;

function flushEntries() {
	flushTimer = null;
	const batches = [...pendingEntries];
	pendingEntries.clear();
	for (const [path, batch] of batches) {
		updateLive(path, (live) => {
			const entries = live.entries.concat(batch);
			const overflow = Math.max(0, entries.length - STREAM_ENTRY_LIMIT);
			return {
				...live,
				entries: overflow > 0 ? entries.slice(overflow) : entries,
				dropped: live.dropped + overflow,
			};
		});
	}
}

export function appendStreamEntry(path: string, entry: SseEvent | SseComment) {
	const batch = pendingEntries.get(path);
	if (batch) batch.push(entry);
	else pendingEntries.set(path, [entry]);
	flushTimer ??= setTimeout(flushEntries, FLUSH_INTERVAL_MS);
}

/// The live stream as it stands, with everything still waiting in the batch
/// written in - what a finished send files as its record.
export function takeLiveStream(path: string): StreamRecord | null {
	if (flushTimer) clearTimeout(flushTimer);
	flushEntries();
	return get(responsesByRequest)[path]?.live ?? null;
}

/// Marks that cancelling was requested. The send stays "loading" until it
/// actually comes back - the connection is torn down by the backend, and
/// pretending it is over before that would let a second send start while the
/// first is still unwinding.
export function markCancelling(path: string) {
	responsesByRequest.update((map) => {
		const current = map[path];
		if (!current?.loading) return map;
		return { ...map, [path]: { ...current, cancelling: true } };
	});
}

/// Stores a finished send. Returns true when it finished in the background
/// (the user had moved on to another request), which is the caller's cue to
/// notify them - decided here, from the one place that knows both paths, so
/// it can't drift from the `unseen` flag the sidebar renders.
export function recordResponse(path: string, record: ResponseRecord): boolean {
	// A cancelled send is never worth a background notification: the user
	// asked for it to stop, so its ending is not news.
	const cancelled = get(responsesByRequest)[path]?.cancelling === true;
	const inBackground = !cancelled && get(activeRequest)?.path !== path;
	responsesByRequest.update((map) => {
		const current = map[path];
		const complete: ResponseRecord = {
			...record,
			cancelled,
			elapsedMs: record.elapsedMs ?? (current?.startedAt != null ? record.at - current.startedAt : undefined),
		};
		return {
			...map,
			[path]: {
				loading: false,
				startedAt: null,
				sendId: null,
				cancelling: false,
				unseen: inBackground,
				live: null,
				history: [complete, ...(current?.history ?? [])].slice(0, HISTORY_LIMIT),
			},
		};
	});
	return inBackground;
}

/// Clears the background-result marker once the user opens the request and
/// actually sees the response.
export function markSeen(path: string) {
	responsesByRequest.update((map) => {
		const current = map[path];
		if (!current?.unseen) return map;
		return { ...map, [path]: { ...current, unseen: false } };
	});
}

export interface SubtreeActivity {
	running: boolean;
	/// The newest result nobody has looked at yet; for a folder or collection
	/// any one of them is enough to put a marker on the row.
	unseen: ResponseRecord | null;
}

const IDLE: SubtreeActivity = { running: false, unseen: null };

/// What is going on at `rootPath` or anywhere below it. Matched by path
/// prefix rather than by walking the tree, because a collapsed collection
/// hasn't necessarily loaded its tree yet - and that is exactly the row that
/// needs to stand in for the requests it hides.
export function subtreeActivity(map: Record<string, RequestResponses>, rootPath: string): SubtreeActivity {
	let unseen: ResponseRecord | null = null;
	for (const [path, state] of Object.entries(map)) {
		if (!isUnder(path, rootPath)) continue;
		// A request still in flight outranks a finished one: the row can only
		// show one marker, and "working" is the more urgent of the two.
		if (state.loading) return { running: true, unseen: null };
		if (state.unseen && !unseen) unseen = state.history[0] ?? null;
	}
	return unseen ? { running: false, unseen } : IDLE;
}

/// Moves cached responses along with a renamed/moved request or folder.
export function rekeyResponses(oldPrefix: string, newPrefix: string) {
	responsesByRequest.update((map) =>
		Object.fromEntries(Object.entries(map).map(([path, value]) => [rebasePath(path, oldPrefix, newPrefix), value])),
	);
}

/// Drops cached responses for everything at or below `rootPath` - used when
/// a folder or collection is deleted and all of its requests go with it.
export function forgetResponsesUnder(rootPath: string) {
	responsesByRequest.update((map) =>
		Object.fromEntries(Object.entries(map).filter(([path]) => !isUnder(path, rootPath))),
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
