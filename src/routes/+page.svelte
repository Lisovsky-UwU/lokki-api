<script lang="ts">
	import { onMount } from "svelte";
	import { workspace, workspacePath, collections } from "../lib/stores/workspace";
	import { activeRequest } from "../lib/stores/activeRequest";
	import { api } from "../lib/api/client";
	import { installGlobalErrorReporting, reportError } from "../lib/ui/notices";
	import { closeEnvironmentsDialog, environmentsDialog } from "../lib/ui/environmentsDialog";
	import { closeSettings, openSettings, settingsOpen } from "../lib/ui/settingsDialog";
	import { exitIncognito, incognito, startIncognito } from "../lib/stores/incognito";
	import { confirmAction } from "../lib/ui/dialogs";
	import { layout, updateLayout } from "../lib/stores/layout";
	import Toasts from "../lib/components/common/Toasts.svelte";
	import SettingsModal from "../lib/components/Settings/SettingsModal.svelte";
	import EnvironmentsModal from "../lib/components/EnvironmentSwitcher/EnvironmentsModal.svelte";
	import ConfirmDialog from "../lib/components/common/ConfirmDialog.svelte";
	import PromptDialog from "../lib/components/common/PromptDialog.svelte";
	import Splitter from "../lib/components/common/Splitter.svelte";
	import WorkspacePicker from "../lib/components/WorkspacePicker/WorkspacePicker.svelte";
	import CollectionTree from "../lib/components/Sidebar/CollectionTree.svelte";
	import RequestWorkbench from "../lib/components/RequestWorkbench.svelte";
	import GhostIcon from "../lib/components/common/GhostIcon.svelte";
	import EnvironmentSwitcher from "../lib/components/EnvironmentSwitcher/EnvironmentSwitcher.svelte";

	let restoring = $state(true);

	/// Both directions throw away whatever is on screen, so both ask first
	/// when there is something to lose. Leaving incognito is the harsher of
	/// the two: that request exists nowhere else.
	async function openIncognito() {
		if ($activeRequest?.dirty) {
			const proceed = await confirmAction(
				"В текущем запросе есть несохранённые изменения. Они будут потеряны. Открыть инкогнито-запрос?",
				{ title: "Несохранённые изменения", confirmLabel: "Открыть" },
			);
			if (!proceed) return;
		}
		startIncognito();
	}

	async function leaveIncognito() {
		if ($activeRequest?.dirty) {
			const proceed = await confirmAction(
				"Инкогнито-запрос нигде не сохранён и будет потерян. Выйти из режима?",
				{ title: "Выход из инкогнито", confirmLabel: "Выйти", danger: true },
			);
			if (!proceed) return;
		}
		exitIncognito();
	}

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
</script>

