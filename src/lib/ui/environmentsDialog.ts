import { writable } from "svelte/store";
import type { EnvironmentScope } from "../bindings/types";

export interface EnvironmentsDialogTarget {
	/// Workspace root for global scope, collection root for collection scope.
	rootPath: string;
	scope: EnvironmentScope;
	/// What the dialog is about — the workspace or the collection name.
	title: string;
}

/// Global and collection environments deliberately never share a dialog:
/// mixing two scopes in one list is how you edit the wrong one. Opening is a
/// store so both the top-bar switcher and the collection menu can raise the
/// same dialog, rendered once at the app root.
export const environmentsDialog = writable<EnvironmentsDialogTarget | null>(null);

export function openEnvironmentsDialog(target: EnvironmentsDialogTarget) {
	environmentsDialog.set(target);
}

export function closeEnvironmentsDialog() {
	environmentsDialog.set(null);
}
