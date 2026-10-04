import type { CollectionTreeNode, HttpMethod } from "../bindings/types";

export const HTTP_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// One palette for the whole app, so a method looks the same in the sidebar
/// tree and in the request editor's selector.
const COLORS: Record<string, string> = {
	GET: "#6188db",
	POST: "#269b2c",
	PUT: "#c26b0f",
	PATCH: "#e2d138",
	DELETE: "#d1443c",
	HEAD: "#6f42c1",
	OPTIONS: "#0d9488",
	// Not a method, but it takes a method's place in the tree: a stream is
	// told apart by what it is, not by the GET that opens it.
	SSE: "#b4489c",
};

export function methodColor(method: string | null | undefined): string {
	return COLORS[(method ?? "").toUpperCase()] ?? "#6e7781";
}

/// What the tree shows in front of a request's name: its method, or for a
/// protocol other than plain HTTP, the protocol.
export function requestBadge(node: Extract<CollectionTreeNode, { kind: "Request" }>): string {
	if (node.protocol === "http" && node.method) return node.method;
	return node.protocol.toUpperCase();
}