<PromptDialog />
<ConfirmDialog />
<Toasts />
{#if $settingsOpen}
	<SettingsModal onClose={closeSettings} />
{/if}
<!-- Rendered here rather than in the switcher: the collection menu in the
     sidebar opens the same dialog for a collection that isn't the active one. -->
{#if $environmentsDialog && $workspacePath}
	<EnvironmentsModal
		rootPath={$environmentsDialog.rootPath}
		scope={$environmentsDialog.scope}
		title={$environmentsDialog.title}
		workspacePath={$workspacePath}
		onClose={closeEnvironmentsDialog}
	/>
{/if}

{#if restoring}
	<div class="restoring">Загрузка…</div>
{:else if $incognito}
	<!-- No sidebar on purpose: an incognito request belongs to no collection,
	     so there is no tree to place it in. -->
	<div class="app incognito">
		<div class="main">
			<header class="topbar">
				<span class="incognito-badge" title="Запрос нигде не сохраняется">
					<GhostIcon />
					Инкогнито
				</span>
				{#if $workspacePath}
					<EnvironmentSwitcher />
				{:else}
					<span class="hint">Пространство не открыто — переменные недоступны</span>
				{/if}
				<div class="topbar-actions">
					<button class="settings-btn" title="Настройки" aria-label="Настройки" onclick={openSettings}>⚙</button>
					<button class="exit-incognito" onclick={leaveIncognito}>Выйти</button>
				</div>
			</header>
			<RequestWorkbench />
		</div>
	</div>
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
				<EnvironmentSwitcher />
				<div class="topbar-actions">
					<button
						class="settings-btn"
						title="Инкогнито-запрос"
						aria-label="Инкогнито-запрос"
						onclick={openIncognito}
					>
						<GhostIcon />
					</button>
					<button class="settings-btn" title="Настройки" aria-label="Настройки" onclick={openSettings}>⚙</button>
				</div>
			</header>
			{#if !$activeRequest}
				<div class="empty-state-outer">
					<div class="empty-state">
						<p>Выберите запрос слева или создайте новый</p>
					</div>
				</div>
			{:else}
				<RequestWorkbench />
			{/if}
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
		color-scheme: light;
		color: #24292f;
		background-color: #ffffff;
		/* Translucent so the bar takes on whatever panel is behind it
		   (sidebar, editor, dialog) instead of carrying its own colour. */
		--scrollbar-thumb: rgba(27, 31, 36, 0.2);
		--scrollbar-thumb-hover: rgba(27, 31, 36, 0.35);
		/* GitHub light syntax palette, consumed by the code editor. */
		--cm-property: #0550ae;
		--cm-string: #0a3069;
		--cm-number: #0550ae;
		--cm-keyword: #cf222e;
		--cm-comment: #6e7781;
		--cm-punctuation: #24292f;
		--cm-tag: #116329;
		--cm-attribute: #0550ae;
		--cm-function: #8250df;
		--cm-variable: #953800;
		--cm-type: #953800;
		--cm-meta: #6e7781;
		--cm-invalid: #82071e;
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
	:global(*) {
		scrollbar-width: thin;
		scrollbar-color: var(--scrollbar-thumb) transparent;
	}
	:global(::-webkit-scrollbar) {
		width: 10px;
		height: 10px;
	}
	:global(::-webkit-scrollbar-track),
	:global(::-webkit-scrollbar-corner) {
		background: transparent;
	}
	:global(::-webkit-scrollbar-thumb) {
		background: var(--scrollbar-thumb);
		/* Padding-box clipping insets the thumb without painting a track. */
		border: 2px solid transparent;
		background-clip: padding-box;
		border-radius: 6px;
	}
	:global(::-webkit-scrollbar-thumb:hover) {
		background: var(--scrollbar-thumb-hover);
		background-clip: padding-box;
	}
	@media (prefers-color-scheme: dark) {
		:global(:root) {
			color-scheme: dark;
			color: #e6edf3;
			background-color: #0d1117;
			--modal-bg: #161b22;
			--scrollbar-thumb: rgba(240, 246, 252, 0.16);
			--scrollbar-thumb-hover: rgba(240, 246, 252, 0.3);
			/* GitHub dark syntax palette. */
			--cm-property: #79c0ff;
			--cm-string: #a5d6ff;
			--cm-number: #79c0ff;
			--cm-keyword: #ff7b72;
			--cm-comment: #8b949e;
			--cm-punctuation: #c9d1d9;
			--cm-tag: #7ee787;
			--cm-attribute: #79c0ff;
			--cm-function: #d2a8ff;
			--cm-variable: #ffa657;
			--cm-type: #ffa657;
			--cm-meta: #8b949e;
			--cm-invalid: #ffa198;
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
	.topbar-actions {
		display: flex;
		align-items: center;
		gap: 0.4em;
		margin-left: auto;
	}
	.incognito-badge {
		display: flex;
		align-items: center;
		gap: 0.35em;
		font-size: 0.85em;
		font-weight: 600;
		opacity: 0.75;
		white-space: nowrap;
	}
	.exit-incognito {
		background: none;
		border: 1px solid rgba(127, 127, 127, 0.45);
		color: inherit;
		border-radius: 6px;
		padding: 0.25em 0.7em;
		font-size: 0.85em;
		cursor: pointer;
	}
	.exit-incognito:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.hint {
		font-size: 0.85em;
		opacity: 0.6;
	}
	.settings-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		border: none;
		padding: 0.2em 0.4em;
		font-size: 1.1em;
		line-height: 1;
		cursor: pointer;
		opacity: 0.7;
	}
	.settings-btn:hover {
		opacity: 1;
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
	.empty-state-outer {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		min-width: 0;
		height: 100%;
		opacity: 0.5;
		flex-direction: column;
	}
</style>
