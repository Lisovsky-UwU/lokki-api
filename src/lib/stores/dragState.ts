import { writable } from "svelte/store";

export interface DragPayload {
	path: string;
	parentPath: string;
	/// A collection is dragged only among collections - it has nowhere else
	/// to go - so every drop target below the top level turns one away.
	kind: "Collection" | "Folder" | "Request";
}

/// What is currently being dragged in the sidebar. Kept in a store (rather
/// than only in the DataTransfer) so drop targets can style themselves and
/// reject invalid drops while the drag is in flight.
export const dragging = writable<DragPayload | null>(null);
