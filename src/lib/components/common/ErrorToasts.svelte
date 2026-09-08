<script lang="ts">
	import { dismissError, errorNotices } from "../../ui/errors";
</script>

{#if $errorNotices.length > 0}
	<div class="toasts">
		{#each $errorNotices as notice (notice.id)}
			<div class="toast">
				<span>{notice.message}</span>
				<button title="Скрыть" onclick={() => dismissError(notice.id)}>×</button>
			</div>
		{/each}
	</div>
{/if}

<style>
	.toasts {
		position: fixed;
		right: 1em;
		bottom: 1em;
		z-index: 40;
		display: flex;
		flex-direction: column;
		gap: 0.4em;
		max-width: min(34em, 80vw);
	}
	.toast {
		display: flex;
		align-items: flex-start;
		gap: 0.6em;
		padding: 0.6em 0.8em;
		border-radius: 8px;
		background: #d1443c;
		color: white;
		font-size: 0.85em;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
	}
	.toast span {
		flex: 1;
		word-break: break-word;
	}
	.toast button {
		background: none;
		border: none;
		color: inherit;
		cursor: pointer;
		font-size: 1.1em;
		line-height: 1;
		padding: 0;
	}
</style>
