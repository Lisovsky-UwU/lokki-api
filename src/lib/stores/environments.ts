import { derived, writable } from "svelte/store";
import type { EnvironmentEntry } from "../bindings/types";

export const globalEnvironments = writable<EnvironmentEntry[]>([]);
export const collectionEnvironments = writable<EnvironmentEntry[]>([]);
export const activeGlobalEnvironmentId = writable<string | null>(null);
export const activeCollectionEnvironmentId = writable<string | null>(null);

/// Bumped whenever environments are created, renamed or edited, so the
/// top-bar switcher re-reads them - the dialog that changes them is rendered
/// elsewhere and has no other way to say "the lists are stale".
export const environmentsRefreshToken = writable(0);
export function requestEnvironmentsRefresh() {
	environmentsRefreshToken.update((n) => n + 1);
}

/// Characters a variable name may contain. Must stay in step with the
/// interpolation regex in the Rust core (`interpolate::var_regex`) -
/// anything outside this set, a space in particular, silently fails to
/// resolve at send time.
export const VARIABLE_NAME_PATTERN = /^[A-Za-z0-9_.-]+$/;

export function isValidVariableName(name: string): boolean {
	return VARIABLE_NAME_PATTERN.test(name);
}

export function sanitizeVariableName(name: string): string {
	return name.replace(/[^A-Za-z0-9_.-]/g, "");
}

export interface VariableSuggestion {
	key: string;
	value: string;
	secret: boolean;
	scope: "collection" | "global";
}

/// Every variable the active environments make available, in the same
/// precedence the core uses when resolving: collection scope shadows global.
export const availableVariables = derived(
	[globalEnvironments, collectionEnvironments, activeGlobalEnvironmentId, activeCollectionEnvironmentId],
	([$global, $collection, $globalId, $collectionId]): VariableSuggestion[] => {
		const byKey = new Map<string, VariableSuggestion>();

		const globalEnv = $global.find((e) => e.meta.id === $globalId);
		for (const v of globalEnv?.variables ?? []) {
			if (v.enabled && v.key) byKey.set(v.key, { key: v.key, value: v.value, secret: v.secret, scope: "global" });
		}

		const collectionEnv = $collection.find((e) => e.meta.id === $collectionId);
		for (const v of collectionEnv?.variables ?? []) {
			if (v.enabled && v.key) byKey.set(v.key, { key: v.key, value: v.value, secret: v.secret, scope: "collection" });
		}

		return [...byKey.values()].sort((a, b) => a.key.localeCompare(b.key));
	},
);
