<script lang="ts">
	import type { CollectionSummary, CollectionTreeNode, HttpMethod } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";
	import { activeRequest, rebaseActiveRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { dragging } from "../../stores/dragState";
	import { forgetResponses, rekeyResponses } from "../../stores/response";
	import { confirmAction, promptForText } from "../../ui/dialogs";
	import { reportError } from "../../ui/errors";
	import { api } from "../../api/client";

	// `collection` travels down the tree so opening a request always points
	// the app at the collection that actually owns it; `parentPath` and
	// `siblings` are what drag-and-drop needs to reorder within a folder.
	let {
		node,
		collection,
		parentPath,
		siblings,
	}: {
		node: CollectionTreeNode;
		collection: CollectionSummary;
		parentPath: string;
		siblings: CollectionTreeNode[];
	} = $props();

	let expanded = $state(true);
	let dropZone = $state<"before" | "after" | "inside" | null>(null);

	let currentPath = $derived($activeRequest?.path);
	let isDragged = $derived($dragging?.path === node.path);

	// Same reason as in CollectionTree: a drop elsewhere must not leave this
	// row's insertion marker behind.
	$effect(() => {
		if (!$dragging) dropZone = null;
	});

	async function openRequest(path: string) {
		if ($activeRequest?.dirty && $activeRequest.path !== path) {
			const proceed = await confirmAction(
				"В текущем запросе есть несохранённые изменения. Они будут потеряны. Открыть другой запрос?",
			);
			if (!proceed) return;
		}
		try {
			const request = await api.loadRequest(path);
			activeCollection.set(collection);
			activeRequest.set({ path, request, dirty: false });
		} catch (e) {
			reportError("Не удалось открыть запрос", e);
		}
	}

	async function addRequest() {
		const name = await promptForText("Новый запрос", "Название запроса", "New Request");
		if (!name) return;
		await api.createRequest(node.path, name, "GET" as HttpMethod);
		expanded = true;
		requestTreeRefresh();
	}

	async function addFolder() {
		const name = await promptForText("Новая папка", "Название папки", "New Folder");
		if (!name) return;
		await api.createFolder(node.path, name);
		expanded = true;
		requestTreeRefresh();
	}

	async function renameFolder() {
		const name = await promptForText("Переименовать папку", "Название папки", node.name);
		if (!name || name === node.name) return;
		const newPath = await api.renameFolder(node.path, name);
		rebaseActiveRequest(node.path, newPath);
		rekeyResponses(node.path, newPath);
		requestTreeRefresh();
	}

	async function renameRequest() {
		const name = await promptForText("Переименовать запрос", "Название запроса", node.name);
		if (!name || name === node.name) return;
		const renamed = await api.renameRequest(node.path, name);
		rebaseActiveRequest(node.path, renamed.path);
		rekeyResponses(node.path, renamed.path);
		if ($activeRequest?.path === renamed.path) {
			activeRequest.set({ path: renamed.path, request: renamed, dirty: false });
		}
		requestTreeRefresh();
	}

	async function removeFolder() {
		if (!(await confirmAction(`Удалить папку «${node.name}» со всем содержимым?`))) return;
		await api.deleteFolder(node.path);
		requestTreeRefresh();
	}

	async function removeRequest() {
		if (!(await confirmAction(`Удалить запрос «${node.name}»?`))) return;
		await api.deleteRequest(node.path);
		forgetResponses(node.path);
		if ($activeRequest?.path === node.path) activeRequest.set(null);
		requestTreeRefresh();
	}

	function onDragStart(e: DragEvent) {
		dragging.set({ path: node.path, parentPath, kind: node.kind });
		e.dataTransfer?.setData("text/plain", node.path);
		if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
	}

	function onDragEnd() {
		dragging.set(null);
		dropZone = null;
	}

	/// Top/bottom edges reorder around this node; the middle of a folder
	/// drops into it.
	function zoneFor(e: DragEvent): "before" | "after" | "inside" {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const ratio = (e.clientY - rect.top) / rect.height;
		if (node.kind === "Folder") {
			if (ratio < 0.25) return "before";
			if (ratio > 0.75) return "after";
			return "inside";
		}
		return ratio < 0.5 ? "before" : "after";
	}

	function onDragOver(e: DragEvent) {
		const payload = $dragging;
		if (!payload || payload.path === node.path) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
		dropZone = zoneFor(e);
	}

	async function onDrop(e: DragEvent) {
		e.preventDefault();
		e.stopPropagation();
		const payload = $dragging;
		const zone = dropZone;
		dropZone = null;
		dragging.set(null);
		if (!payload || !zone || payload.path === node.path) return;

		try {
			if (zone === "inside") {
				const moved = await api.moveNode(payload.path, node.path);
				rebaseActiveRequest(payload.path, moved);
				rekeyResponses(payload.path, moved);
				expanded = true;
			} else {
				let sourcePath = payload.path;
				if (payload.parentPath !== parentPath) {
					sourcePath = await api.moveNode(payload.path, parentPath);
					rebaseActiveRequest(payload.path, sourcePath);
					rekeyResponses(payload.path, sourcePath);
				}
				const order = siblings.map((s) => s.path).filter((p) => p !== payload.path && p !== sourcePath);
				const anchor = order.indexOf(node.path);
				order.splice(zone === "before" ? anchor : anchor + 1, 0, sourcePath);
				await api.reorderChildren(order);
			}
			requestTreeRefresh();
		} catch (err) {
			reportError("Не удалось переместить", err);
		}
	}

	let folderMenu = $derived([
		{ label: "Добавить запрос", action: addRequest },
		{ label: "Добавить папку", action: addFolder },
		{ label: "Переименовать папку", action: renameFolder },
		{ label: "Удалить папку", action: removeFolder, danger: true },
	]);

	let requestMenu = $derived([
		{ label: "Переименовать запрос", action: renameRequest },
		{ label: "Удалить запрос", action: removeRequest, danger: true },
	]);
</script>

{#if node.kind === "Folder"}
	<div class="folder">
		<div
			class="node-row"
			class:drop-before={dropZone === "before"}
			class:drop-after={dropZone === "after"}
			class:drop-inside={dropZone === "inside"}
			class:dragged={isDragged}
			role="presentation"
			draggable="true"
			ondragstart={onDragStart}
			ondragend={onDragEnd}
			ondragover={onDragOver}
			ondragleave={() => (dropZone = null)}
			ondrop={onDrop}
		>
			<button class="folder-label" onclick={() => (expanded = !expanded)}>
				<span class="chevron" class:collapsed={!expanded}>▾</span>
				<span class="node-name">{node.name}</span>
			</button>
			<NodeMenu items={folderMenu} label="Действия с папкой" />
		</div>
		{#if expanded}
			<div class="children">
				{#each node.children as child (child.path)}
					<TreeNode node={child} {collection} parentPath={node.path} siblings={node.children} />
				{/each}
				{#if node.children.length === 0}
					<p class="empty">Пусто</p>
				{/if}
			</div>
		{/if}
	</div>
{:else}
	<div
		class="node-row"
		class:active={currentPath === node.path}
		class:drop-before={dropZone === "before"}
		class:drop-after={dropZone === "after"}
		class:dragged={isDragged}
		role="presentation"
		draggable="true"
		ondragstart={onDragStart}
		ondragend={onDragEnd}
		ondragover={onDragOver}
		ondragleave={() => (dropZone = null)}
		ondrop={onDrop}
	>
		<button class="request-label" onclick={() => openRequest(node.path)}>
			<span class="method method-{(node.method ?? node.protocol).toLowerCase()}">{node.method ?? node.protocol}</span>
			<span class="node-name">{node.name}</span>
		</button>
		<NodeMenu items={requestMenu} label="Действия с запросом" />
	</div>
{/if}

<style>
	.node-row {
		display: flex;
		align-items: center;
		border-radius: 4px;
		border-top: 2px solid transparent;
		border-bottom: 2px solid transparent;
	}
	.node-row:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.node-row.active {
		background: rgba(57, 108, 216, 0.2);
	}
	.node-row.dragged {
		opacity: 0.4;
	}
	.node-row.drop-before {
		border-top-color: #396cd8;
	}
	.node-row.drop-after {
		border-bottom-color: #396cd8;
	}
	.node-row.drop-inside {
		background: rgba(57, 108, 216, 0.25);
		outline: 1px dashed #396cd8;
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
	.node-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
	.empty {
		margin: 0;
		padding: 0.2em 0.6em;
		opacity: 0.45;
		font-size: 0.8em;
	}
	.method {
		font-size: 0.7em;
		font-weight: 700;
		min-width: 2.8em;
		flex-shrink: 0;
	}
	.method-get {
		color: #6188db;
	}
	.method-post {
		color: #269b2c;
	}
	.method-put {
		color: #c26b0f
	}
	.method-patch {
		color: #e2d138;
	}
	.method-delete {
		color: #d1443c;
	}
</style>
