<script lang="ts">
	import { confirmRequest } from "../../ui/dialogs";

	let confirmButton = $state<HTMLButtonElement>();

	// Focus the confirming action so Enter/Escape work without reaching for
	// the mouse, the way the OS dialog did.
	$effect(() => {
		if ($confirmRequest) queueMicrotask(() => confirmButton?.focus());
	});

	function answer(value: boolean) {
		const request = $confirmRequest;
		confirmRequest.set(null);
		request?.resolve(value);
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if ($confirmRequest && e.key === "Escape") {
			e.preventDefault();
			answer(false);
		}
	}}
/>

{#if $confirmRequest}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) answer(false);
		}}
	>
		<div class="dialog" role="alertdialog" aria-modal="true" aria-label={$confirmRequest.title}>
			<h2>{$confirmRequest.title}</h2>
			<p>{$confirmRequest.message}</p>
			<div class="actions">
				<button type="button" onclick={() => answer(false)}>Отмена</button>
				<button
					bind:this={confirmButton}
					type="button"
					class="primary"
					class:danger={$confirmRequest.danger}
					onclick={() => answer(true)}
				>
					{$confirmRequest.confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
	}
	.dialog {
		background: var(--modal-bg, #fff);
		border-radius: 10px;
		padding: 1.2em;
		width: min(26em, 90vw);
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.35);
		display: flex;
		flex-direction: column;
		gap: 0.7em;
	}
	h2 {
		margin: 0;
		font-size: 1em;
	}
	p {
		margin: 0;
		font-size: 0.9em;
		opacity: 0.85;
		line-height: 1.45;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5em;
		margin-top: 0.3em;
	}
	.primary {
		background: #396cd8;
		border-color: #396cd8;
		color: white;
	}
	.primary.danger {
		background: #d1443c;
		border-color: #d1443c;
	}
</style>
