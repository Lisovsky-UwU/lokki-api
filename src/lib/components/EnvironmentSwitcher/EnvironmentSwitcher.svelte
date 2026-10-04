<script lang="ts">
	import { api } from "../../api/client";
	import Icon from "../common/Icon.svelte";
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

<!-- Each scope is one compact control: the scope's name, the environment
     picked for it, and the way into editing that scope's environments. -->
<div class="switcher">
	<div class="picker">
		<label for="global-env">{$t("env.global")}</label>
		<select
			id="global-env"
			title={$t("env.global")}
			class:none={!$activeGlobalEnvironmentId}
			value={$activeGlobalEnvironmentId ?? ""}
			onchange={(e) => selectGlobal((e.target as HTMLSelectElement).value)}
		>
			<option value="">{$t("env.none")}</option>
			{#each $globalEnvironments as env (env.meta.id)}
				<option value={env.meta.id}>{env.meta.name}</option>
			{/each}
		</select>
		<button
			class="edit"
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
			<Icon name="rename" size="14px" />
		</button>
	</div>

	{#if $activeCollection}
		<div class="picker">
			<label for="collection-env" title={$activeCollection.name}>{$activeCollection.name}</label>
			<select
				id="collection-env"
				title={$activeCollection.name}
				class:none={!$activeCollectionEnvironmentId}
				value={$activeCollectionEnvironmentId ?? ""}
				onchange={(e) => selectCollection((e.target as HTMLSelectElement).value)}
			>
				<option value="">{$t("env.none")}</option>
				{#each $collectionEnvironments as env (env.meta.id)}
					<option value={env.meta.id}>{env.meta.name}</option>
				{/each}
			</select>
			<button
				class="edit"
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
				<Icon name="rename" size="14px" />
			</button>
		</div>
	{/if}
</div>

<style>
	.switcher {
		display: flex;
		gap: 6px;
		align-items: center;
		min-width: 0;
	}
	.picker {
		display: flex;
		align-items: center;
		height: 28px;
		min-width: 0;
		padding-left: 10px;
		border: 1px solid var(--line);
		border-radius: 7px;
		background: var(--surface-raised);
		font-size: var(--fs-sm);
	}
	.picker:focus-within {
		border-color: var(--line-strong);
	}
	label {
		color: var(--text-muted);
		max-width: 12em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* In a narrow window the breadcrumb needs the room more than the scope
	   names do; the select's tooltip still names its scope. */
	@container topbar (max-width: 760px) {
		label {
			display: none;
		}
		.picker {
			padding-left: 4px;
		}
	}
	select {
		height: 100%;
		max-width: 12em;
		border: none;
		background: none;
		padding: 0 4px 0 6px;
		font-weight: 600;
		cursor: pointer;
	}
	select.none {
		font-weight: 400;
		color: var(--text-muted);
	}
	select:focus-visible {
		outline-offset: -2px;
	}
	.edit {
		display: flex;
		align-items: center;
		height: 100%;
		padding: 0 7px;
		border: none;
		border-left: 1px solid var(--line);
		border-radius: 0 6px 6px 0;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}
	.edit:hover:not(:disabled) {
		color: var(--text);
		background: var(--hover);
	}
	.edit:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
