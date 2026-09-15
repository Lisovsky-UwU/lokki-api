import { writable } from "svelte/store";
import { translate } from "../i18n";

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

export interface AlertRequest {
	title: string;
	message: string;
	resolve: () => void;
}

export const alertRequest = writable<AlertRequest | null>(null);

/// Reports a failure the user has to acknowledge before the flow they
/// started carries on. Unlike `reportError`, which toasts off to one side
/// while they do something else, this one blocks - use it where the failure
/// *is* the answer to what they just asked for.
export function alertMessage(message: string, title?: string): Promise<void> {
	return new Promise((resolve) => {
		alertRequest.set({ title: title ?? translate("common.error"), message, resolve });
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
			title: options.title ?? translate("common.confirmTitle"),
			message,
			confirmLabel: options.confirmLabel ?? translate("common.yes"),
			danger: options.danger ?? false,
			resolve,
		});
	});
}
