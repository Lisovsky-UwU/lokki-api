<script lang="ts">
	// A failure the user has to acknowledge, as opposed to `ConfirmDialog`,
	// which asks a question, and `Toasts`, which reports alongside whatever
	// they do next. Kept separate from the confirm dialog rather than given a
	// "hide the cancel button" flag: a dialog with nothing to decline is not
	// a confirmation, and one of the two would end up wording the other's
	// buttons.
	import { t } from "../../i18n";
	import { alertRequest } from "../../ui/dialogs";

	let okButton = $state<HTMLButtonElement>();

	// Focused so Enter and Escape both dismiss without reaching for the
	// mouse, matching ConfirmDialog.
	$effect(() => {
		if ($alertRequest) queueMicrotask(() => okButton?.focus());
	});

	function dismiss() {
		const request = $alertRequest;
		alertRequest.set(null);
		request?.resolve();
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if ($alertRequest && e.key === "Escape") {
			e.preventDefault();
			dismiss();
		}
	}}
/>

{#if $alertRequest}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) dismiss();
		}}
	>
		<div class="dialog" role="alertdialog" aria-modal="true" aria-label={$alertRequest.title}>
			<h2>{$alertRequest.title}</h2>
			<p>{$alertRequest.message}</p>
			<div class="actions">
				<button bind:this={okButton} type="button" class="primary" onclick={dismiss}>{$t("common.ok")}</button>
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
		/* Above the confirm dialog: a failure can be raised by the flow the
		   confirm dialog just started. */
		z-index: 60;
	}
	.dialog {
		background: var(--modal-bg, #fff);
		border-radius: 10px;
		padding: 1.2em;
		width: min(30em, 90vw);
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
		/* Messages name the folder that was refused, and a path has no spaces
		   to wrap on. */
		overflow-wrap: anywhere;
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
</style>
