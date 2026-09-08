<script lang="ts">
	import { api } from "../../api/client";
	import { workspace, workspacePath, collections } from "../../stores/workspace";

	let error = $state<string | null>(null);
	let loading = $state(false);

	async function pickAndOpen() {
		error = null;
		try {
			const folder = await api.pickWorkspaceFolder();
			if (!folder) return;
			loading = true;
			const result = await api.openWorkspace(folder);
			workspacePath.set(folder);
			workspace.set(result.workspace);
			collections.set(result.collections);
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}
</script>

<div class="picker">
	<h1>LokkiAPI</h1>
	<p>Локальный, файловый клиент для тестирования API.</p>
	<button onclick={pickAndOpen} disabled={loading}>
		{loading ? "Открываем..." : "Открыть папку workspace"}
	</button>
	{#if error}
		<p class="error">{error}</p>
	{/if}
</div>

<style>
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
	h1 {
		margin: 0;
	}
	button {
		padding: 0.6em 1.4em;
		border-radius: 8px;
		border: 1px solid #396cd8;
		background: #396cd8;
		color: white;
		cursor: pointer;
		font-size: 1em;
	}
	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.error {
		color: #d33;
	}
</style>
