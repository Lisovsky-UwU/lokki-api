<script lang="ts">
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { workspace, workspacePath } from "../../stores/workspace";
	import { activeCollection } from "../../stores/collectionTree";
	import {
		globalEnvironments,
		collectionEnvironments,
		activeGlobalEnvironmentId,
		activeCollectionEnvironmentId,
		environmentsRefreshToken,
	} from "../../stores/environments";
	import type { EnvironmentEntry } from "../../bindings/types";
	import { openEnvironmentsDialog } from "../../ui/environmentsDialog";
	import { reportError } from "../../ui/notices";

	/// Loads a scope's environments and which one is active. Both go through
	/// one call site with error handling - previously a failed list left the
	/// dropdown empty while the active id survived, so the edit button had
	/// nothing to open and appeared to do nothing.
	async function loadScope(rootPath: string): Promise<{ entries: EnvironmentEntry[]; activeId: string | null }> {
		try {
			const entries = await api.listEnvironments(rootPath);
			const active = await api.getActiveEnvironment(rootPath);
			return { entries, activeId: active?.meta.id ?? null };
		} catch (e) {
			reportError($t("env.loadFailed"), e);
			return { entries: [], activeId: null };
		}
	}

	async function refreshGlobal() {
		const path = $workspacePath;
		if (!path) return;
		const { entries, activeId } = await loadScope(path);
		globalEnvironments.set(entries);
		activeGlobalEnvironmentId.set(activeId);
	}

	async function refreshCollection() {
		const collection = $activeCollection;
		if (!collection) {
			collectionEnvironments.set([]);
			activeCollectionEnvironmentId.set(null);
			return;
		}
		const { entries, activeId } = await loadScope(collection.path);
		collectionEnvironments.set(entries);
		activeCollectionEnvironmentId.set(activeId);
	}

	// The refresh token is what the environments dialog signals with: it is
	// rendered at the app root, not here, so it can't call these directly.
	$effect(() => {
		$environmentsRefreshToken;
		if ($workspacePath) refreshGlobal();
	});
	$effect(() => {
		$environmentsRefreshToken;
		refreshCollection();
	});

	async function selectGlobal(id: string) {
		const path = $workspacePath;
		if (!path) return;
		await api.setActiveEnvironment(path, id || null);
		activeGlobalEnvironmentId.set(id || null);
	}

	async function selectCollection(id: string) {
		const collection = $activeCollection;
		if (!collection) return;
		await api.setActiveEnvironment(collection.path, id || null);
		activeCollectionEnvironmentId.set(id || null);
	}
</script>

<div class="switcher">
	<label for="global-env">{$t("env.label")}</label>
	<div class="group">
		<label for="global-env">{$t("env.global")}</label>
		<select
			id="global-env"
			value={$activeGlobalEnvironmentId ?? ""}
			onchange={(e) => selectGlobal((e.target as HTMLSelectElement).value)}
		>
			<option value="">{$t("env.none")}</option>
			{#each $globalEnvironments as env (env.meta.id)}
				<option value={env.meta.id}>{env.meta.name}</option>
			{/each}
		</select>
		<button
			class="icon"
			title={$t("env.workspaceEnvironments")}
			aria-label={$t("env.workspaceEnvironments")}
			disabled={!$workspacePath}
			onclick={() =>
				$workspacePath &&
				openEnvironmentsDialog({
					rootPath: $workspacePath,
					scope: "global",
					title: $workspace?.name ?? "",
				})}
		>
			✎
		</button>
	</div>

	{#if $activeCollection}
		<div class="group">
			<label for="collection-env">{$t("env.collection", { name: $activeCollection.name })}</label>
			<select
				id="collection-env"
				value={$activeCollectionEnvironmentId ?? ""}
				onchange={(e) => selectCollection((e.target as HTMLSelectElement).value)}
			>
				<option value="">{$t("env.none")}</option>
				{#each $collectionEnvironments as env (env.meta.id)}
					<option value={env.meta.id}>{env.meta.name}</option>
				{/each}
			</select>
			<button
				class="icon"
				title={$t("env.collectionEnvironments")}
				aria-label={$t("env.collectionEnvironments")}
				onclick={() =>
					$activeCollection &&
					openEnvironmentsDialog({
						rootPath: $activeCollection.path,
						scope: "collection",
						title: $activeCollection.name,
					})}
			>
				✎
			</button>
		</div>
	{/if}
</div>

<style>
	.switcher {
		display: flex;
		gap: 1em;
		align-items: center;
		font-size: 0.85em;
	}
	.group {
		display: flex;
		align-items: center;
		gap: 0.3em;
	}
	.group label {
		opacity: 0.6;
		font-size: 0.85em;
		max-width: 16em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon {
		background: none;
		border: none;
		cursor: pointer;
		color: inherit;
		opacity: 0.6;
		padding: 0.1em 0.35em;
		border-radius: 4px;
	}
	.icon:hover {
		opacity: 1;
		background: rgba(127, 127, 127, 0.18);
	}
</style>
