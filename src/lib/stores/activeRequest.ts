import { writable } from "svelte/store";
import type { RequestFile } from "../bindings/types";

export interface ActiveRequestState {
	path: string;
	request: RequestFile;
	dirty: boolean;
}

export const activeRequest = writable<ActiveRequestState | null>(null);
