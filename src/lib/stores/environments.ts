import { writable } from "svelte/store";
import type { EnvironmentEntry } from "../bindings/types";

export const globalEnvironments = writable<EnvironmentEntry[]>([]);
export const collectionEnvironments = writable<EnvironmentEntry[]>([]);
export const activeGlobalEnvironmentId = writable<string | null>(null);
export const activeCollectionEnvironmentId = writable<string | null>(null);
