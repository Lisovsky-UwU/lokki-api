<script lang="ts">
	// The two shortcuts worth knowing, shown where the screen would otherwise
	// be empty - an empty pane is the one moment the user is looking for
	// what to do next.
	import { t } from "../../i18n";
	import { MOD_KEY } from "../../ui/keys";

	let { save = true }: { save?: boolean } = $props();
</script>

<dl class="shortcuts">
	<div>
		<dt><kbd>{MOD_KEY}</kbd><kbd>↵</kbd></dt>
		<dd>{$t("app.shortcutSend")}</dd>
	</div>
	{#if save}
		<div>
			<dt><kbd>{MOD_KEY}</kbd><kbd>S</kbd></dt>
			<dd>{$t("app.shortcutSave")}</dd>
		</div>
	{/if}
</dl>

<style>
	.shortcuts {
		display: grid;
		gap: 6px;
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--text-muted);
	}
	.shortcuts div {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	dt {
		display: flex;
		gap: 3px;
		/* Keys line up whatever the length of the modifier's name. */
		min-width: 5.5em;
		justify-content: flex-end;
	}
	dd {
		margin: 0;
	}
	kbd {
		min-width: 22px;
		padding: 1px 6px;
		border: 1px solid var(--line-strong);
		border-bottom-width: 2px;
		border-radius: 5px;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		text-align: center;
		color: var(--text);
		background: var(--surface-raised);
	}
</style>
