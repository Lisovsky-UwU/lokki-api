<script lang="ts">
	import { untrack } from "svelte";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { workspacePath, collections, workspace } from "../../stores/workspace";
	import type { CollectionSummary, CollectionTreeNode, HttpMethod, Protocol } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";
	import Icon from "../common/Icon.svelte";
	import ActivityIndicator from "../common/ActivityIndicator.svelte";
	import { activeCollection, treeRefreshToken, requestTreeRefresh } from "../../stores/collectionTree";
	import { dragging } from "../../stores/dragState";
	import { activeRequest, isUnder, rebaseActiveRequest, rebasePath } from "../../stores/activeRequest";
	import {
		COLLECTION_DEFAULT_EXPANDED,
		collapseAll,
		collapseAllUnder,
		expandAll,
		expandedPaths,
		forgetExpandedUnder,
		isExpanded,
		rebaseExpanded,
		setExpanded,
		toggleExpanded,
	} from "../../stores/expansion";
	import { forgetResponsesUnder, rekeyResponses, responsesByRequest, subtreeActivity } from "../../stores/response";
	import { confirmAction, promptForText } from "../../ui/dialogs";
	import { openEnvironmentsDialog } from "../../ui/environmentsDialog";
	import { openContextMenu, type Menu } from "../../ui/contextMenu";
	import { openImportDialog } from "../../ui/importDialog";
	import { reportError } from "../../ui/notices";

	let trees = $state<Record<string, CollectionTreeNode | null>>({});
	let dropTarget = $state<string | null>(null);
	/// Where a dragged collection would land. One at a time, so it is held
	/// here rather than per row: unlike the tree, every collection is
	/// rendered by this one component.
	let collectionDrop = $state<{ path: string; zone: "before" | "after" } | null>(null);

	function collectionExpanded(path: string): boolean {
		return isExpanded($expandedPaths, path, COLLECTION_DEFAULT_EXPANDED);
	}

	// A drop handled by a child node stops propagation, so this container's
	// own drop/dragleave never fires and its highlight would stay on. Clear
	// it whenever the drag itself is over, wherever it ended.
	$effect(() => {
		if (!$dragging) {
			dropTarget = null;
			collectionDrop = null;
		}
	});

	// Plain Map (not reactive): only used to tell stale responses apart.
	const refreshSeq = new Map<string, number>();

	async function refreshCollection(collection: CollectionSummary) {
		const seq = (refreshSeq.get(collection.path) ?? 0) + 1;
		refreshSeq.set(collection.path, seq);
		try {
			const tree = await api.loadCollectionTree(collection.path);
			// A slower earlier request must not overwrite fresher data - that
			// race is what made a just-created request blink in and out.
			if (refreshSeq.get(collection.path) !== seq) return;
			// `trees` is read here, after the await, deliberately: doing it
			// before (as in `{ ...trees, [key]: await … }`) makes the spread
			// run inside the tracked scope of the effect below, so the effect
			// would depend on the state it writes and loop forever.
			trees = { ...trees, [collection.path]: tree };
		} catch (e) {
			reportError($t("error.loadCollection"), e);
		}
	}

	// Expanding is not selecting: the active collection follows the open
	// request, since it decides which collection environment resolves that
	// request's variables. Browsing another collection's tree must not pull
	// the environment out from under the request on screen.
	function toggleCollection(collection: CollectionSummary) {
		toggleExpanded(collection.path, COLLECTION_DEFAULT_EXPANDED);
		// Loading is left to the effect below, which already reacts to a
		// collection becoming expanded.
	}

	/// Renaming a collection renames its directory, so every path below it
	/// moves with it - including the ones this component keys its own caches
	/// by.
	async function renameCollection(collection: CollectionSummary) {
		const name = await promptForText($t("prompt.renameCollection"), $t("prompt.collectionName"), collection.name);
		if (!name || name === collection.name) return;
		try {
			const renamed = await api.renameCollection(collection.path, name);
			rebaseActiveRequest(collection.path, renamed.path);
			rekeyResponses(collection.path, renamed.path);
			if ($activeCollection?.path === collection.path) activeCollection.set(renamed);
			await reloadCollections();
			rebaseExpanded(collection.path, renamed.path);
			trees = rekeyByPath(trees, collection.path, renamed.path);
			requestTreeRefresh();
		} catch (e) {
			reportError($t("error.renameCollection"), e);
		}
	}

	/// Deleting a collection takes its whole tree with it, so everything the
	/// app holds by path inside it has to go too - the open request, cached
	/// responses, and this component's own caches.
	async function removeCollection(collection: CollectionSummary) {
		const confirmed = await confirmAction(
			$t("confirm.deleteCollection", { name: collection.name }),
			{ title: $t("confirm.deleteCollectionTitle"), confirmLabel: $t("common.delete"), danger: true },
		);
		if (!confirmed) return;
		try {
			await api.deleteCollection(collection.path);
			collections.update((list) => list.filter((c) => c.path !== collection.path));
			forgetResponsesUnder(collection.path);
			if ($activeCollection?.path === collection.path) activeCollection.set(null);
			if ($activeRequest && isUnder($activeRequest.path, collection.path)) activeRequest.set(null);
			forgetExpandedUnder(collection.path);
			trees = dropByPath(trees, collection.path);
		} catch (e) {
			reportError($t("error.deleteCollection"), e);
		}
	}

	function dropByPath<T>(map: Record<string, T>, prefix: string): Record<string, T> {
		return Object.fromEntries(Object.entries(map).filter(([key]) => !isUnder(key, prefix)));
	}

	function rekeyByPath<T>(map: Record<string, T>, from: string, to: string): Record<string, T> {
		return Object.fromEntries(Object.entries(map).map(([key, value]) => [rebasePath(key, from, to), value]));
	}

	// Any request/folder create/save/delete/move anywhere in the app bumps
	// this token - re-fetch every currently-expanded collection's tree so the
	// sidebar never shows stale names, methods or ordering.
	$effect(() => {
		$treeRefreshToken;
		const pending = $collections.filter((c) => collectionExpanded(c.path));
		// Fetching is kept out of the tracked scope so this effect never
		// subscribes to what the fetch writes.
		untrack(() => {
			for (const collection of pending) refreshCollection(collection);
		});
	});

	/// Re-reads the list instead of re-sorting it here: the order of
	/// collections lives in their files now, so after a create, a rename or a
	/// drag the core is the one that knows where everything goes.
	async function reloadCollections() {
		const path = $workspacePath;
		if (!path) return;
		try {
			collections.set(await api.listCollections(path));
		} catch (e) {
			reportError($t("error.loadCollection"), e);
		}
	}

	async function createCollection() {
		const path = $workspacePath;
		const name = await promptForText($t("prompt.newCollection"), $t("prompt.collectionName"), $t("prompt.newCollection"));
		if (!path || !name) return;
		const summary = await api.createCollection(path, name);
		await reloadCollections();
		setExpanded(summary.path, true);
	}

	function onCollectionDragStart(e: DragEvent, collection: CollectionSummary) {
		const path = $workspacePath;
		if (!path) return;
		dragging.set({ path: collection.path, parentPath: path, kind: "Collection" });
		e.dataTransfer?.setData("text/plain", collection.path);
		if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
	}

	/// Collections sit side by side with nothing to drop into, so the whole
	/// row is an insertion point: above it or below it.
	function onCollectionDragOver(e: DragEvent, collection: CollectionSummary) {
		const payload = $dragging;
		// A request or a folder dragged up here is heading into a collection,
		// which the tree area below handles - the header is not a target for
		// it.
		if (payload?.kind !== "Collection" || payload.path === collection.path) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const zone = (e.clientY - rect.top) / rect.height < 0.5 ? "before" : "after";
		collectionDrop = { path: collection.path, zone };
	}

	async function onCollectionDrop(collection: CollectionSummary) {
		const payload = $dragging;
		const drop = collectionDrop;
		collectionDrop = null;
		dragging.set(null);
		if (payload?.kind !== "Collection" || !drop || payload.path === collection.path) return;

		const order = $collections.map((c) => c.path).filter((p) => p !== payload.path);
		const anchor = order.indexOf(collection.path);
		order.splice(drop.zone === "before" ? anchor : anchor + 1, 0, payload.path);
		try {
			await api.reorderCollections(order);
		} catch (e) {
			reportError($t("error.move"), e);
		} finally {
			// Always resync: after a partially applied reorder the sidebar
			// would otherwise show an order nothing on disk agrees with.
			await reloadCollections();
		}
	}

	async function addRequest(collection: CollectionSummary, protocol: Protocol = "http") {
		const title = $t(protocol === "sse" ? "prompt.newSseRequest" : "prompt.newRequest");
		const name = await promptForText(title, $t("prompt.requestName"), title);
		if (!name) return;
		await api.createRequest(collection.path, name, "GET" as HttpMethod, protocol);
		setExpanded(collection.path, true);
		requestTreeRefresh();
	}

	async function addFolder(collection: CollectionSummary) {
		const name = await promptForText($t("prompt.newFolder"), $t("prompt.folderName"), $t("prompt.newFolder"));
		if (!name) return;
		await api.createFolder(collection.path, name);
		setExpanded(collection.path, true);
		requestTreeRefresh();
	}

	/// Requests are left out: they have nothing to expand.
	function folderPaths(node: CollectionTreeNode, into: string[]) {
		if (node.kind !== "Folder") return;
		for (const child of node.children) {
			if (child.kind !== "Folder") continue;
			into.push(child.path);
			folderPaths(child, into);
		}
	}

	/// Every path that can be opened in `list`: the collections themselves and
	/// the folders inside them. The tree has to be on hand to know what those
	/// folders are, so a collapsed collection - which may never have loaded
	/// one - fetches it first.
	async function expandablePaths(list: CollectionSummary[]): Promise<string[]> {
		await Promise.all(list.filter((c) => !trees[c.path]).map((c) => refreshCollection(c)));
		const paths: string[] = [];
		for (const collection of list) {
			paths.push(collection.path);
			const tree = trees[collection.path];
			if (tree) folderPaths(tree, paths);
		}
		return paths;
	}

	async function expandAllIn(collection: CollectionSummary) {
		expandAll(await expandablePaths([collection]));
	}

	/// The same two actions across the whole workspace. Collapsing names the
	/// collections rather than the workspace folder: the map is keyed by
	/// absolute path and outlives a workspace switch, so the workspace it is
	/// applied to has to be spelled out.
	async function expandEverything() {
		expandAll(await expandablePaths($collections));
	}

	function collapseEverything() {
		collapseAll($collections.map((c) => c.path));
	}

	/// The "Collections" heading is itself the menu: filling the list, and
	/// folding all of it at once. Two icon buttons used to sit there instead,
	/// which had no room left for a third and a fourth action.
	let collectionsMenu: Menu = $derived([
		[
			{ label: $t("sidebar.newCollection"), icon: "plus", action: createCollection },
			{ label: $t("sidebar.importCollection"), icon: "import", action: openImportDialog },
		],
		[
			{ label: $t("menu.expandAll"), icon: "expand-all", action: expandEverything },
			{ label: $t("menu.collapseAll"), icon: "collapse-all", action: collapseEverything },
		],
	]);

	/// Four groups, in the order a collection is usually worked with: fill
	/// it, look through it, change what it is, then get rid of it.
	function collectionMenu(collection: CollectionSummary): Menu {
		return [
			[
				{ label: $t("menu.addRequest"), icon: "request-add", action: () => addRequest(collection) },
				{ label: $t("menu.addSseRequest"), icon: "stream", action: () => addRequest(collection, "sse") },
				{ label: $t("menu.addFolder"), icon: "folder-add", action: () => addFolder(collection) },
			],
			[
				{ label: $t("menu.expandAll"), icon: "expand-all", action: () => expandAllIn(collection) },
				{ label: $t("menu.collapseAll"), icon: "collapse-all", action: () => collapseAllUnder(collection.path) },
			],
			[
				// Reachable without opening a request first - the switcher in
				// the top bar only ever shows the active collection's
				// environments.
				{
					label: $t("menu.collectionEnvironments"),
					icon: "environments",
					action: () =>
						openEnvironmentsDialog({ rootPath: collection.path, scope: "collection", title: collection.name }),
				},
				{ label: $t("menu.renameCollection"), icon: "rename", action: () => renameCollection(collection) },
			],
			[{ label: $t("menu.deleteCollection"), icon: "delete", action: () => removeCollection(collection), danger: true }],
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
		// A collection cannot be filed inside another one; it only moves
		// among its peers, which the headers handle.
		if (!payload || payload.kind === "Collection") return;
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
			reportError($t("error.move"), e);
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
		const name = await promptForText($t("prompt.renameWorkspace"), $t("prompt.workspaceName"), current.name);
		if (!name || name === current.name) return;
		try {
			workspace.set(await api.renameWorkspace(path, name));
		} catch (e) {
			reportError($t("error.renameWorkspace"), e);
		}
	}

	// Only workspace-scoped actions belong here: app settings sit in the top
	// bar, since a menu hanging off the workspace name reads as "settings of
	// this workspace".
	let workspaceMenu: Menu = $derived([
		[
			{ label: $t("menu.renameWorkspace"), icon: "rename", action: renameWorkspace },
			{
				label: $t("menu.workspaceEnvironments"),
				icon: "environments",
				action: () =>
					$workspacePath &&
					openEnvironmentsDialog({ rootPath: $workspacePath, scope: "global", title: $workspace?.name ?? "" }),
			},
		],
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
	<!-- The workspace's name is the sidebar's title: everything below it
	     belongs to that workspace, so it needs no label of its own. -->
	{#if $workspace === null}
		<div class="empty">{$t("sidebar.noWorkspace")}</div>
	{:else}
		<div class="workspace-header">
			<NodeMenu menu={workspaceMenu} label={$t("sidebar.workspaceMenu")} align="left">
				{#snippet trigger()}
					<span class="workspace-name">{$workspace?.name}</span>
					<span class="menu-hint"><Icon name="chevron" size="14px" /></span>
				{/snippet}
			</NodeMenu>
			<button class="icon-btn" title={$t("menu.switchWorkspace")} aria-label={$t("menu.switchWorkspace")} onclick={closeWorkspace}>
				<Icon name="switch" size="16px" />
			</button>
		</div>
	{/if}
	<div class="section-header">
		<span class="heading">{$t("sidebar.collections")}</span>
		<NodeMenu menu={collectionsMenu} label={$t("sidebar.collectionsMenu")} />
	</div>

	{#each $collections as collection (collection.path)}
		<div
			class="collection"
			class:drop-before={collectionDrop?.path === collection.path && collectionDrop.zone === "before"}
			class:drop-after={collectionDrop?.path === collection.path && collectionDrop.zone === "after"}
			class:dragged={$dragging?.kind === "Collection" && $dragging.path === collection.path}
		>
			<div
				class="collection-header"
				class:active={$activeCollection?.path === collection.path}
				role="presentation"
				oncontextmenu={(e) => openContextMenu(e, collectionMenu(collection))}
				draggable="true"
				ondragstart={(e) => onCollectionDragStart(e, collection)}
				ondragend={() => {
					dragging.set(null);
					collectionDrop = null;
				}}
				ondragover={(e) => onCollectionDragOver(e, collection)}
				ondragleave={() => (collectionDrop = null)}
				ondrop={(e) => {
					e.preventDefault();
					e.stopPropagation();
					onCollectionDrop(collection);
				}}
			>
				<button class="collection-label" onclick={() => toggleCollection(collection)}>
					<span class="chevron" class:collapsed={!collectionExpanded(collection.path)}><Icon name="chevron" size="14px" /></span>
					<span class="collection-name">{collection.name}</span>
					{#if !collectionExpanded(collection.path)}
						<ActivityIndicator activity={subtreeActivity($responsesByRequest, collection.path)} group />
					{/if}
				</button>
				<NodeMenu menu={collectionMenu(collection)} label={$t("sidebar.collectionActions")} />
			</div>
			{#if collectionExpanded(collection.path)}
				{@const children = childrenOf(collection.path)}
				<div
					class="tree"
					class:drop-root={dropTarget === collection.path}
					role="presentation"
					ondragover={(e) => {
						// Same rule as the header, from the other side: a
						// collection dropped in here would be moved inside its
						// neighbour.
						if (!$dragging || $dragging.kind === "Collection") return;
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
						<p class="empty">{$t("sidebar.emptyCollection")}</p>
					{/if}
				</div>
			{/if}
		</div>
	{/each}

	{#if $collections.length === 0}
		<p class="empty">{$t("sidebar.noCollections")}</p>
	{/if}
</div>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow-y: auto;
		padding: 0 6px 8px;
		font-size: var(--fs-md);
	}
	.workspace-header {
		display: flex;
		align-items: center;
		gap: 4px;
		/* Edge to edge across the panel, so its rule lines up with the top
		   bar's on the other side of the splitter. */
		margin: 0 -6px;
		padding: 0 6px;
		height: 46px;
		flex-shrink: 0;
		border-bottom: 1px solid var(--line);
	}
	/* The menu component owns the button; these rules dress its trigger. */
	.workspace-header :global(.node-menu) {
		flex: 1;
		min-width: 0;
	}
	.workspace-header :global(.trigger.custom) {
		gap: 4px;
		padding: 5px 8px;
		border-radius: 6px;
	}
	.workspace-header :global(.trigger.custom:hover) {
		background: var(--hover);
	}
	.workspace-name {
		font-weight: 700;
		font-size: var(--fs-lg);
		white-space: nowrap;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		text-align: left;
	}
	/* One marker for "this opens a menu". */
	.menu-hint {
		display: inline-flex;
		color: var(--text-muted);
	}
	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		flex-shrink: 0;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0;
		border-radius: 6px;
		color: var(--text-muted);
	}
	.icon-btn:hover {
		color: var(--text);
		background: var(--hover);
	}
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 2px 4px 8px;
	}
	.heading {
		font-weight: 600;
		font-size: var(--fs-sm);
		color: var(--text-muted);
	}
	/* A collection is told apart from a folder by its header rather than by
	   a box around it: the header is heavier, and it stays pinned to the top
	   of the panel while its tree scrolls under it, so a long tree never
	   loses track of whose folders these are. Boxes stopped working once a
	   workspace had enough collections to stack a column of them. */
	.collection {
		margin-bottom: 2px;
	}
	.collection.dragged {
		opacity: 0.4;
	}
	.collection-header {
		position: sticky;
		top: 0;
		z-index: 1;
		display: flex;
		align-items: center;
		border-radius: 6px;
		/* Opaque, or the rows scrolling under it would show through. */
		background: var(--surface-sunken);
	}
	/* The insertion line sits on the header, where the eye already is. */
	.collection.drop-before .collection-header {
		box-shadow: inset 0 2px 0 var(--accent-text);
	}
	.collection.drop-after {
		box-shadow: 0 2px 0 var(--accent-text);
	}
	/* The collection whose environment is in effect - it follows the open
	   request, so this also says where that request lives. Layered over the
	   opaque fill rather than replacing it. */
	.collection-header.active {
		background-image: linear-gradient(var(--selected), var(--selected));
	}
	.collection-label {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: 1;
		min-width: 0;
		text-align: left;
		background: none;
		border: none;
		padding: 6px 6px;
		border-radius: 6px;
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
		background: var(--hover);
	}
	/* The row's menu shows up with the pointer or the keyboard, and stays
	   while it is open; otherwise a column of dots runs down the panel. */
	.collection-header:not(:hover):not(:focus-within) :global(.trigger[aria-expanded="false"]) {
		opacity: 0;
	}
	.chevron {
		display: inline-flex;
		color: var(--text-muted);
		transition: transform 0.15s;
	}
	.chevron.collapsed {
		transform: rotate(-90deg);
	}
	/* The guide line marks how deep a row sits, the same way at every
	   level (see `.children` in TreeNode). */
	.tree {
		margin-left: 13px;
		padding: 2px 0 6px 4px;
		border-left: 1px solid var(--line);
	}
	.tree.drop-root {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		outline: 1px dashed color-mix(in srgb, var(--accent) 60%, transparent);
	}
	.empty {
		color: var(--text-muted);
		padding: 4px 8px;
		margin: 0;
		font-size: var(--fs-sm);
	}
	@media (prefers-reduced-motion: reduce) {
		.chevron {
			transition: none;
		}
	}
</style>
