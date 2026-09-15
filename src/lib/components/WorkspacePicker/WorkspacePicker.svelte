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
		<GhostIcon size="1.3em" />
	</button>
	<img class="logo" src="{base}/logo-horizontal.png" alt="LokkiAPI" />
	<!-- <p>{$t("picker.tagline")}</p> -->
	<div class="actions">
		<button class="primary" onclick={createWorkspace} disabled={busy !== null}>
			{busy === "create" ? $t("picker.creating") : $t("picker.create")}
		</button>
		<button onclick={openWorkspace} disabled={busy !== null}>
			{busy === "open" ? $t("picker.opening") : $t("picker.openExisting")}
		</button>
	</div>
	<p class="hint">{$t("picker.hint")}</p>
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

<style>
	.logo {
		/* The wordmark is white on transparency, so the asset ships on a plate
		   the colour of the dark theme's background: it disappears into the
		   page there, and stays a readable dark banner on a light one. */
		width: 380px;
		max-width: 80vw;
		height: auto;
		border-radius: 12px;
		margin-bottom: 0.25rem;
	}
	.picker {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 0.75rem;
		text-align: center;
		padding: 2rem;
		/* The recent list is the one part that can outgrow the window. */
		overflow-y: auto;
	}
	.actions {
		display: flex;
		gap: 0.6rem;
		flex-wrap: wrap;
		justify-content: center;
	}
	button {
		padding: 0.6em 1.4em;
		border-radius: 8px;
		border: 1px solid rgba(127, 127, 127, 0.45);
		background: transparent;
		color: inherit;
		cursor: pointer;
		font-size: 1em;
	}
	.primary {
		border-color: #396cd8;
		background: #396cd8;
		color: white;
	}
	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.ghost {
		position: absolute;
		top: 1rem;
		right: 1rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		background: none;
		border-radius: 6px;
		padding: 0.4em;
		opacity: 0.6;
	}
	.ghost:hover:not(:disabled) {
		opacity: 1;
		background: rgba(127, 127, 127, 0.15);
	}
	.recent {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		width: min(34rem, 100%);
		text-align: left;
		margin-top: 0.5rem;
	}
	.recent-head {
		font-size: 0.75em;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		opacity: 0.6;
		padding: 0 0.2em 0.2em;
	}
	.recent-row {
		display: flex;
		align-items: center;
		gap: 0.2em;
		border-radius: 8px;
		padding-right: 0.3em;
	}
	.recent-row:hover {
		background: rgba(127, 127, 127, 0.12);
	}
	.recent-open {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.1em;
		background: none;
		border: none;
		padding: 0.5em 0.6em;
		font-size: 1em;
		text-align: left;
	}
	.recent-name {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 600;
		font-size: 0.95em;
	}
	.recent-path {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: ui-monospace, monospace;
		font-size: 0.78em;
		opacity: 0.6;
	}
	.recent-busy {
		font-size: 0.8em;
		opacity: 0.6;
		white-space: nowrap;
	}
	.hint {
		margin: 0;
		font-size: 0.85em;
		opacity: 0.6;
		margin-bottom: 1em;
	}
</style>
