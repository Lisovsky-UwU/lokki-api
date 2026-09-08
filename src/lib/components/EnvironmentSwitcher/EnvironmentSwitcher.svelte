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
	import EnvironmentEditorModal from "./EnvironmentEditorModal.svelte";

	let editing = $state<EnvironmentEntry | null>(null);

	async function refreshGlobal() {
		const path = $workspacePath;
		if (!path) return;
		globalEnvironments.set(await api.listEnvironments(path));
		const active = await api.getActiveEnvironment(path);
		activeGlobalEnvironmentId.set(active?.meta.id ?? null);
	}

	async function refreshCollection() {
		const collection = $activeCollection;
		if (!collection) {
			collectionEnvironments.set([]);
			activeCollectionEnvironmentId.set(null);
			return;
		}
		collectionEnvironments.set(await api.listEnvironments(collection.path));
		const active = await api.getActiveEnvironment(collection.path);
		activeCollectionEnvironmentId.set(active?.meta.id ?? null);
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

	async function createGlobal() {
		const path = $workspacePath;
		const name = prompt("Название глобального окружения:");
		if (!path || !name) return;
		const entry = await api.createEnvironment(path, name, "global" as EnvironmentScope);
		await refreshGlobal();
		editing = entry;
	}

	async function createCollectionEnv() {
		const collection = $activeCollection;
		const name = prompt("Название окружения коллекции:");
		if (!collection || !name) return;
		const entry = await api.createEnvironment(collection.path, name, "collection" as EnvironmentScope);
		await refreshCollection();
		editing = entry;
	}

	function findGlobal(id: string | null) {
		return $globalEnvironments.find((e) => e.meta.id === id) ?? null;
	}
	function findCollection(id: string | null) {
		return $collectionEnvironments.find((e) => e.meta.id === id) ?? null;
	}
</script>

<div class="switcher">
	<div class="group">
		<label for="global-env">Global</label>
		<select id="global-env" value={$activeGlobalEnvironmentId ?? ""} onchange={(e) => selectGlobal((e.target as HTMLSelectElement).value)}>
			<option value="">—</option>
			{#each $globalEnvironments as env (env.meta.id)}
				<option value={env.meta.id}>{env.meta.name}</option>
			{/each}
		</select>
		{#if $activeGlobalEnvironmentId}
			<button class="edit" title="Редактировать" onclick={() => (editing = findGlobal($activeGlobalEnvironmentId))}>✎</button>
		{/if}
		<button class="add" title="Новое глобальное окружение" onclick={createGlobal}>+</button>
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
			{#if $activeCollectionEnvironmentId}
				<button class="edit" title="Редактировать" onclick={() => (editing = findCollection($activeCollectionEnvironmentId))}>✎</button>
			{/if}
			<button class="add" title="Новое окружение коллекции" onclick={createCollectionEnv}>+</button>
		</div>
	{/if}
</div>

{#if editing && $workspacePath}
	<EnvironmentEditorModal
		environment={editing}
		workspacePath={$workspacePath}
		onClose={() => (editing = null)}
		onSaved={(saved) => {
			if (saved.meta.scope === "global") {
				globalEnvironments.update((list) => list.map((e) => (e.path === saved.path ? saved : e)));
			} else {
				collectionEnvironments.update((list) => list.map((e) => (e.path === saved.path ? saved : e)));
			}
		}}
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
	}
	.edit,
	.add {
		background: none;
		border: none;
		cursor: pointer;
		color: inherit;
		opacity: 0.6;
		padding: 0.1em 0.3em;
	}
	.edit:hover,
	.add:hover {
		opacity: 1;
	}
</style>
