// Hand-written mirror of the Rust domain types in src-tauri/src/domain/*.
// Field names match serde's wire format exactly (snake_case, since the
// same structs also serialize to the on-disk TOML files) - only Tauri
// command *arguments* are camelCased by the IPC layer, not struct fields.

export type Id = string;

export interface SyncMeta {
	id: Id;
	created_at: string;
	updated_at: string;
	version: number;
}

export type Protocol = "http" | "websocket" | "sse" | "graphql";

export interface RequestMeta extends SyncMeta {
	name: string;
	seq: number;
	protocol: Protocol;
}

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export interface KeyValue {
	key: string;
	value: string;
	enabled: boolean;
}

export type AuthSpec =
	| { type: "none" }
	| { type: "basic"; username: string; password: string }
	| { type: "bearer"; token: string };

/// Text body formats. Each only picks the default Content-Type and the
/// editor highlighting; the payload is the text as typed.
export type TextFormat = "plain" | "json" | "xml" | "yaml" | "edn" | "html" | "css" | "javascript";

/// `raw` and `json` predate `text` and stay readable for requests written
/// before formats existed; new bodies are written as `text`.
export type BodySpec =
	| { type: "none" }
	| { type: "raw"; content: string }
	| { type: "json"; content: string }
	| { type: "text"; content: string; format: TextFormat }
	| { type: "form"; fields: KeyValue[] }
	| { type: "file"; path: string };

export interface HttpRequestSpec {
	method: HttpMethod;
	url: string;
	query: KeyValue[];
	headers: KeyValue[];
	auth: AuthSpec;
	body: BodySpec;
}

/// The message a WebSocket request sends. The format only picks the editor
/// highlighting: a message carries no type, and always goes out as text.
export interface WebSocketSpec {
	format: TextFormat;
	message: string;
}

/// An SSE request keeps its spec in `http` too: what opens a stream is an
/// ordinary HTTP request, only the response is read differently. So does a
/// WebSocket request, for its handshake; `websocket` holds what it sends
/// once open, and stays when the request is switched to another protocol.
export interface RequestFile {
	meta: RequestMeta;
	http?: HttpRequestSpec;
	websocket?: WebSocketSpec;
}

export interface WorkspaceFile extends SyncMeta {
	name: string;
	schema_version: number;
}

/// An entry in the welcome screen's recent list. The name is stored with the
/// path rather than read from the folder, so an entry still renders when the
/// folder behind it cannot be reached.
export interface RecentWorkspace {
	path: string;
	name: string;
}

export interface CollectionSummary {
	name: string;
	path: string;
}

/// What an import created. `warnings` is the part worth reading: each entry
/// is something the specification asked for that the collection could not
/// reproduce.
export interface ImportResult {
	collection: CollectionSummary;
	requests: number;
	folders: number;
	environments: number;
	warnings: string[];
}

export type CollectionTreeNode =
	| { kind: "Folder"; name: string; path: string; seq: number; children: CollectionTreeNode[] }
	| {
			kind: "Request";
			name: string;
			path: string;
			seq: number;
			protocol: Protocol;
			method: HttpMethod | null;
	  };

/// How requests go out. Per-device (stored next to the app's other local
/// state, not in the workspace): timeouts and TLS checking describe the
/// machine being tested from, not the collection. Every timeout is in
/// milliseconds and 0 means "no limit".
export interface RequestSettings {
	verify_tls: boolean;
	connect_timeout_ms: number;
	read_timeout_ms: number;
	total_timeout_ms: number;
	follow_redirects: boolean;
	max_redirects: number;
	user_agent: string;
}

/// A UI language the app ships. Mirrors domain::language::Language, which
/// serializes to these exact strings in app_state.json.
export type Language = "en" | "ru";

/// What the app opens on launch. Mirrors domain::settings::StartupBehavior.
export type StartupBehavior = "last_workspace" | "welcome";

/// Build- and run-time facts about the app itself, shown in Settings.
/// Any field can come back empty when it couldn't be determined (no git
/// checkout, no node on PATH at build time) - the UI words that case.
export interface AppInfo {
	name: string;
	version: string;
	build_date: string;
	commit: string;
	os: string;
	arch: string;
	rust_version: string;
	tauri_version: string;
	node_version: string;
	webview_version: string;
	/// Sent as `User-Agent` while the setting is left empty.
	default_user_agent: string;
}

/// A request together with the path it lives at (renaming moves the file).
export interface RequestAtPath extends RequestFile {
	path: string;
}

export type EnvironmentScope = "global" | "collection";

export interface EnvironmentMeta extends SyncMeta {
	name: string;
	scope: EnvironmentScope;
}

export interface Variable {
	id: Id;
	key: string;
	value: string;
	enabled: boolean;
	secret: boolean;
}

export interface EnvironmentFile {
	meta: EnvironmentMeta;
	variables: Variable[];
}

/// An environment paired with its on-disk file path (not part of the
/// persisted entity itself - see src-tauri EnvironmentEntry).
export interface EnvironmentEntry extends EnvironmentFile {
	path: string;
}

export type TraceLevel = "info" | "warn" | "error";

/// One line of the request log, stamped with how far into the request it
/// happened.
export interface TraceEvent {
	at_ms: number;
	level: TraceLevel;
	message: string;
}

