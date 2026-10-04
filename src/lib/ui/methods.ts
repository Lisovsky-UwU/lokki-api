import type { CollectionTreeNode, HttpMethod, Protocol } from "../bindings/types";

export const HTTP_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// Badges with a colour of their own. SSE and WS are not methods, but they
/// take a method's place in the tree: a stream or a socket is told apart by
/// what it is, not by the GET that opens it.
const KNOWN = new Set<string>([...HTTP_METHODS, "SSE", "WS"]);

/// One palette for the whole app, so a method looks the same in the sidebar
/// tree and in the request editor's selector. The values live with the rest
/// of the palette in `+page.svelte`, because each theme needs its own.
export function methodColor(method: string | null | undefined): string {
	const key = (method ?? "").toUpperCase();
	return KNOWN.has(key) ? `var(--method-${key.toLowerCase()})` : "var(--method-other)";
}

/// The tree's column is four characters wide; the three methods that don't
/// fit go by the abbreviations other API clients have made familiar.
const SHORT: Record<string, string> = { PATCH: "PTCH", DELETE: "DEL", OPTIONS: "OPT" };

export function shortBadge(badge: string): string {
	return SHORT[badge] ?? badge;
}

/// What the tree shows in front of a request's name: its method, or for a
/// protocol other than plain HTTP, the protocol.
export function requestBadge(node: Extract<CollectionTreeNode, { kind: "Request" }>): string {
	if (node.protocol === "http" && node.method) return node.method;
	return protocolBadge(node.protocol);
}

/// What the name prompt is titled when a request of each kind is created
/// from the tree. GraphQL can't be created yet, so it borrows the plain one.
export const NEW_REQUEST_TITLE = {
	http: "prompt.newRequest",
	sse: "prompt.newSseRequest",
	websocket: "prompt.newWsRequest",
	graphql: "prompt.newRequest",
} as const satisfies Record<Protocol, string>;

/// "websocket" is too long for the tree's four-character column, and "WS"
/// is what everyone calls it anyway.
export function protocolBadge(protocol: Protocol): string {
	return protocol === "websocket" ? "WS" : protocol.toUpperCase();
}
