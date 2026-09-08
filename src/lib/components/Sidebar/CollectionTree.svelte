<script lang="ts">
	import { api } from "../../api/client";
	import { workspacePath, collections } from "../../stores/workspace";
	import type { CollectionSummary, CollectionTreeNode } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import { activeCollection } from "../../stores/collectionTree";

	let trees = $state<Record<string, CollectionTreeNode | null>>({});
	let expandedCollections = $state<Record<string, boolean>>({});
	let creatingCollection = $state(false);
	let newCollectionName = $state("");

	async function toggleCollection(collection: CollectionSummary) {
		const isExpanded = !expandedCollections[collection.path];
		expandedCollections = { ...expandedCollections, [collection.path]: isExpanded };
		activeCollection.set(collection);
		if (isExpanded && !trees[collection.path]) {
			try {
				trees = { ...trees, [collection.path]: await api.loadCollectionTree(collection.path) };
			} catch (e) {
				console.error("failed to load collection tree", e);
			}
		}
	}

	async function refreshCollection(collection: CollectionSummary) {
		trees = { ...trees, [collection.path]: await api.loadCollectionTree(collection.path) };
	}

	async function createCollection() {
		const path = $workspacePath;
		const name = newCollectionName.trim();
		if (!path || !name) return;
		const summary = await api.createCollection(path, name);
		collections.update((list) => [...list, summary].sort((a, b) => a.name.localeCompare(b.name)));
		newCollectionName = "";
		creatingCollection = false;
	}

	async function addRequest(collection: CollectionSummary) {
		const name = prompt("Название запроса:");
		if (!name) return;
		await api.createRequest(collection.path, name, "GET");
		await refreshCollection(collection);
	}
</script>

<div class="sidebar">
	<div class="sidebar-header">
		<span>Коллекции</span>
		<button class="icon-btn" title="Новая коллекция" onclick={() => (creatingCollection = true)}>+</button>
	</div>

	{#if creatingCollection}
		<form
			class="new-collection"
			onsubmit={(e) => {
				e.preventDefault();
				createCollection();
			}}
		>
			<input placeholder="Имя коллекции" bind:value={newCollectionName} />
			<button type="submit">OK</button>
		</form>
	{/if}

	{#each $collections as collection (collection.path)}
		<div class="collection">
			<div class="collection-header">
				<button class="collection-label" onclick={() => toggleCollection(collection)}>
					<span class="chevron" class:collapsed={!expandedCollections[collection.path]}>▾</span>
					{collection.name}
				</button>
				<button class="icon-btn" title="Новый запрос" onclick={() => addRequest(collection)}>+</button>
			</div>
			{#if expandedCollections[collection.path]}
				{@const tree = trees[collection.path]}
				{#if tree && tree.kind === "Folder"}
					<div class="tree">
						{#each tree.children as child (child.path)}
							<TreeNode node={child} />
						{/each}
					</div>
				{/if}
			{/if}
		</div>
	{/each}

	{#if $collections.length === 0 && !creatingCollection}
		<p class="empty">Нет коллекций. Создайте первую.</p>
	{/if}
</div>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow-y: auto;
		padding: 0.5rem;
		font-size: 0.9em;
	}
	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-weight: 600;
		padding: 0.3em 0.4em;
		text-transform: uppercase;
		font-size: 0.75em;
		letter-spacing: 0.04em;
		opacity: 0.7;
	}
	.icon-btn {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1em;
		line-height: 1;
		padding: 0.1em 0.4em;
		border-radius: 4px;
		color: inherit;
	}
	.icon-btn:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.collection-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.collection-label {
		display: flex;
		align-items: center;
		gap: 0.4em;
		flex: 1;
		text-align: left;
		background: none;
		border: none;
		padding: 0.3em 0.4em;
		border-radius: 4px;
		cursor: pointer;
		font-weight: 600;
		color: inherit;
	}
	.collection-label:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.chevron {
		display: inline-block;
		transition: transform 0.15s;
	}
	.chevron.collapsed {
		transform: rotate(-90deg);
	}
	.tree {
		padding-left: 0.6em;
	}
	.new-collection {
		display: flex;
		gap: 0.3em;
		padding: 0.3em;
	}
	.new-collection input {
		flex: 1;
		min-width: 0;
	}
	.empty {
		opacity: 0.6;
		padding: 0.5em;
		font-size: 0.85em;
	}
</style>
