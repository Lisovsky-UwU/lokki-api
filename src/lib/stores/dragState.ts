import { writable } from "svelte/store";

export interface DragPayload {
	path: string;
	parentPath: string;
	kind: "Folder" | "Request";
}

/// What is currently being dragged in the sidebar. Kept in a store (rather
/// than only in the DataTransfer) so drop targets can style themselves and
/// reject invalid drops while the drag is in flight.
export const dragging = writable<DragPayload | null>(null);
