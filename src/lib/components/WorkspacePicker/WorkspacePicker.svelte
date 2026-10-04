<script lang="ts">
	import { base } from "$app/paths";
	import { api } from "../../api/client";
	import { workspace, workspacePath, collections } from "../../stores/workspace";
	import type { OpenWorkspaceResult } from "../../api/client";
	import type { RecentWorkspace } from "../../bindings/types";
	import { t } from "../../i18n";
	import { alertMessage, promptForText } from "../../ui/dialogs";
	import { startIncognito } from "../../stores/incognito";
	import { reportError } from "../../ui/notices";
	import { openContextMenu, type Menu } from "../../ui/contextMenu";
	import GhostIcon from "../common/GhostIcon.svelte";
	import NodeMenu from "../common/NodeMenu.svelte";

	/// Which action is in flight - "create", "open", or a recent workspace's
	/// path. Held as the identity rather than a boolean so only the control
	/// that was clicked reports progress while the rest merely lock.
	let busy = $state<string | null>(null);
	let recent = $state<RecentWorkspace[]>([]);

	// The list only holds folders that are still there, so an entry that has
	// been moved or deleted since disappears rather than failing on click.
	async function loadRecent() {
		try {
			recent = await api.listRecentWorkspaces();
		} catch (e) {
			reportError($t("picker.recentLoadFailed"), e);
		}
	}
	loadRecent();

	/// The core words these failures for the user ("folder X is not empty",
	/// "folder X holds no workspace"), so they are shown verbatim.
	///
	/// Deliberately not awaited: nothing here depends on the acknowledgement,
	/// and waiting for it would hold `busy` - and with it the greyed-out
	/// screen - up for as long as the dialog is.
	function failed(e: unknown): void {
		void alertMessage(e instanceof Error ? e.message : String(e));
	}

	function enter(path: string, result: OpenWorkspaceResult) {
		workspacePath.set(path);
		workspace.set(result.workspace);
		collections.set(result.collections);
	}

	/// Picks a folder and initializes it. The name is asked for separately
	/// because it is metadata, not the folder name - it can be changed later
	/// without moving anything on disk.
	///
	/// The folder is vetted before that prompt rather than only by
	/// `createWorkspace` after it: making the user name a workspace and only
	/// then telling them the folder was never eligible wastes the one step
	/// they actually had to think about.
	async function createWorkspace() {
		try {
			const folder = await api.pickWorkspaceFolder();
			if (!folder) return;
			await api.checkNewWorkspaceFolder(folder);
			const suggested = folder.split(/[\\/]+/).filter(Boolean).pop() ?? $t("picker.newWorkspace");
			const name = await promptForText($t("picker.newWorkspace"), $t("prompt.workspaceName"), suggested);
			if (!name) return;
			busy = "create";
			enter(folder, await api.createWorkspace(folder, name));
		} catch (e) {
			failed(e);
		} finally {
			busy = null;
		}
	}

	/// Opening only ever opens: a folder that isn't a workspace is reported
	/// as such instead of quietly becoming a new empty one.
	async function openWorkspace() {
		try {
			const folder = await api.pickWorkspaceFolder();
			if (!folder) return;
			busy = "open";
			enter(folder, await api.openWorkspace(folder));
		} catch (e) {
			failed(e);
		} finally {
			busy = null;
		}
	}

	/// The failure worth reporting here is "this is no longer a workspace".
	/// The list is reloaded after one, since the folder may have gone along
	/// with the workspace and the entry then has no business still showing.
	async function openRecent(entry: RecentWorkspace) {
		try {
			busy = entry.path;
			enter(entry.path, await api.openWorkspace(entry.path));
		} catch (e) {
			failed(e);
			await loadRecent();
		} finally {
			busy = null;
		}
	}

	async function forgetRecent(entry: RecentWorkspace) {
		try {
			recent = await api.forgetRecentWorkspace(entry.path);
		} catch (e) {
			reportError($t("picker.recentRemoveFailed"), e);
		}
	}

	async function revealRecent(entry: RecentWorkspace) {
		try {
			await api.revealWorkspace(entry.path);
		} catch (e) {
			reportError($t("picker.recentRevealFailed"), e);
		}
	}

	/// "Remove" is not marked `danger` and does not use the trash icon: it
	/// takes the entry off this list and leaves the folder alone, and the two
	/// must not look like the same act.
	function recentMenu(entry: RecentWorkspace): Menu {
		return [
			[{ label: $t("picker.recentReveal"), icon: "folder-open", action: () => revealRecent(entry) }],
			[{ label: $t("picker.recentRemove"), icon: "remove", action: () => forgetRecent(entry) }],
		];
	}
</script>

