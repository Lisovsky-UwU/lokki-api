import type { CollectionTreeNode, HttpMethod } from "../bindings/types";

export const HTTP_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// Badges with a colour of their own. SSE is not a method, but it takes a
/// method's place in the tree: a stream is told apart by what it is, not by
/// the GET that opens it.
const KNOWN = new Set<string>([...HTTP_METHODS, "SSE"]);

/// One palette for the whole app, so a method looks the same in the sidebar
/// tree and in the request editor's selector. The values live with the rest
/// of the palette in `+page.svelte`, because each theme needs its own.
export function methodColor(method: string | null | undefined): string {
	const key = (method ?? "").toUpperCase();
	return KNOWN.has(key) ? `var(--method-${key.toLowerCase()})` : "var(--method-other)";
}

/// What the tree shows in front of a request's name: its method, or for a
/// protocol other than plain HTTP, the protocol.
export function requestBadge(node: Extract<CollectionTreeNode, { kind: "Request" }>): string {
	if (node.protocol === "http" && node.method) return node.method;
	return node.protocol.toUpperCase();
}
