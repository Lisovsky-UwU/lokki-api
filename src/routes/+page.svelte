<script lang="ts">
	import { workspace } from "../lib/stores/workspace";
	import WorkspacePicker from "../lib/components/WorkspacePicker/WorkspacePicker.svelte";
	import CollectionTree from "../lib/components/Sidebar/CollectionTree.svelte";
	import RequestEditorTabs from "../lib/components/RequestEditor/RequestEditorTabs.svelte";
	import ResponseViewer from "../lib/components/ResponseViewer/ResponseViewer.svelte";
	import EnvironmentSwitcher from "../lib/components/EnvironmentSwitcher/EnvironmentSwitcher.svelte";
</script>

{#if !$workspace}
	<WorkspacePicker />
{:else}
	<div class="app">
		<aside class="sidebar">
			<CollectionTree />
		</aside>
		<div class="main">
			<header class="topbar">
				<span class="workspace-name">{$workspace.name}</span>
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
	:global(html, body) {
		margin: 0;
		height: 100%;
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
	}
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5em 1em;
		border-bottom: 1px solid rgba(127, 127, 127, 0.25);
	}
	.workspace-name {
		font-weight: 600;
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
