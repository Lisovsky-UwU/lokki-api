<script lang="ts">
	import type { CollectionSummary, CollectionTreeNode, HttpMethod, Protocol } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";
	import ActivityIndicator from "../common/ActivityIndicator.svelte";
	import { activeRequest, rebaseActiveRequest } from "../../stores/activeRequest";
	import {
		FOLDER_DEFAULT_EXPANDED,
		expandedPaths,
		forgetExpandedUnder,
		isExpanded,
		rebaseExpanded,
		setExpanded,
		toggleExpanded,
	} from "../../stores/expansion";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { dragging } from "../../stores/dragState";
	import { forgetResponses, markSeen, rekeyResponses, responsesByRequest, subtreeActivity } from "../../stores/response";
	import { confirmAction, promptForText } from "../../ui/dialogs";
	import { openContextMenu, type Menu } from "../../ui/contextMenu";
	import { reportError } from "../../ui/notices";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { methodColor, requestBadge } from "../../ui/methods";

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

	// Not component state: it has to survive both this component unmounting
	// (the tree is re-fetched and rebuilt on every refresh token) and the
	// app restarting.
	let expanded = $derived(isExpanded($expandedPaths, node.path, FOLDER_DEFAULT_EXPANDED));
	let dropZone = $state<"before" | "after" | "inside" | null>(null);

	let currentPath = $derived($activeRequest?.path);
	let badge = $derived(node.kind === "Request" ? requestBadge(node) : "");
	let isDragged = $derived($dragging?.path === node.path);

	// A folder stands in for its subtree only while collapsed - expanded, the
	// rows carry their own indicators and repeating them just adds noise.
	let activity = $derived(
		node.kind === "Folder" && expanded
			? { running: false, unseen: null }
			: subtreeActivity($responsesByRequest, node.path),
	);

	// Same reason as in CollectionTree: a drop elsewhere must not leave this
	// row's insertion marker behind.
	$effect(() => {
		if (!$dragging) dropZone = null;
	});

	async function openRequest(path: string) {
		if ($activeRequest?.dirty && $activeRequest.path !== path) {
			const proceed = await confirmAction(
				$t("confirm.openOther"),
				{ title: $t("common.unsavedChanges"), confirmLabel: $t("common.open") },
			);
			if (!proceed) return;
		}
		try {
			const request = await api.loadRequest(path);
			activeCollection.set(collection);
			activeRequest.set({ path, request, dirty: false });
			// The response panel now shows whatever came back while the user was
			// elsewhere, so the sidebar marker has done its job.
			markSeen(path);
		} catch (e) {
			reportError($t("request.openFailed"), e);
		}
	}

	async function addRequest(protocol: Protocol = "http") {
		const title = $t(protocol === "sse" ? "prompt.newSseRequest" : "prompt.newRequest");
		const name = await promptForText(title, $t("prompt.requestName"), title);
		if (!name) return;
		await api.createRequest(node.path, name, "GET" as HttpMethod, protocol);
		setExpanded(node.path, true);
		requestTreeRefresh();
	}

	async function addFolder() {
		const name = await promptForText($t("prompt.newFolder"), $t("prompt.folderName"), $t("prompt.newFolder"));
		if (!name) return;
		await api.createFolder(node.path, name);
		setExpanded(node.path, true);
		requestTreeRefresh();
	}

	async function renameFolder() {
		const name = await promptForText($t("prompt.renameFolder"), $t("prompt.folderName"), node.name);
		if (!name || name === node.name) return;
		const newPath = await api.renameFolder(node.path, name);
		rebaseActiveRequest(node.path, newPath);
		rekeyResponses(node.path, newPath);
		rebaseExpanded(node.path, newPath);
		requestTreeRefresh();
	}

	async function renameRequest() {
		const name = await promptForText($t("prompt.renameRequest"), $t("prompt.requestName"), node.name);
		if (!name || name === node.name) return;
		const renamed = await api.renameRequest(node.path, name);
		rebaseActiveRequest(node.path, renamed.path);
		rekeyResponses(node.path, renamed.path);
		if ($activeRequest?.path === renamed.path) {
			activeRequest.set({ path: renamed.path, request: renamed, dirty: false });
		}
		requestTreeRefresh();
	}

	/// Clones the request and opens the copy - that is what the next click
	/// would be anyway. Opening goes through `openRequest`, so an unsaved
	/// request on screen still gets its confirmation.
	async function cloneRequest() {
		const name = await promptForText(
			$t("prompt.cloneRequest"),
			$t("prompt.copyName"),
			$t("prompt.copySuffix", { name: node.name }),
		);
		if (!name) return;
		try {
			const clone = await api.cloneRequest(node.path, name);
			requestTreeRefresh();
			await openRequest(clone.path);
		} catch (e) {
			reportError($t("error.cloneRequest"), e);
		}
	}

	async function removeFolder() {
		const confirmed = await confirmAction($t("confirm.deleteFolder", { name: node.name }), {
			title: $t("confirm.deleteFolderTitle"),
			confirmLabel: $t("common.delete"),
			danger: true,
		});
		if (!confirmed) return;
		await api.deleteFolder(node.path);
		forgetExpandedUnder(node.path);
		requestTreeRefresh();
	}

	async function removeRequest() {
		const confirmed = await confirmAction($t("confirm.deleteRequest", { name: node.name }), {
			title: $t("confirm.deleteRequestTitle"),
			confirmLabel: $t("common.delete"),
			danger: true,
		});
		if (!confirmed) return;
		await api.deleteRequest(node.path);
		forgetResponses(node.path);
		if ($activeRequest?.path === node.path) activeRequest.set(null);
		requestTreeRefresh();
	}

	/// Keeps everything that points at the moved entry in step with its new
	/// location: the open request, its cached responses, and - when it lands
	/// in a different collection - which collection is considered active.
	function afterMove(from: string, to: string) {
		const wasActive = $activeRequest?.path === from || $activeRequest?.path.startsWith(from + "\\");
		rebaseActiveRequest(from, to);
		rekeyResponses(from, to);
		rebaseExpanded(from, to);
		if (wasActive) activeCollection.set(collection);
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
		// A collection has no place inside the tree - it only moves among the
		// other collections - so this row never offers itself as a target for
		// one.
		if (!payload || payload.kind === "Collection" || payload.path === node.path) return;
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
				afterMove(payload.path, moved);
				setExpanded(node.path, true);
			} else {
				let sourcePath = payload.path;
				if (payload.parentPath !== parentPath) {
					sourcePath = await api.moveNode(payload.path, parentPath);
					afterMove(payload.path, sourcePath);
				}
				const order = siblings.map((s) => s.path).filter((p) => p !== payload.path && p !== sourcePath);
				const anchor = order.indexOf(node.path);
				order.splice(zone === "before" ? anchor : anchor + 1, 0, sourcePath);
				await api.reorderChildren(order);
			}
		} catch (err) {
			reportError($t("error.move"), err);
		} finally {
			// Always resync: after a partially applied move the sidebar would
			// otherwise keep showing the entry in its old place.
			requestTreeRefresh();
		}
	}

	// Grouped the same way in every menu in the sidebar: what this entry can
	// contain, then what can be done to the entry itself, then what destroys
	// it - so the delete item is never the neighbour of something harmless.
	let folderMenu: Menu = $derived([
		[
			{ label: $t("menu.addRequest"), icon: "request-add", action: () => addRequest() },
			{ label: $t("menu.addSseRequest"), icon: "stream", action: () => addRequest("sse") },
			{ label: $t("menu.addFolder"), icon: "folder-add", action: addFolder },
		],
		[{ label: $t("menu.renameFolder"), icon: "rename", action: renameFolder }],
		[{ label: $t("menu.deleteFolder"), icon: "delete", action: removeFolder, danger: true }],
	]);

	let requestMenu: Menu = $derived([
		[{ label: $t("menu.cloneRequest"), icon: "clone", action: cloneRequest }],
		[{ label: $t("menu.renameRequest"), icon: "rename", action: renameRequest }],
		[{ label: $t("menu.deleteRequest"), icon: "delete", action: removeRequest, danger: true }],
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
			oncontextmenu={(e) => openContextMenu(e, folderMenu)}
			draggable="true"
			ondragstart={onDragStart}
			ondragend={onDragEnd}
			ondragover={onDragOver}
			ondragleave={() => (dropZone = null)}
			ondrop={onDrop}
		>
			<button class="folder-label" onclick={() => toggleExpanded(node.path, FOLDER_DEFAULT_EXPANDED)}>
				<span class="chevron" class:collapsed={!expanded}>▾</span>
				<span class="node-name">{node.name}</span>
				<ActivityIndicator {activity} group />
			</button>
			<NodeMenu menu={folderMenu} label={$t("sidebar.folderActions")} />
		</div>
		{#if expanded}
			<div class="children">
				{#each node.children as child (child.path)}
					<TreeNode node={child} {collection} parentPath={node.path} siblings={node.children} />
				{/each}
				{#if node.children.length === 0}
					<p class="empty">{$t("common.empty")}</p>
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
		oncontextmenu={(e) => openContextMenu(e, requestMenu)}
		draggable="true"
		ondragstart={onDragStart}
		ondragend={onDragEnd}
		ondragover={onDragOver}
		ondragleave={() => (dropZone = null)}
		ondrop={onDrop}
	>
		<button class="request-label" onclick={() => openRequest(node.path)}>
			<span class="method" style="color: {methodColor(badge)}">{badge}</span>
			<span class="node-name">{node.name}</span>
			<ActivityIndicator {activity} />
		</button>
		<NodeMenu menu={requestMenu} label={$t("sidebar.requestActions")} />
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
		background: var(--hover);
	}
	.node-row.active {
		background: var(--selected);
	}
	.node-row.dragged {
		opacity: 0.4;
	}
	.node-row.drop-before {
		border-top-color: var(--accent-text);
	}
	.node-row.drop-after {
		border-bottom-color: var(--accent-text);
	}
	.node-row.drop-inside {
		background: color-mix(in srgb, var(--accent) 25%, transparent);
		outline: 1px dashed var(--accent-text);
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
		font-size: var(--fs-md);
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
		color: var(--text-muted);
		font-size: var(--fs-sm);
	}
	.method {
		font-size: var(--fs-xs);
		font-family: var(--font-mono);
		font-weight: 600;
		min-width: 2.8em;
		flex-shrink: 0;
	}
</style>