/// Where the time went. A phase is `null` when it did not happen or could
/// not be measured - a request on a pooled connection has no DNS or connect
/// phase at all - while 0 means it happened in under a millisecond. Don't
/// render the two the same way. Only `total_ms` is always known, failed
/// attempts included.
export interface ExecutionTrace {
	dns_ms: number | null;
	connect_ms: number | null;
	tls_ms: number | null;
	send_ms: number | null;
	wait_ms: number | null;
	download_ms: number | null;
	total_ms: number;
	reused_connection: boolean | null;
	remote_addr: string | null;
	events: TraceEvent[];
}

export interface ExecutionOutcome {
	status: number;
	status_text: string;
	headers: KeyValue[];
	body_base64: string;
	trace: ExecutionTrace;
	unresolved_variables: string[];
}

/// One event of a Server-Sent Events stream. `at_ms` is how far into the
/// request it arrived; `event` is "message" when the server named no type.
export interface SseEvent {
	kind: "event";
	at_ms: number;
	id: string | null;
	event: string;
	data: string;
	retry_ms: number | null;
}

/// A `:` line - ignored by a real client, shown here because a heartbeat
/// that stops arriving is often the thing being debugged.
export interface SseComment {
	kind: "comment";
	at_ms: number;
	text: string;
}

export type StreamEnd =
	| { type: "closed" }
	| { type: "cancelled" }
	/// The connection broke after the stream started; the events before it
	/// still stand.
	| { type: "failed"; message: string }
	/// The server answered with something other than an event stream, which
	/// was read whole and is shown as an ordinary response.
	| { type: "not_a_stream"; body_base64: string };

export interface SseOutcome {
	status: number;
	status_text: string;
	headers: KeyValue[];
	events: number;
	end: StreamEnd;
}

export interface SseEnd {
	kind: "end";
	outcome: SseOutcome;
	trace: ExecutionTrace;
	unresolved_variables: string[];
}

/// Everything `open_sse_stream` sends down its channel, in order. `end` is
/// always last - it travels on the channel rather than as the command's
/// result so it can't overtake the final events.
export type SseMessage =
	| { kind: "open"; status: number; status_text: string; headers: KeyValue[] }
	| SseEvent
	| SseComment
	| SseEnd;

/// One WebSocket message, either way, or a ping/pong. `data` is the text of
/// a text message and base64 for anything else - binary is never decoded
/// as text on its way here. `size` is in bytes.
export interface WsFrame {
	kind: "frame";
	at_ms: number;
	direction: "sent" | "received";
	opcode: "text" | "binary" | "ping" | "pong";
	data: string;
	size: number;
	/// For a sent message: names in it that stayed `{{literal}}`.
	unresolved_variables?: string[];
}

export type SocketEnd =
	/// `code` is null when the server's close frame carried none.
	| { type: "closed"; code: number | null; reason: string }
	| { type: "cancelled" }
	| { type: "failed"; message: string }
	/// The server refused to switch protocols; its answer is shown as an
	/// ordinary response.
	| { type: "rejected"; body_base64: string };

export interface WsOutcome {
	status: number;
	status_text: string;
	headers: KeyValue[];
	/// Data messages only, pings and pongs not counted.
	sent: number;
	received: number;
	end: SocketEnd;
}

export interface WsEnd {
	kind: "end";
	outcome: WsOutcome;
	trace: ExecutionTrace;
	unresolved_variables: string[];
}

/// Everything `open_websocket` sends down its channel, `end` always last.
export type WsMessage =
	| { kind: "open"; status: number; status_text: string; headers: KeyValue[] }
	| WsFrame
	| WsEnd;

/// Languages the code editor can highlight. Kept next to the domain types
/// because both the body format and the response viewer map onto it.
export type EditorLanguage = "json" | "xml" | "yaml" | "edn" | "html" | "css" | "javascript" | "text";

export function newKeyValue(): KeyValue {
	return { key: "", value: "", enabled: true };
}

const ULID_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Client-side ULID, matching the format the Rust side generates (see
/// domain::ids::Id) - ids must stay lexicographically time-sortable for the
/// planned change-log sync, which a UUIDv4 would break.
export function newId(): Id {
	let timestamp = "";
	let now = Date.now();
	for (let i = 0; i < 10; i++) {
		timestamp = ULID_ALPHABET[now % 32] + timestamp;
		now = Math.floor(now / 32);
	}
	const random = crypto.getRandomValues(new Uint8Array(16));
	let suffix = "";
	for (let i = 0; i < 16; i++) suffix += ULID_ALPHABET[random[i] % 32];
	return timestamp + suffix;
}

/// A request that exists only in memory (an incognito send). It carries the
/// same shape as one read from disk, ids included, so saving it later is a
/// plain write rather than a conversion.
export function newRequestFile(name: string): RequestFile {
	const now = new Date().toISOString();
	return {
		meta: {
			id: newId(),
			created_at: now,
			updated_at: now,
			version: 1,
			name,
			seq: 1,
			protocol: "http",
		},
		http: newHttpRequestSpec(),
	};
}

export function newWebSocketSpec(): WebSocketSpec {
	return { format: "json", message: "" };
}

export function newHttpRequestSpec(method: HttpMethod = "GET"): HttpRequestSpec {
	return {
		method,
		url: "",
		query: [],
		headers: [],
		auth: { type: "none" },
		body: { type: "none" },
	};
}
