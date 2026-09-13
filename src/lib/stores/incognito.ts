import { writable } from "svelte/store";
import { newRequestFile } from "../bindings/types";
import { activeRequest } from "./activeRequest";
import { activeCollection } from "./collectionTree";
import { forgetResponses } from "./response";

/// The key an incognito request is filed under in the per-path stores
/// (responses, timer, cancellation), so all of that machinery keeps working
/// unchanged. A NUL can't start a real filesystem path, so it can never
/// collide with a saved request.
export const INCOGNITO_PATH = "\u0000incognito";

/// Whether the app is in incognito mode: one request that lives only in
/// memory, with no collection around it.
export const incognito = writable(false);

export function startIncognito() {
	// No collection means no collection environment — that is the rule the
	// mode is built on, and the switcher follows the active collection.
	activeCollection.set(null);
	forgetResponses(INCOGNITO_PATH);
	activeRequest.set({ path: INCOGNITO_PATH, request: newRequestFile("Инкогнито-запрос"), dirty: false });
	incognito.set(true);
}

/// Leaves the mode and drops everything it held: the request and its
/// response were never on disk, and keeping them around after the user
/// closed the mode would defeat the point of it.
export function exitIncognito() {
	incognito.set(false);
	activeRequest.set(null);
	forgetResponses(INCOGNITO_PATH);
}
