import { ask } from "@tauri-apps/plugin-dialog";
import { writable } from "svelte/store";

/// The webview's built-in `window.confirm` / `window.prompt` are unreliable
/// inside Tauri (the unsaved-changes confirmation never appeared), so
/// confirmations go through the native dialog plugin and text input goes
/// through the app's own dialog below.
export async function confirmAction(message: string, title = "LokkiAPI"): Promise<boolean> {
	try {
		return await ask(message, { title, kind: "warning" });
	} catch (e) {
		console.error("confirm dialog failed", e);
		return false;
	}
}

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
