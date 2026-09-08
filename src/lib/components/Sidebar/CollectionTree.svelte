<script lang="ts">
	import { api } from "../../api/client";
	import { workspacePath, collections } from "../../stores/workspace";
	import type { CollectionSummary, CollectionTreeNode, HttpMethod } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";
	import { activeCollection, treeRefreshToken, requestTreeRefresh } from "../../stores/collectionTree";
	import { dragging } from "../../stores/dragState";
	import { rebaseActiveRequest } from "../../stores/activeRequest";
	import { rekeyResponses } from "../../stores/response";
	import { promptForText } from "../../ui/dialogs";
	import { reportError } from "../../ui/errors";

	let trees = $state<Record<string, CollectionTreeNode | null>>({});
	let expandedCollections = $state<Record<string, boolean>>({});
	let dropTarget = $state<string | null>(null);

	// A drop handled by a child node stops propagation, so this container's
	// own drop/dragleave never fires and its highlight would stay on. Clear
	// it whenever the drag itself is over, wherever it ended.
	$effect(() => {
		if (!$dragging) dropTarget = null;
	});

	async function refreshCollection(collection: CollectionSummary) {
		try {
			trees = { ...trees, [collection.path]: await api.loadCollectionTree(collection.path) };
		} catch (e) {
			reportError("Не удалось загрузить коллекцию", e);
		}
	}

	async function toggleCollection(collection: CollectionSummary) {
		const isExpanded = !expandedCollections[collection.path];
		expandedCollections = { ...expandedCollections, [collection.path]: isExpanded };
		activeCollection.set(collection);
		if (isExpanded && !trees[collection.path]) {
			await refreshCollection(collection);
		}
	}

	// Any request/folder create/save/delete/move anywhere in the app bumps
	// this token — re-fetch every currently-expanded collection's tree so the
	// sidebar never shows stale names, methods or ordering.
	$effect(() => {
		$treeRefreshToken;
		for (const collection of $collections) {
			if (expandedCollections[collection.path]) refreshCollection(collection);
		}
	});

	async function createCollection() {
		const path = $workspacePath;
		const name = await promptForText("Новая коллекция", "Название коллекции", "New Collection");
		if (!path || !name) return;
		const summary = await api.createCollection(path, name);
		collections.update((list) => [...list, summary].sort((a, b) => a.name.localeCompare(b.name)));
		expandedCollections = { ...expandedCollections, [summary.path]: true };
		await refreshCollection(summary);
	}

	async function addRequest(collection: CollectionSummary) {
		const name = await promptForText("Новый запрос", "Название запроса", "New Request");
		if (!name) return;
		await api.createRequest(collection.path, name, "GET" as HttpMethod);
		expandedCollections = { ...expandedCollections, [collection.path]: true };
		await refreshCollection(collection);
	}

	async function addFolder(collection: CollectionSummary) {
		const name = await promptForText("Новая папка", "Название папки", "New Folder");
		if (!name) return;
		await api.createFolder(collection.path, name);
		expandedCollections = { ...expandedCollections, [collection.path]: true };
		await refreshCollection(collection);
	}

	function collectionMenu(collection: CollectionSummary) {
		return [
			{ label: "Добавить запрос", action: () => addRequest(collection) },
			{ label: "Добавить папку", action: () => addFolder(collection) },
		];
	}

	function childrenOf(collectionPath: string): CollectionTreeNode[] {
		const tree = trees[collectionPath];
		return tree && tree.kind === "Folder" ? tree.children : [];
	}

	/// Dropping onto the collection's empty area moves the dragged entry to
	/// the collection root and puts it last.
	async function onRootDrop(collection: CollectionSummary) {
		const payload = $dragging;
		dropTarget = null;
		dragging.set(null);
		if (!payload) return;
		try {
			let sourcePath = payload.path;
			if (payload.parentPath !== collection.path) {
				sourcePath = await api.moveNode(payload.path, collection.path);
				rebaseActiveRequest(payload.path, sourcePath);
				rekeyResponses(payload.path, sourcePath);
			}
			const order = childrenOf(collection.path)
				.map((c) => c.path)
				.filter((p) => p !== payload.path && p !== sourcePath);
			order.push(sourcePath);
			await api.reorderChildren(order);
			requestTreeRefresh();
		} catch (e) {
			reportError("Не удалось переместить", e);
		}
	}
</script>

<div class="sidebar">
	<div class="sidebar-header">
		<span>Коллекции</span>
		<button class="icon-btn" title="Новая коллекция" onclick={createCollection}>+</button>
	</div>

	{#each $collections as collection (collection.path)}
		<div class="collection">
			<div class="collection-header">
				<button class="collection-label" onclick={() => toggleCollection(collection)}>
					<span class="chevron" class:collapsed={!expandedCollections[collection.path]}>▾</span>
					{collection.name}
				</button>
				<NodeMenu items={collectionMenu(collection)} label="Действия с коллекцией" />
			</div>
			{#if expandedCollections[collection.path]}
				{@const children = childrenOf(collection.path)}
				<div
					class="tree"
					class:drop-root={dropTarget === collection.path}
					role="presentation"
					ondragover={(e) => {
						if (!$dragging) return;
						e.preventDefault();
						dropTarget = collection.path;
					}}
					ondragleave={() => (dropTarget = null)}
					ondrop={(e) => {
						e.preventDefault();
						onRootDrop(collection);
					}}
				>
					{#each children as child (child.path)}
						<TreeNode node={child} {collection} parentPath={collection.path} siblings={children} />
					{/each}
					{#if children.length === 0}
						<p class="empty">Пусто — добавьте запрос или папку через меню ⋯</p>
					{/if}
				</div>
			{/if}
		</div>
	{/each}

	{#if $collections.length === 0}
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
	}
	.collection-label {
		display: flex;
		align-items: center;
		gap: 0.4em;
		flex: 1;
		min-width: 0;
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
		padding-bottom: 0.3em;
		border-radius: 4px;
	}
	.tree.drop-root {
		background: rgba(57, 108, 216, 0.12);
		outline: 1px dashed rgba(57, 108, 216, 0.6);
	}
	.empty {
		opacity: 0.6;
		padding: 0.4em 0.6em;
		margin: 0;
		font-size: 0.8em;
	}
</style>
