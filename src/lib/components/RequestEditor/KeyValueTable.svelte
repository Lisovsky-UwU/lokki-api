<script lang="ts">
	import type { KeyValue } from "../../bindings/types";
	import { newKeyValue } from "../../bindings/types";

	let { rows, onChange }: { rows: KeyValue[]; onChange: (rows: KeyValue[]) => void } = $props();

	function update(index: number, patch: Partial<KeyValue>) {
		const next = rows.map((row, i) => (i === index ? { ...row, ...patch } : row));
		ensureTrailingBlankRow(next);
	}

	function remove(index: number) {
		const next = rows.filter((_, i) => i !== index);
		onChange(next);
	}

	function ensureTrailingBlankRow(next: KeyValue[]) {
		const last = next[next.length - 1];
		if (!last || last.key !== "" || last.value !== "") {
			next.push(newKeyValue());
		}
		onChange(next);
	}
</script>

<div class="kv-table">
	{#each rows as row, i (i)}
		<div class="kv-row">
			<input
				type="checkbox"
				checked={row.enabled}
				onchange={(e) => update(i, { enabled: (e.target as HTMLInputElement).checked })}
			/>
			<input
				class="kv-key"
				placeholder="key"
				value={row.key}
				oninput={(e) => update(i, { key: (e.target as HTMLInputElement).value })}
			/>
			<input
				class="kv-value"
				placeholder="value"
				value={row.value}
				oninput={(e) => update(i, { value: (e.target as HTMLInputElement).value })}
			/>
			<button class="remove" title="Удалить" onclick={() => remove(i)}>×</button>
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
	.kv-key,
	.kv-value {
		flex: 1;
		min-width: 0;
		font-family: ui-monospace, monospace;
		font-size: 0.9em;
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
