<script lang="ts">
	import { untrack } from "svelte";
	import { api } from "../../api/client";
	import { workspacePath, collections, workspace } from "../../stores/workspace";
	import type { CollectionSummary, CollectionTreeNode, HttpMethod } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";
	import ActivityIndicator from "../common/ActivityIndicator.svelte";
	import { activeCollection, treeRefreshToken, requestTreeRefresh } from "../../stores/collectionTree";
	import { dragging } from "../../stores/dragState";
	import { activeRequest, isUnder, rebaseActiveRequest, rebasePath } from "../../stores/activeRequest";
	import { forgetResponsesUnder, rekeyResponses, responsesByRequest, subtreeActivity } from "../../stores/response";
	import { confirmAction, promptForText } from "../../ui/dialogs";
	import { openEnvironmentsDialog } from "../../ui/environmentsDialog";
	import { openContextMenu } from "../../ui/contextMenu";
	import { reportError } from "../../ui/notices";

	let trees = $state<Record<string, CollectionTreeNode | null>>({});
	let expandedCollections = $state<Record<string, boolean>>({});
	let dropTarget = $state<string | null>(null);

	// A drop handled by a child node stops propagation, so this container's
	// own drop/dragleave never fires and its highlight would stay on. Clear
	// it whenever the drag itself is over, wherever it ended.
	$effect(() => {
		if (!$dragging) dropTarget = null;
	});

	// Plain Map (not reactive): only used to tell stale responses apart.
	const refreshSeq = new Map<string, number>();

	async function refreshCollection(collection: CollectionSummary) {
		const seq = (refreshSeq.get(collection.path) ?? 0) + 1;
		refreshSeq.set(collection.path, seq);
		try {
			const tree = await api.loadCollectionTree(collection.path);
			// A slower earlier request must not overwrite fresher data — that
			// race is what made a just-created request blink in and out.
			if (refreshSeq.get(collection.path) !== seq) return;
			// `trees` is read here, after the await, deliberately: doing it
			// before (as in `{ ...trees, [key]: await … }`) makes the spread
			// run inside the tracked scope of the effect below, so the effect
			// would depend on the state it writes and loop forever.
			trees = { ...trees, [collection.path]: tree };
		} catch (e) {
			reportError("Не удалось загрузить коллекцию", e);
		}
	}

	// Expanding is not selecting: the active collection follows the open
	// request, since it decides which collection environment resolves that
	// request's variables. Browsing another collection's tree must not pull
	// the environment out from under the request on screen.
	function toggleCollection(collection: CollectionSummary) {
		expandedCollections = { ...expandedCollections, [collection.path]: !expandedCollections[collection.path] };
		// Loading is left to the effect below, which already reacts to a
		// collection becoming expanded.
	}

	/// Renaming a collection renames its directory, so every path below it
	/// moves with it — including the ones this component keys its own caches
	/// by.
	async function renameCollection(collection: CollectionSummary) {
		const name = await promptForText("Переименовать коллекцию", "Название коллекции", collection.name);
		if (!name || name === collection.name) return;
		try {
			const renamed = await api.renameCollection(collection.path, name);
			rebaseActiveRequest(collection.path, renamed.path);
			rekeyResponses(collection.path, renamed.path);
			if ($activeCollection?.path === collection.path) activeCollection.set(renamed);
			collections.update((list) =>
				list.map((c) => (c.path === collection.path ? renamed : c)).sort((a, b) => a.name.localeCompare(b.name)),
			);
			expandedCollections = rekeyByPath(expandedCollections, collection.path, renamed.path);
			trees = rekeyByPath(trees, collection.path, renamed.path);
			requestTreeRefresh();
		} catch (e) {
			reportError("Не удалось переименовать коллекцию", e);
		}
	}

	/// Deleting a collection takes its whole tree with it, so everything the
	/// app holds by path inside it has to go too — the open request, cached
	/// responses, and this component's own caches.
	async function removeCollection(collection: CollectionSummary) {
		const confirmed = await confirmAction(
			`Удалить коллекцию «${collection.name}» со всеми запросами, папками и окружениями? Папка будет удалена с диска.`,
			{ title: "Удаление коллекции", confirmLabel: "Удалить", danger: true },
		);
		if (!confirmed) return;
		try {
			await api.deleteCollection(collection.path);
			collections.update((list) => list.filter((c) => c.path !== collection.path));
			forgetResponsesUnder(collection.path);
			if ($activeCollection?.path === collection.path) activeCollection.set(null);
			if ($activeRequest && isUnder($activeRequest.path, collection.path)) activeRequest.set(null);
			expandedCollections = dropByPath(expandedCollections, collection.path);
			trees = dropByPath(trees, collection.path);
		} catch (e) {
			reportError("Не удалось удалить коллекцию", e);
		}
	}

	function dropByPath<T>(map: Record<string, T>, prefix: string): Record<string, T> {
		return Object.fromEntries(Object.entries(map).filter(([key]) => !isUnder(key, prefix)));
	}

	function rekeyByPath<T>(map: Record<string, T>, from: string, to: string): Record<string, T> {
		return Object.fromEntries(Object.entries(map).map(([key, value]) => [rebasePath(key, from, to), value]));
	}

	// Any request/folder create/save/delete/move anywhere in the app bumps
	// this token — re-fetch every currently-expanded collection's tree so the
	// sidebar never shows stale names, methods or ordering.
	$effect(() => {
		$treeRefreshToken;
		const pending = $collections.filter((c) => expandedCollections[c.path]);
		// Fetching is kept out of the tracked scope so this effect never
		// subscribes to what the fetch writes.
		untrack(() => {
			for (const collection of pending) refreshCollection(collection);
		});
	});

	async function createCollection() {
		const path = $workspacePath;
		const name = await promptForText("Новая коллекция", "Название коллекции", "Новая коллекция");
		if (!path || !name) return;
		const summary = await api.createCollection(path, name);
		collections.update((list) => [...list, summary].sort((a, b) => a.name.localeCompare(b.name)));
		expandedCollections = { ...expandedCollections, [summary.path]: true };
	}

	async function addRequest(collection: CollectionSummary) {
		const name = await promptForText("Новый запрос", "Название запроса", "Новый запрос");
		if (!name) return;
		await api.createRequest(collection.path, name, "GET" as HttpMethod);
		expandedCollections = { ...expandedCollections, [collection.path]: true };
		requestTreeRefresh();
	}

	async function addFolder(collection: CollectionSummary) {
		const name = await promptForText("Новая папка", "Название папки", "Новая папка");
		if (!name) return;
		await api.createFolder(collection.path, name);
		expandedCollections = { ...expandedCollections, [collection.path]: true };
		requestTreeRefresh();
	}

	function collectionMenu(collection: CollectionSummary) {
		return [
			{ label: "Добавить запрос", action: () => addRequest(collection) },
			{ label: "Добавить папку", action: () => addFolder(collection) },
			// Reachable without opening a request first — the switcher in the
			// top bar only ever shows the active collection's environments.
			{
				label: "Окружения коллекции",
				action: () => openEnvironmentsDialog({ rootPath: collection.path, scope: "collection", title: collection.name }),
			},
			{ label: "Переименовать коллекцию", action: () => renameCollection(collection) },
			{ label: "Удалить коллекцию", action: () => removeCollection(collection), danger: true },
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
				const wasActive = $activeRequest?.path === payload.path || $activeRequest?.path.startsWith(payload.path + "\\");
				rebaseActiveRequest(payload.path, sourcePath);
				rekeyResponses(payload.path, sourcePath);
				if (wasActive) activeCollection.set(collection);
			}
			const order = childrenOf(collection.path)
				.map((c) => c.path)
				.filter((p) => p !== payload.path && p !== sourcePath);
			order.push(sourcePath);
			await api.reorderChildren(order);
		} catch (e) {
			reportError("Не удалось переместить", e);
		} finally {
			requestTreeRefresh();
		}
	}

	/// The workspace name is metadata in `.lokki/workspace.toml`, so renaming
	/// it touches neither the folder nor any cached path.
	async function renameWorkspace() {
		const path = $workspacePath;
		const current = $workspace;
		if (!path || !current) return;
		const name = await promptForText("Переименовать пространство", "Название пространства", current.name);
		if (!name || name === current.name) return;
		try {
			workspace.set(await api.renameWorkspace(path, name));
		} catch (e) {
			reportError("Не удалось переименовать пространство", e);
		}
	}

	// Only workspace-scoped actions belong here: app settings sit in the top
	// bar, since a menu hanging off the workspace name reads as "settings of
	// this workspace".
	let workspaceMenu = $derived([
		{ label: "Переименовать пространство", action: renameWorkspace },
		{
			label: "Окружения пространства",
			action: () =>
				$workspacePath &&
				openEnvironmentsDialog({ rootPath: $workspacePath, scope: "global", title: $workspace?.name ?? "" }),
		},
		{ label: "Сменить пространство", action: closeWorkspace },
	]);

	function closeWorkspace() {
		workspace.set(null);
		workspacePath.set(null);
		collections.set([]);
		activeCollection.set(null);
		activeRequest.set(null);
		responsesByRequest.set({});
	}
