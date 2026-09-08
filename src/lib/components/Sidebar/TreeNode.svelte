<script lang="ts">
	import type { CollectionSummary, CollectionTreeNode, HttpMethod } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import { activeRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { api } from "../../api/client";

	// `collection` travels down the tree so opening a request always points
	// the app at the collection that actually owns it — relying on whichever
	// collection was expanded last would resolve environments (and send)
	// against the wrong one.
	let { node, collection }: { node: CollectionTreeNode; collection: CollectionSummary } = $props();

	let expanded = $state(true);

	async function openRequest(path: string) {
		if ($activeRequest?.dirty && $activeRequest.path !== path) {
			if (!confirm("В текущем запросе есть несохранённые изменения. Открыть другой запрос?")) return;
		}
		try {
			const request = await api.loadRequest(path);
			activeCollection.set(collection);
			activeRequest.set({ path, request, dirty: false });
		} catch (e) {
			console.error("failed to load request", e);
		}
	}

	async function addRequest(parentPath: string, e: MouseEvent) {
		e.stopPropagation();
		const name = prompt("Название запроса:");
		if (!name) return;
		await api.createRequest(parentPath, name, "GET" as HttpMethod);
		requestTreeRefresh();
	}

	async function addFolder(parentPath: string, e: MouseEvent) {
		e.stopPropagation();
		const name = prompt("Название папки:");
		if (!name) return;
		await api.createFolder(parentPath, name);
		requestTreeRefresh();
	}

	async function removeFolder(e: MouseEvent) {
		e.stopPropagation();
		if (!confirm(`Удалить папку «${node.name}» со всем содержимым?`)) return;
		await api.deleteFolder(node.path);
		requestTreeRefresh();
	}

	async function removeRequest(e: MouseEvent) {
		e.stopPropagation();
		if (!confirm(`Удалить запрос «${node.name}»?`)) return;
		await api.deleteRequest(node.path);
		if ($activeRequest?.path === node.path) activeRequest.set(null);
		requestTreeRefresh();
	}

	let currentPath = $derived($activeRequest?.path);
</script>

{#if node.kind === "Folder"}
	<div class="folder">
		<div class="node-row">
			<button class="folder-label" onclick={() => (expanded = !expanded)}>
				<span class="chevron" class:collapsed={!expanded}>▾</span>
				{node.name}
			</button>
			<button class="icon-btn" title="Новый запрос" onclick={(e) => addRequest(node.path, e)}>+</button>
			<button class="icon-btn" title="Новая папка" onclick={(e) => addFolder(node.path, e)}>📁</button>
			<button class="icon-btn danger" title="Удалить папку" onclick={removeFolder}>×</button>
		</div>
		{#if expanded}
			<div class="children">
				{#each node.children as child (child.path)}
					<TreeNode node={child} {collection} />
				{/each}
			</div>
		{/if}
	</div>
{:else}
	<div class="node-row" class:active={currentPath === node.path}>
		<button class="request-label" onclick={() => openRequest(node.path)}>
			<span class="method method-{(node.method ?? node.protocol).toLowerCase()}">{node.method ?? node.protocol}</span>
			<span class="request-name">{node.name}</span>
		</button>
		<button class="icon-btn danger" title="Удалить запрос" onclick={removeRequest}>×</button>
	</div>
{/if}

<style>
	.node-row {
		display: flex;
		align-items: center;
		border-radius: 4px;
	}
	.node-row:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.node-row.active {
		background: rgba(57, 108, 216, 0.2);
	}
	.folder-label,
	.request-label {
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
		font-size: 0.9em;
		color: inherit;
	}
	.request-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon-btn {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 0.8em;
		line-height: 1;
		padding: 0.1em 0.3em;
		border-radius: 4px;
		color: inherit;
		opacity: 0;
		flex-shrink: 0;
	}
	.node-row:hover .icon-btn {
		opacity: 0.55;
	}
	.icon-btn:hover {
		opacity: 1 !important;
		background: rgba(127, 127, 127, 0.2);
	}
	.icon-btn.danger:hover {
		color: #d1443c;
	}
	.chevron {
		display: inline-block;
		transition: transform 0.15s;
	}
	.chevron.collapsed {
		transform: rotate(-90deg);
	}
	.children {
		padding-left: 1.1em;
	}
	.method {
		font-size: 0.7em;
		font-weight: 700;
		min-width: 2.8em;
		flex-shrink: 0;
	}
	.method-get {
		color: #2e9e5b;
	}
	.method-post {
		color: #a37c00;
	}
	.method-put,
	.method-patch {
		color: #c26b0f;
	}
	.method-delete {
		color: #d1443c;
	}
</style>
