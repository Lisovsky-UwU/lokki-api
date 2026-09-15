<script lang="ts">
	import { untrack } from "svelte";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { workspacePath, collections, workspace } from "../../stores/workspace";
	import type { CollectionSummary, CollectionTreeNode, HttpMethod } from "../../bindings/types";
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

	function collectionExpanded(path: string): boolean {
		return isExpanded($expandedPaths, path, COLLECTION_DEFAULT_EXPANDED);
	}

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
			collections.update((list) =>
				list.map((c) => (c.path === collection.path ? renamed : c)).sort((a, b) => a.name.localeCompare(b.name)),
			);
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

	async function createCollection() {
		const path = $workspacePath;
		const name = await promptForText($t("prompt.newCollection"), $t("prompt.collectionName"), $t("prompt.newCollection"));
		if (!path || !name) return;
		const summary = await api.createCollection(path, name);
		collections.update((list) => [...list, summary].sort((a, b) => a.name.localeCompare(b.name)));
		setExpanded(summary.path, true);
	}

	async function addRequest(collection: CollectionSummary) {
		const name = await promptForText($t("prompt.newRequest"), $t("prompt.requestName"), $t("prompt.newRequest"));
		if (!name) return;
		await api.createRequest(collection.path, name, "GET" as HttpMethod);
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
	<div class="sidebar-header">
		<span class="heading">{$t("sidebar.workspace")}</span>
	</div>
	{#if $workspace === null}
		<div class="empty">{$t("sidebar.noWorkspace")}</div>
	{:else}
		<div class="workspace-name-outer">
			<button class="icon-btn" title={$t("menu.switchWorkspace")} aria-label={$t("menu.switchWorkspace")} onclick={closeWorkspace}>
				<Icon name="switch" size="1.1em" />
			</button>
			<NodeMenu menu={workspaceMenu} label={$t("sidebar.workspaceMenu")} align="left">
				{#snippet trigger()}
					<span class="workspace-name">{$workspace?.name}</span>
					<span class="menu-hint">▾</span>
				{/snippet}
			</NodeMenu>
		</div>
	{/if}
	<div class="sidebar-header sidebar-header-collections">
		<NodeMenu menu={collectionsMenu} label={$t("sidebar.collectionsMenu")} align="left">
			{#snippet trigger()}
				<span class="heading">{$t("sidebar.collections")}</span>
				<span class="menu-hint">▾</span>
			{/snippet}
		</NodeMenu>
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
					<span class="chevron" class:collapsed={!collectionExpanded(collection.path)}>▾</span>
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
		padding: 0.5rem;
		font-size: 0.9em;
	}
	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.3em 0.4em;
	}
	/* The small caps sit on the label, not on the row. The collections
	   heading is a menu button and the menu is rendered inside that row, so
	   anything typographic here would land on every item - and `opacity`
	   worst of all, since it makes the whole subtree translucent and no
	   child can undo it. */
	.heading {
		font-weight: 600;
		text-transform: uppercase;
		font-size: 0.75em;
		letter-spacing: 0.04em;
		opacity: 0.7;
	}
	.sidebar-header-collections {
		margin-bottom: 0.8em;
		/* The heading is the menu button now, so it carries the padding
		   itself - on the container the hover highlight would sit inset from
		   the row. */
		padding: 0;
	}
	.sidebar-header-collections :global(.node-menu) {
		flex: 1;
		min-width: 0;
	}
	.sidebar-header-collections :global(.trigger.custom) {
		padding: 0.3em 0.4em;
		border-radius: 4px;
	}
	.sidebar-header-collections :global(.trigger.custom:hover) {
		background: rgba(127, 127, 127, 0.15);
	}
	/* The row is at normal size now, so the marker follows the small-caps
	   label beside it rather than the row it sits in. */
	.sidebar-header-collections .menu-hint {
		font-size: 0.75em;
	}
	.icon-btn {
		display: flex;
		align-items: center;
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1em;
		line-height: 1;
		padding: 0.2em 0.35em;
		border-radius: 4px;
		color: inherit;
	}
	.icon-btn:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	/* A collection is drawn as a container, a folder as a plain row inside
	   one. Weight alone stopped carrying that once a workspace had enough
	   collections for the two to interleave on screen.

	   Outlined rather than filled: a fill would have to be lighter than the
	   sidebar in the light theme and darker in the dark one to read as
	   "raised", which is two more palette entries to keep in step. A hairline
	   says "container" the same way in both. */
	.collection {
		border: 1px solid rgba(127, 127, 127, 0.25);
		border-radius: 6px;
		padding: 0.15em;
		margin-bottom: 0.4em;
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
	/* The collection whose environment is in effect - it follows the open
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
		gap: 0.2em;
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
	/* One marker for "this opens a menu": the workspace name and the
	   collections heading carry the same one. */
	.menu-hint {
		opacity: 0.5;
		font-size: 0.85em;
	}
</style>
