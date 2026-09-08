import { writable } from "svelte/store";
import type { CollectionSummary } from "../bindings/types";

export const activeCollection = writable<CollectionSummary | null>(null);

/// Bumped whenever a request/folder is created, saved, or deleted anywhere
/// in the app, so the sidebar (which caches its own loaded trees) knows to
/// re-fetch — otherwise e.g. a renamed/re-methoded request stays stale in
/// the tree until the collection is collapsed and re-expanded.
export const treeRefreshToken = writable(0);
export function requestTreeRefresh() {
	treeRefreshToken.update((n) => n + 1);
}
