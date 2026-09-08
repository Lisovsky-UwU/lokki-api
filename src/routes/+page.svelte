<script lang="ts">
	import { onMount } from "svelte";
	import { workspace, workspacePath, collections } from "../lib/stores/workspace";
	import { activeCollection } from "../lib/stores/collectionTree";
	import { activeRequest } from "../lib/stores/activeRequest";
	import { responseState } from "../lib/stores/response";
	import { api } from "../lib/api/client";
	import WorkspacePicker from "../lib/components/WorkspacePicker/WorkspacePicker.svelte";
	import CollectionTree from "../lib/components/Sidebar/CollectionTree.svelte";
	import RequestEditorTabs from "../lib/components/RequestEditor/RequestEditorTabs.svelte";
	import ResponseViewer from "../lib/components/ResponseViewer/ResponseViewer.svelte";
	import EnvironmentSwitcher from "../lib/components/EnvironmentSwitcher/EnvironmentSwitcher.svelte";

	let restoring = $state(true);

	// Reopen whatever workspace was last used instead of making the user
	// pick the same folder on every launch.
	onMount(async () => {
		try {
			const last = await api.getLastWorkspace();
			if (last) {
				const result = await api.openWorkspace(last);
				workspacePath.set(last);
				workspace.set(result.workspace);
				collections.set(result.collections);
			}
		} catch (e) {
			console.error("failed to restore last workspace", e);
		} finally {
			restoring = false;
		}
	});

	function closeWorkspace() {
		workspace.set(null);
		workspacePath.set(null);
		collections.set([]);
		activeCollection.set(null);
		activeRequest.set(null);
		responseState.set({ outcome: null, error: null, loading: false });
	}
</script>

{#if restoring}
	<div class="restoring">Загрузка…</div>
{:else if !$workspace}
	<WorkspacePicker />
{:else}
	<div class="app">
		<aside class="sidebar">
			<CollectionTree />
		</aside>
		<div class="main">
			<header class="topbar">
				<button class="workspace-name" title="Сменить workspace" onclick={closeWorkspace}>
					{$workspace.name}
					<span class="switch-hint">⇄</span>
				</button>
				<EnvironmentSwitcher />
			</header>
			<div class="panes">
				<section class="pane editor-pane">
					<RequestEditorTabs />
				</section>
				<section class="pane response-pane">
					<ResponseViewer />
				</section>
			</div>
		</div>
	</div>
{/if}

<style>
	:global(*, *::before, *::after) {
		box-sizing: border-box;
	}
	:global(html, body) {
		margin: 0;
		height: 100%;
		overflow: hidden;
	}
	:global(:root) {
		font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
		color: #0f0f0f;
		background-color: #f6f6f6;
	}
	:global(input, select, textarea, button) {
		font-family: inherit;
		border-radius: 6px;
		border: 1px solid rgba(127, 127, 127, 0.35);
		padding: 0.4em 0.6em;
		background: white;
		color: inherit;
	}
	:global(button) {
		cursor: pointer;
	}
	@media (prefers-color-scheme: dark) {
		:global(:root) {
			color: #f0f0f0;
			background-color: #1e1e1e;
			--modal-bg: #2a2a2a;
		}
		:global(input, select, textarea, button) {
			background: #2a2a2a;
			border-color: rgba(255, 255, 255, 0.2);
		}
	}

	.app {
		display: grid;
		grid-template-columns: 260px 1fr;
		grid-template-rows: minmax(0, 1fr);
		height: 100vh;
	}
	.sidebar {
		border-right: 1px solid rgba(127, 127, 127, 0.25);
		overflow-y: auto;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5em 1em;
		border-bottom: 1px solid rgba(127, 127, 127, 0.25);
	}
	.restoring {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		opacity: 0.6;
	}
	.workspace-name {
		display: flex;
		align-items: center;
		gap: 0.4em;
		font-weight: 600;
		background: none;
		border: none;
		padding: 0.2em 0.4em;
		border-radius: 6px;
		cursor: pointer;
		color: inherit;
	}
	.workspace-name:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.switch-hint {
		opacity: 0.5;
		font-size: 0.85em;
	}
	.panes {
		flex: 1;
		display: grid;
		grid-template-rows: 1.1fr 1fr;
		min-height: 0;
		gap: 0.5em;
		padding: 0.8em;
	}
	.pane {
		min-height: 0;
		overflow: hidden;
	}
	.editor-pane {
		display: flex;
	}
	.response-pane {
		border-top: 1px solid rgba(127, 127, 127, 0.2);
		padding-top: 0.6em;
	}
</style>
