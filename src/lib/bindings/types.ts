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

export interface ExecutionOutcome {
	status: number;
	status_text: string;
	headers: KeyValue[];
	body_base64: string;
	duration_ms: number;
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
