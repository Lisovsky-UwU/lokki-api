import { writable } from "svelte/store";

/// The webview's built-in `window.confirm` / `window.prompt` are unreliable
/// inside Tauri, and the OS-native dialog plugin looks like a foreign
/// Windows window. Both flows therefore use the app's own modals below.

export interface PromptRequest {
	title: string;
	label: string;
	initial: string;
	resolve: (value: string | null) => void;
}

export const promptRequest = writable<PromptRequest | null>(null);

/// Asks the user for a single line of text. Resolves with the trimmed value,
/// or null if they cancelled or left it empty.
export function promptForText(title: string, label: string, initial = ""): Promise<string | null> {
	return new Promise((resolve) => {
		promptRequest.set({ title, label, initial, resolve });
	});
}

export interface ConfirmRequest {
	title: string;
	message: string;
	confirmLabel: string;
	danger: boolean;
	resolve: (value: boolean) => void;
}

export const confirmRequest = writable<ConfirmRequest | null>(null);

export interface ConfirmOptions {
	title?: string;
	confirmLabel?: string;
	danger?: boolean;
}

export function confirmAction(message: string, options: ConfirmOptions = {}): Promise<boolean> {
	return new Promise((resolve) => {
		confirmRequest.set({
			title: options.title ?? "Подтверждение",
			message,
			confirmLabel: options.confirmLabel ?? "Да",
			danger: options.danger ?? false,
			resolve,
		});
	});
}
