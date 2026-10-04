<script lang="ts">
	// HTTP or SSE for the open request. Both are described by the same HTTP
	// spec, so switching is lossless and needs no confirmation.
	import { t } from "../../i18n";
	import type { Protocol } from "../../bindings/types";

	let {
		value,
		disabled = false,
		onChange,
	}: { value: Protocol; disabled?: boolean; onChange: (protocol: Protocol) => void } = $props();

	const OPTIONS: { id: Protocol; label: string; hint: "protocol.httpHint" | "protocol.sseHint" }[] = [
		{ id: "http", label: "HTTP", hint: "protocol.httpHint" },
		{ id: "sse", label: "SSE", hint: "protocol.sseHint" },
	];
</script>

<div class="protocol-switch" role="radiogroup" aria-label={$t("protocol.label")}>
	{#each OPTIONS as option (option.id)}
		<button
			role="radio"
			aria-checked={value === option.id}
			class:current={value === option.id}
			title={$t(option.hint)}
			{disabled}
			onclick={() => onChange(option.id)}
		>
			{option.label}
		</button>
	{/each}
</div>

<style>
	.protocol-switch {
		display: flex;
		margin-left: auto;
		flex-shrink: 0;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		overflow: hidden;
	}
	.protocol-switch button {
		background: none;
		border: none;
		border-radius: 0;
		padding: 0.15em 0.6em;
		font-size: 0.75em;
		font-weight: 700;
		letter-spacing: 0.03em;
		color: inherit;
		opacity: 0.55;
		cursor: pointer;
	}
	.protocol-switch button + button {
		border-left: 1px solid var(--line-strong);
	}
	.protocol-switch button.current {
		opacity: 1;
		background: var(--selected);
	}
	.protocol-switch button:disabled {
		cursor: default;
	}
	.protocol-switch button:disabled:not(.current) {
		opacity: 0.3;
	}
</style>
