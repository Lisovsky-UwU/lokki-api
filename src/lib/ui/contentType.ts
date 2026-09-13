import type { EditorLanguage, KeyValue } from "../bindings/types";

/// How a response should be shown: as highlighted text, as a rendered page,
/// as a picture, or as a file that only makes sense saved to disk.
export type ResponseKind = "text" | "html" | "image" | "binary";

export interface ResponseFormat {
	kind: ResponseKind;
	/// Highlighting for the source view. Meaningful for `text` and `html`,
	/// and for SVG, which is both a picture and markup.
	language: EditorLanguage;
	/// Media type without its parameters (`application/json`, not
	/// `application/json; charset=utf-8`).
	mediaType: string;
	/// What to call the file if the user saves the body.
	fileName: string;
}

export function headerValue(headers: KeyValue[], name: string): string | null {
	return headers.find((h) => h.key.toLowerCase() === name.toLowerCase())?.value ?? null;
}

const EXTENSIONS: Record<string, string> = {
	"application/json": "json",
	"application/xml": "xml",
	"text/xml": "xml",
	"text/html": "html",
	"text/css": "css",
	"text/plain": "txt",
	"text/csv": "csv",
	"application/javascript": "js",
	"text/javascript": "js",
	"application/yaml": "yaml",
	"text/yaml": "yaml",
	"application/edn": "edn",
	"application/pdf": "pdf",
	"application/zip": "zip",
	"image/png": "png",
	"image/jpeg": "jpg",
	"image/gif": "gif",
	"image/webp": "webp",
	"image/svg+xml": "svg",
};

/// Filename from `Content-Disposition`, which is what a server sends when it
/// means "this is a download". Handles both `filename=` and the RFC 5987
/// `filename*=` form.
function dispositionFileName(headers: KeyValue[]): string | null {
	const disposition = headerValue(headers, "content-disposition");
	if (!disposition) return null;
	const encoded = /filename\*=(?:UTF-8'')?([^;]+)/i.exec(disposition);
	if (encoded) {
		try {
			return decodeURIComponent(encoded[1].trim().replace(/^"|"$/g, ""));
		} catch {
			// A malformed filename* is not worth failing over; fall through to
			// the plain form below.
		}
	}
	const plain = /filename="?([^";]+)"?/i.exec(disposition);
	return plain ? plain[1].trim() : null;
}

function languageFor(mediaType: string): EditorLanguage | null {
	if (/(^|\/|\+)json$/.test(mediaType)) return "json";
	if (/(^|\/|\+)xml$/.test(mediaType) || mediaType === "text/xml") return "xml";
	if (/ya?ml/.test(mediaType)) return "yaml";
	if (mediaType.endsWith("edn")) return "edn";
	if (mediaType === "text/css") return "css";
	if (/javascript|ecmascript/.test(mediaType)) return "javascript";
	if (mediaType === "text/html" || mediaType === "application/xhtml+xml") return "html";
	if (mediaType.startsWith("text/")) return "text";
	return null;
}

/// Text that came back as bytes the decoder couldn't make sense of. A
/// replacement character or a NUL means this was never text, whatever the
/// server labelled it.
const NUL = String.fromCharCode(0);
const REPLACEMENT_CHAR = String.fromCharCode(0xfffd);

function looksBinary(text: string): boolean {
	return text.includes(NUL) || text.includes(REPLACEMENT_CHAR);
}

/// Guesses from the body when the server sent no usable type — common enough
/// with hand-rolled APIs to be worth doing rather than falling back to
/// "binary" and hiding the response.
function sniff(text: string): EditorLanguage {
	const head = text.trimStart().slice(0, 200).toLowerCase();
	if (/^<!doctype html|^<html/.test(head)) return "html";
	if (head.startsWith("<")) return "xml";
	const trimmed = text.trim();
	if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
		try {
			JSON.parse(trimmed);
			return "json";
		} catch {
			// Not JSON after all — treat it as plain text.
		}
	}
	return "text";
}

export function mediaTypeOf(headers: KeyValue[]): string {
	return (headerValue(headers, "content-type") ?? "").split(";")[0].trim().toLowerCase();
}

/// Whether the body is worth decoding into a string at all. A 20 MB PDF or
/// image would otherwise be turned into a useless 20 MB of text on every
/// render just to be thrown away. An absent type still decodes: that is the
/// case `sniff` exists for.
export function isTextualMediaType(mediaType: string): boolean {
	if (mediaType === "") return true;
	if (mediaType === "image/svg+xml") return true;
	if (mediaType.startsWith("image/") || mediaType.startsWith("audio/") || mediaType.startsWith("video/")) return false;
	if (mediaType.startsWith("font/")) return false;
	return mediaType.startsWith("text/") || languageFor(mediaType) !== null;
}

export function detectFormat(headers: KeyValue[], bodyText: string): ResponseFormat {
	const mediaType = mediaTypeOf(headers);
	const named = dispositionFileName(headers);

	const language = languageFor(mediaType) ?? (mediaType === "" ? sniff(bodyText) : null);
	const isImage = mediaType.startsWith("image/");
	const kind: ResponseKind = isImage
		? "image"
		: language === "html"
			? "html"
			: language && !looksBinary(bodyText)
				? "text"
				: "binary";

	const extension = EXTENSIONS[mediaType] ?? (kind === "binary" ? "bin" : "txt");
	return {
		kind,
		// SVG is a picture and markup at once, so its source view stays XML.
		language: mediaType === "image/svg+xml" ? "xml" : (language ?? "text"),
		mediaType: mediaType || "application/octet-stream",
		fileName: named ?? `response.${extension}`,
	};
}