</script>

<div class="sidebar">
	<div class="sidebar-header">
		<span>Пространство</span>
	</div>
	{#if $workspace === null}
		<div class="empty">Нет открытого пространства. Создайте или откройте существующий</div>
	{:else}
		<div class="workspace-name-outer">
			<NodeMenu items={workspaceMenu} label="Меню пространства" align="left">
				{#snippet trigger()}
					<span class="workspace-name">{$workspace?.name}</span>
					<span class="switch-hint">▾</span>
				{/snippet}
			</NodeMenu>
		</div>
	{/if}
	<div class="sidebar-header">
		<span>Коллекции</span>
		<button class="icon-btn" title="Новая коллекция" onclick={createCollection}>+</button>
	</div>

	{#each $collections as collection (collection.path)}
		<div class="collection">
			<div
				class="collection-header"
				class:active={$activeCollection?.path === collection.path}
				role="presentation"
				oncontextmenu={(e) => openContextMenu(e, collectionMenu(collection))}
			>
				<button class="collection-label" onclick={() => toggleCollection(collection)}>
					<span class="chevron" class:collapsed={!expandedCollections[collection.path]}>▾</span>
					<span class="collection-name">{collection.name}</span>
					{#if !expandedCollections[collection.path]}
						<ActivityIndicator activity={subtreeActivity($responsesByRequest, collection.path)} group />
					{/if}
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
						<p class="empty">Пусто - создайте запрос или папку</p>
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
	.collection-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.collection-label:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	/* The collection whose environment is in effect — it follows the open
	   request, so this also says where that request lives. */
	.collection-header.active {
		background: rgba(57, 108, 216, 0.14);
		border-radius: 4px;
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
	.workspace-name-outer {
		display: flex;
		align-items: center;
		padding: 0.5em 0.2em;
	}
	/* The menu component owns the button; these rules dress its trigger. */
	.workspace-name-outer :global(.node-menu) {
		flex: 1;
		min-width: 0;
	}
	.workspace-name-outer :global(.trigger.custom) {
		padding: 0.6em 0.8em;
		border-radius: 6px;
	}
	.workspace-name-outer :global(.trigger.custom:hover) {
		background: rgba(127, 127, 127, 0.15);
	}
	.workspace-name {
		font-weight: 600;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		text-align: left;
	}
	.switch-hint {
		opacity: 0.5;
		font-size: 0.85em;
	}
</style>
