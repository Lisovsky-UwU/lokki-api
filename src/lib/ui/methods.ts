import type { HttpMethod } from "../bindings/types";

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
};

export function methodColor(method: string | null | undefined): string {
	return COLORS[(method ?? "").toUpperCase()] ?? "#6e7781";
}
