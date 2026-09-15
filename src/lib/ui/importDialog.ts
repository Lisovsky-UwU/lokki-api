import { writable } from "svelte/store";

/// Raised from the sidebar header and rendered at the app root, the same way
/// the environments dialog is: the import creates a collection in the
/// workspace, so it belongs to the app shell rather than to the tree it adds
/// a row to.
export const importDialogOpen = writable(false);

export function openImportDialog() {
	importDialogOpen.set(true);
}

export function closeImportDialog() {
	importDialogOpen.set(false);
}
