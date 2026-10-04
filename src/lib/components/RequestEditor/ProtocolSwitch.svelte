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
		gap: 2px;
		flex-shrink: 0;
		padding: 2px;
		border: 1px solid var(--line);
		border-radius: 6px;
	}
	.protocol-switch button {
		background: none;
		border: none;
		border-radius: 4px;
		padding: 1px 7px;
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		font-weight: 600;
		color: var(--text-muted);
		cursor: pointer;
	}
	.protocol-switch button:hover:not(:disabled) {
		color: var(--text);
	}
	.protocol-switch button.current {
		color: var(--text);
		background: var(--selected);
	}
	.protocol-switch button:disabled {
		cursor: default;
	}
	.protocol-switch button:disabled:not(.current) {
		opacity: 0.3;
	}
</style>
