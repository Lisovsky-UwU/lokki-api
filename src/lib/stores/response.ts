import { writable } from "svelte/store";
import type { ExecutionOutcome } from "../bindings/types";

export interface ResponseState {
	outcome: ExecutionOutcome | null;
	error: string | null;
	loading: boolean;
}

export const responseState = writable<ResponseState>({ outcome: null, error: null, loading: false });