<div class="picker">
	<button
		class="ghost"
		title={$t("incognito.pickerTitle")}
		aria-label={$t("incognito.request")}
		onclick={startIncognito}
		disabled={busy !== null}
	>
		<GhostIcon size="18px" />
	</button>
	<div class="column">
		<!-- The app icon plus a typeset wordmark rather than the banner image:
		     the icon is a tile of its own and sits on either theme, where the
		     banner was a dark plate pasted onto the light one. -->
		<header class="brand">
			<img class="logo" src="{base}/logo-256.png" alt="" />
			<h1 class="wordmark">Lokki<span>API</span></h1>
		</header>
		<p class="hint">{$t("picker.hint")}</p>
		<div class="actions">
			<button class="primary" onclick={createWorkspace} disabled={busy !== null}>
				{busy === "create" ? $t("picker.creating") : $t("picker.create")}
			</button>
			<button onclick={openWorkspace} disabled={busy !== null}>
				{busy === "open" ? $t("picker.opening") : $t("picker.openExisting")}
			</button>
		</div>
		{#if recent.length > 0}
			<div class="recent">
				<div class="recent-head">{$t("picker.recent")}</div>
				{#each recent as entry (entry.path)}
					<div class="recent-row" role="presentation" oncontextmenu={(e) => openContextMenu(e, recentMenu(entry))}>
						<button class="recent-open" onclick={() => openRecent(entry)} disabled={busy !== null}>
							<span class="recent-name">{entry.name}</span>
							<!-- A path long enough to be clipped is still identifiable
							     from the tooltip; `dir="rtl"` would keep the tail visible
							     but reorders the separators around it. -->
							<span class="recent-path" title={entry.path}>{entry.path}</span>
						</button>
						{#if busy === entry.path}
							<span class="recent-busy">{$t("picker.opening")}</span>
						{:else}
							<NodeMenu menu={recentMenu(entry)} label={$t("picker.recentActions")} />
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	.picker {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 100vh;
		padding: 15vh 2rem 2rem;
		/* The recent list is the one part that can outgrow the window. */
		overflow-y: auto;
	}
	/* Left-aligned in a column of its own: the name, what a workspace is,
	   the two ways in, and where you were last - read top to bottom. */
	.column {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 14px;
		width: min(480px, 100%);
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.logo {
		/* The tile has transparent margins of its own; the negative margin
		   lines the visible edge up with the text below. */
		width: 76px;
		height: 76px;
		margin: -8px -4px -8px -10px;
	}
	.wordmark {
		margin: 0;
		font-size: 30px;
		font-weight: 800;
		letter-spacing: -0.02em;
	}
	/* The two halves of the name as the logo sets them. */
	.wordmark span {
		color: var(--accent-text);
	}
	.hint {
		margin: 0;
		color: var(--text-muted);
		line-height: 1.5;
	}
	.actions {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		margin-top: 4px;
	}
	button {
		padding: 0 16px;
		height: 36px;
		border-radius: 8px;
		border: 1px solid var(--line-strong);
		background: var(--surface-raised);
		color: inherit;
		cursor: pointer;
		font-size: var(--fs-lg);
	}
	.primary {
		border-color: var(--accent);
		background: var(--accent);
		color: var(--accent-ink);
		font-weight: 600;
	}
	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.ghost {
		position: absolute;
		top: 10px;
		right: 10px;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		padding: 0;
		border: none;
		background: none;
		border-radius: 6px;
		color: var(--text-muted);
	}
	.ghost:hover:not(:disabled) {
		color: var(--text);
		background: var(--hover);
	}
	.recent {
		display: flex;
		flex-direction: column;
		gap: 2px;
		/* Rows reach past the column by their own padding, so the names line
		   up with the text above while the hover fill still has room. */
		width: calc(100% + 16px);
		margin: 20px -8px 0;
	}
	.recent-head {
		font-size: var(--fs-sm);
		font-weight: 600;
		color: var(--text-muted);
		padding: 0 8px 4px;
	}
	.recent-row {
		display: flex;
		align-items: center;
		gap: 4px;
		border-radius: 8px;
		padding-right: 4px;
	}
	.recent-row:hover {
		background: var(--hover);
	}
	.recent-row:not(:hover):not(:focus-within) :global(.trigger[aria-expanded="false"]) {
		opacity: 0;
	}
	.recent-open {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		justify-content: center;
		gap: 2px;
		height: auto;
		background: none;
		border: none;
		padding: 8px;
		text-align: left;
	}
	.recent-name {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 600;
		font-size: var(--fs-md);
	}
	.recent-path {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		color: var(--text-muted);
	}
	.recent-busy {
		font-size: var(--fs-sm);
		color: var(--text-muted);
		white-space: nowrap;
	}
</style>
