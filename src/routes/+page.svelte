<script lang="ts">
	import { onMount } from "svelte";
	import { workspace, workspacePath, collections } from "../lib/stores/workspace";
	import { activeCollection } from "../lib/stores/collectionTree";
	import { activeRequest } from "../lib/stores/activeRequest";
	import { responsesByRequest } from "../lib/stores/response";
	import { api } from "../lib/api/client";
	import { installGlobalErrorReporting, reportError } from "../lib/ui/errors";
	import { layout, updateLayout } from "../lib/stores/layout";
	import ErrorToasts from "../lib/components/common/ErrorToasts.svelte";
	import PromptDialog from "../lib/components/common/PromptDialog.svelte";
	import Splitter from "../lib/components/common/Splitter.svelte";
	import WorkspacePicker from "../lib/components/WorkspacePicker/WorkspacePicker.svelte";
	import CollectionTree from "../lib/components/Sidebar/CollectionTree.svelte";
	import RequestEditorTabs from "../lib/components/RequestEditor/RequestEditorTabs.svelte";
	import ResponseViewer from "../lib/components/ResponseViewer/ResponseViewer.svelte";
	import EnvironmentSwitcher from "../lib/components/EnvironmentSwitcher/EnvironmentSwitcher.svelte";

	let restoring = $state(true);
	let panesHeight = $state(0);

	// Reopen whatever workspace was last used instead of making the user
	// pick the same folder on every launch.
	onMount(async () => {
		installGlobalErrorReporting();
		try {
			const last = await api.getLastWorkspace();
			if (last) {
				const result = await api.openWorkspace(last);
				workspacePath.set(last);
				workspace.set(result.workspace);
				collections.set(result.collections);
			}
		} catch (e) {
			reportError("Не удалось открыть последний workspace", e);
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
		responsesByRequest.set({});
	}
</script>

<PromptDialog />
<ErrorToasts />

{#if restoring}
	<div class="restoring">Загрузка…</div>
{:else if !$workspace}
	<WorkspacePicker />
{:else}
	<div class="app" style="grid-template-columns: {$layout.sidebarWidth}px auto 1fr">
		<aside class="sidebar">
			<CollectionTree />
		</aside>
		<Splitter
			direction="vertical"
			value={$layout.sidebarWidth}
			min={180}
			max={640}
			ariaLabel="Ширина дерева коллекций"
			onResize={(v) => updateLayout({ sidebarWidth: v })}
		/>
		<div class="main">
			<header class="topbar">
				<button class="workspace-name" title="Сменить workspace" onclick={closeWorkspace}>
					{$workspace.name}
					<span class="switch-hint">⇄</span>
				</button>
				<EnvironmentSwitcher />
			</header>
			<div
				class="panes"
				bind:clientHeight={panesHeight}
				style="grid-template-rows: {$layout.editorHeight}px auto 1fr"
			>
				<section class="pane editor-pane">
					<RequestEditorTabs />
				</section>
				<Splitter
					direction="horizontal"
					value={$layout.editorHeight}
					min={140}
					max={Math.max(200, panesHeight - 160)}
					ariaLabel="Высота панели запроса"
					onResize={(v) => updateLayout({ editorHeight: v })}
				/>
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
		color: #24292f;
		background-color: #ffffff;
		/* GitHub light syntax palette, consumed by the code editor. */
		--cm-property: #0550ae;
		--cm-string: #0a3069;
		--cm-number: #0550ae;
		--cm-keyword: #cf222e;
		--cm-comment: #6e7781;
		--cm-punctuation: #24292f;
		--cm-selection: rgba(84, 174, 255, 0.4);
		--cm-active-line: rgba(234, 238, 242, 0.7);
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
			color: #e6edf3;
			background-color: #0d1117;
			--modal-bg: #161b22;
			/* GitHub dark syntax palette. */
			--cm-property: #79c0ff;
			--cm-string: #a5d6ff;
			--cm-number: #79c0ff;
			--cm-keyword: #ff7b72;
			--cm-comment: #8b949e;
			--cm-punctuation: #c9d1d9;
			--cm-selection: rgba(56, 139, 253, 0.4);
			--cm-active-line: rgba(110, 118, 129, 0.1);
		}
		:global(input, select, textarea, button) {
			background: #161b22;
			border-color: rgba(240, 246, 252, 0.15);
		}
	}

	.app {
		display: grid;
		grid-template-rows: minmax(0, 1fr);
		height: 100vh;
	}
	.sidebar {
		border-right: 1px solid rgba(127, 127, 127, 0.25);
		overflow-y: auto;
		min-width: 0;
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
		gap: 1em;
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
		white-space: nowrap;
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
		min-height: 0;
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
