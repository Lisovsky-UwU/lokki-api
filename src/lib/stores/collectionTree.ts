import { writable } from "svelte/store";
import type { CollectionSummary, CollectionTreeNode } from "../bindings/types";

export const activeCollection = writable<CollectionSummary | null>(null);
export const collectionTree = writable<CollectionTreeNode | null>(null);
