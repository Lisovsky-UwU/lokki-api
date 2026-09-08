<script lang="ts">
	import { promptRequest } from "../../ui/dialogs";

	let value = $state("");
	let input = $state<HTMLInputElement>();

	// Seed the field each time a new request arrives, and focus it so the
	// user can just type — same immediacy the native prompt had.
	$effect(() => {
		const request = $promptRequest;
		if (!request) return;
		value = request.initial;
		queueMicrotask(() => {
			input?.focus();
			input?.select();
		});
	});

	function submit() {
		const request = $promptRequest;
		if (!request) return;
		const trimmed = value.trim();
		promptRequest.set(null);
		request.resolve(trimmed === "" ? null : trimmed);
	}

	function cancel() {
		const request = $promptRequest;
		promptRequest.set(null);
		request?.resolve(null);
	}
</script>

{#if $promptRequest}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) cancel();
		}}
	>
		<form
			class="dialog"
			aria-label={$promptRequest.title}
			onsubmit={(e) => {
				e.preventDefault();
				submit();
			}}
		>
			<h2>{$promptRequest.title}</h2>
			<label>
				<span>{$promptRequest.label}</span>
				<!-- svelte-ignore a11y_autofocus -->
				<input bind:this={input} bind:value onkeydown={(e) => e.key === "Escape" && cancel()} />
			</label>
			<div class="actions">
				<button type="button" onclick={cancel}>Отмена</button>
				<button type="submit" class="primary">OK</button>
			</div>
		</form>
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
		z-index: 20;
	}
	.dialog {
		background: var(--modal-bg, #fff);
		border-radius: 10px;
		padding: 1.2em;
		width: min(24em, 90vw);
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
		display: flex;
		flex-direction: column;
		gap: 0.8em;
	}
	h2 {
		margin: 0;
		font-size: 1em;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.3em;
		font-size: 0.85em;
	}
	label span {
		opacity: 0.7;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5em;
	}
	.primary {
		background: #396cd8;
		color: white;
		border-color: #396cd8;
	}
</style>
