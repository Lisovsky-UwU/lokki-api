// Hand-written mirror of the Rust domain types in src-tauri/src/domain/*.
// Field names match serde's wire format exactly (snake_case, since the
// same structs also serialize to the on-disk TOML files) — only Tauri
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

export type BodySpec =
	| { type: "none" }
	| { type: "raw"; content: string }
	| { type: "json"; content: string }
	| { type: "form"; fields: KeyValue[] };

export interface HttpRequestSpec {
	method: HttpMethod;
	url: string;
	query: KeyValue[];
	headers: KeyValue[];
	auth: AuthSpec;
	body: BodySpec;
}

export interface RequestFile {
	meta: RequestMeta;
	http?: HttpRequestSpec;
}

export interface WorkspaceFile extends SyncMeta {
	name: string;
	schema_version: number;
}

export interface CollectionSummary {
	name: string;
	path: string;
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

/// Build- and run-time facts about the app itself, shown in Settings.
/// Any field can come back empty when it couldn't be determined (no git
/// checkout, no node on PATH at build time) — the UI words that case.
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
/// persisted entity itself — see src-tauri EnvironmentEntry).
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
/// not be measured — a request on a pooled connection has no DNS or connect
/// phase at all — while 0 means it happened in under a millisecond. Don't
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

export function newKeyValue(): KeyValue {
	return { key: "", value: "", enabled: true };
}

const ULID_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Client-side ULID, matching the format the Rust side generates (see
/// domain::ids::Id) — ids must stay lexicographically time-sortable for the
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
