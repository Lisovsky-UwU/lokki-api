<script lang="ts">
	import type { KeyValue } from "../../bindings/types";
	import { newKeyValue } from "../../bindings/types";
	import { t } from "../../i18n";
	import VariableInput from "../common/VariableInput.svelte";

	let { rows, onChange }: { rows: KeyValue[]; onChange: (rows: KeyValue[]) => void } = $props();

	// A trailing blank row is always shown so there is something to type into
	// - including for a brand-new request, whose list starts out empty. It is
	// display-only: blank rows are stripped before they reach the store/disk.
	let displayRows = $derived.by(() => {
		const last = rows[rows.length - 1];
		return !last || last.key !== "" || last.value !== "" ? [...rows, newKeyValue()] : rows;
	});

	function commit(next: KeyValue[]) {
		onChange(next.filter((row) => row.key !== "" || row.value !== ""));
	}

	function update(index: number, patch: Partial<KeyValue>) {
		commit(displayRows.map((row, i) => (i === index ? { ...row, ...patch } : row)));
	}

	function remove(index: number) {
		commit(displayRows.filter((_, i) => i !== index));
	}
</script>

<div class="kv-table">
	{#each displayRows as row, i (i)}
		<div class="kv-row">
			<input
				type="checkbox"
				checked={row.enabled}
				title={$t("kv.toggle")}
				onchange={(e) => update(i, { enabled: (e.target as HTMLInputElement).checked })}
			/>
			<VariableInput mono placeholder={$t("kv.keyPlaceholder")} value={row.key} onChange={(key) => update(i, { key })} />
			<VariableInput
				mono
				placeholder={$t("kv.valuePlaceholder")}
				value={row.value}
				onChange={(value) => update(i, { value })}
			/>
			<button class="remove" title={$t("kv.remove")} onclick={() => remove(i)}>×</button>
		</div>
	{/each}
</div>

<style>
	.kv-table {
		display: flex;
		flex-direction: column;
		gap: 0.3em;
	}
	.kv-row {
		display: flex;
		align-items: center;
		gap: 0.4em;
	}
	.remove {
		background: none;
		border: none;
		cursor: pointer;
		opacity: 0.5;
		font-size: 1.1em;
		line-height: 1;
		padding: 0.2em 0.4em;
	}
	.remove:hover {
		opacity: 1;
	}
</style>
