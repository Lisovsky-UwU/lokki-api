import { writable } from "svelte/store";
import type { CollectionSummary, WorkspaceFile } from "../bindings/types";

export const workspacePath = writable<string | null>(null);
export const workspace = writable<WorkspaceFile | null>(null);
export const collections = writable<CollectionSummary[]>([]);
