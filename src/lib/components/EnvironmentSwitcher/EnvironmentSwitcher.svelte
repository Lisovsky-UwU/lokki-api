<script lang="ts">
	import { api } from "../../api/client";
	import { workspacePath } from "../../stores/workspace";
	import { activeCollection } from "../../stores/collectionTree";
	import {
		globalEnvironments,
		collectionEnvironments,
		activeGlobalEnvironmentId,
		activeCollectionEnvironmentId,
	} from "../../stores/environments";
	import type { EnvironmentEntry, EnvironmentScope } from "../../bindings/types";
	import { promptForText } from "../../ui/dialogs";
	import { reportError } from "../../ui/notices";
	import EnvironmentEditorModal from "./EnvironmentEditorModal.svelte";

	let editing = $state<EnvironmentEntry | null>(null);

	/// Loads a scope's environments and which one is active. Both go through
	/// one call site with error handling — previously a failed list left the
	/// dropdown empty while the active id survived, so the edit button had
	/// nothing to open and appeared to do nothing.
	async function loadScope(rootPath: string): Promise<{ entries: EnvironmentEntry[]; activeId: string | null }> {
		try {
			const entries = await api.listEnvironments(rootPath);
			const active = await api.getActiveEnvironment(rootPath);
			return { entries, activeId: active?.meta.id ?? null };
		} catch (e) {
			reportError("Не удалось загрузить окружения", e);
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

	$effect(() => {
		if ($workspacePath) refreshGlobal();
	});
	$effect(() => {
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

	async function createEnvironment(scope: EnvironmentScope) {
		const rootPath = scope === "global" ? $workspacePath : $activeCollection?.path;
		if (!rootPath) return;
		const title = scope === "global" ? "Новое глобальное окружение" : "Новое окружение коллекции";
		const name = await promptForText(title, "Название окружения", "Новое окружение");
		if (!name) return;

		const entry = await api.createEnvironment(rootPath, name, scope);
		// A freshly created environment becomes the active one, otherwise it
		// would sit unused and unreachable from the edit button.
		await api.setActiveEnvironment(rootPath, entry.meta.id);
		if (scope === "global") await refreshGlobal();
		else await refreshCollection();
		editing = entry;
	}

	/// Opens the editor for whichever environment the scope currently has
	/// selected, re-fetching first if the cached list doesn't contain it.
	async function openEditor(scope: EnvironmentScope) {
		const rootPath = scope === "global" ? $workspacePath : $activeCollection?.path;
		const activeId = scope === "global" ? $activeGlobalEnvironmentId : $activeCollectionEnvironmentId;
		if (!rootPath) return;

		const cached = scope === "global" ? $globalEnvironments : $collectionEnvironments;
		let entry = cached.find((e) => e.meta.id === activeId) ?? null;

		if (!entry) {
			if (scope === "global") await refreshGlobal();
			else await refreshCollection();
			const refreshed = scope === "global" ? $globalEnvironments : $collectionEnvironments;
			const refreshedId = scope === "global" ? $activeGlobalEnvironmentId : $activeCollectionEnvironmentId;
			entry = refreshed.find((e) => e.meta.id === refreshedId) ?? refreshed[0] ?? null;
		}

		if (!entry) {
			reportError("Нет окружения для редактирования", new Error(`scope: ${scope}`));
			return;
		}
		editing = entry;
	}

	function onSaved(saved: EnvironmentEntry) {
		const store = saved.meta.scope === "global" ? globalEnvironments : collectionEnvironments;
		store.update((list) => list.map((e) => (e.path === saved.path ? saved : e)));
	}
</script>

<div class="switcher">
	<div class="group">
		<label for="global-env">Глобальное</label>
		<select
			id="global-env"
			value={$activeGlobalEnvironmentId ?? ""}
			onchange={(e) => selectGlobal((e.target as HTMLSelectElement).value)}
		>
			<option value="">—</option>
			{#each $globalEnvironments as env (env.meta.id)}
				<option value={env.meta.id}>{env.meta.name}</option>
			{/each}
		</select>
		{#if $globalEnvironments.length > 0}
			<button class="icon" title="Редактировать окружение" onclick={() => openEditor("global")}>✎</button>
		{/if}
		<button class="icon" title="Новое глобальное окружение" onclick={() => createEnvironment("global")}>+</button>
	</div>

	{#if $activeCollection}
		<div class="group">
			<label for="collection-env">{$activeCollection.name}</label>
			<select
				id="collection-env"
				value={$activeCollectionEnvironmentId ?? ""}
				onchange={(e) => selectCollection((e.target as HTMLSelectElement).value)}
			>
				<option value="">—</option>
				{#each $collectionEnvironments as env (env.meta.id)}
					<option value={env.meta.id}>{env.meta.name}</option>
				{/each}
			</select>
			{#if $collectionEnvironments.length > 0}
				<button class="icon" title="Редактировать окружение" onclick={() => openEditor("collection")}>✎</button>
			{/if}
			<button class="icon" title="Новое окружение коллекции" onclick={() => createEnvironment("collection")}>+</button>
		</div>
	{/if}
</div>

{#if editing && $workspacePath}
	<EnvironmentEditorModal
		environment={editing}
		workspacePath={$workspacePath}
		onClose={() => (editing = null)}
		{onSaved}
	/>
{/if}

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
		max-width: 12em;
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
