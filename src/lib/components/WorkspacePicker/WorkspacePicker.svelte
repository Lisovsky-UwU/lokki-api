<script lang="ts">
	import { base } from "$app/paths";
	import { api } from "../../api/client";
	import { workspace, workspacePath, collections } from "../../stores/workspace";
	import type { OpenWorkspaceResult } from "../../api/client";
	import { promptForText } from "../../ui/dialogs";

	let error = $state<string | null>(null);
	let busy = $state<"create" | "open" | null>(null);

	function enter(path: string, result: OpenWorkspaceResult) {
		workspacePath.set(path);
		workspace.set(result.workspace);
		collections.set(result.collections);
	}

	/// Picks a folder and initializes it. The name is asked for separately
	/// because it is metadata, not the folder name — it can be changed later
	/// without moving anything on disk.
	async function createWorkspace() {
		error = null;
		try {
			const folder = await api.pickWorkspaceFolder();
			if (!folder) return;
			const suggested = folder.split(/[\\/]+/).filter(Boolean).pop() ?? "Новое пространство";
			const name = await promptForText("Новое пространство", "Название пространства", suggested);
			if (!name) return;
			busy = "create";
			enter(folder, await api.createWorkspace(folder, name));
		} catch (e) {
			error = String(e);
		} finally {
			busy = null;
		}
	}

	/// Opening only ever opens: a folder that isn't a workspace is reported
	/// as such instead of quietly becoming a new empty one.
	async function openWorkspace() {
		error = null;
		try {
			const folder = await api.pickWorkspaceFolder();
			if (!folder) return;
			busy = "open";
			enter(folder, await api.openWorkspace(folder));
		} catch (e) {
			error = String(e);
		} finally {
			busy = null;
		}
	}
</script>

<div class="picker">
	<img class="logo" src="{base}/logo-horizontal.png" alt="LokkiAPI" />
	<p>Локальный, файловый клиент для тестирования API.</p>
	<div class="actions">
		<button class="primary" onclick={createWorkspace} disabled={busy !== null}>
			{busy === "create" ? "Создаём..." : "Создать пространство"}
		</button>
		<button onclick={openWorkspace} disabled={busy !== null}>
			{busy === "open" ? "Открываем..." : "Открыть существующее"}
		</button>
	</div>
	<p class="hint">Пространство — это обычная папка на диске с коллекциями и окружениями.</p>
	{#if error}
		<p class="error">{error}</p>
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
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		gap: 0.75rem;
		text-align: center;
		padding: 2rem;
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
	.hint {
		margin: 0;
		font-size: 0.85em;
		opacity: 0.6;
	}
	.error {
		color: #d33;
	}
</style>
