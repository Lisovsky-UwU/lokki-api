<script lang="ts">
	import { t } from "../../i18n";
	import { availableVariables } from "../../stores/environments";

	let {
		value,
		onChange,
		placeholder = "",
		type = "text",
		mono = false,
		ariaLabel = "",
	}: {
		value: string;
		onChange: (value: string) => void;
		placeholder?: string;
		type?: "text" | "password";
		mono?: boolean;
		ariaLabel?: string;
	} = $props();

	let input = $state<HTMLInputElement>();
	let open = $state(false);
	let query = $state("");
	let highlighted = $state(0);
	// Where the `{{` that triggered the suggestions starts.
	let triggerStart = $state(-1);

	let matches = $derived(
		open ? $availableVariables.filter((v) => v.key.toLowerCase().startsWith(query.toLowerCase())).slice(0, 8) : [],
	);

	/// Suggestions appear while the caret sits inside an unclosed `{{ ... }}`
	/// that was just typed, so the list follows what is actually being written.
	function updateSuggestions() {
		const el = input;
		if (!el) return;
		const caret = el.selectionStart ?? el.value.length;
		const before = el.value.slice(0, caret);
		const start = before.lastIndexOf("{{");
		if (start === -1 || before.slice(start).includes("}}")) {
			open = false;
			triggerStart = -1;
			return;
		}
		triggerStart = start;
		query = before.slice(start + 2).trim();
		highlighted = 0;
		open = true;
	}

	function apply(key: string) {
		const el = input;
		if (!el || triggerStart < 0) return;
		const start = triggerStart;
		const caret = el.selectionStart ?? el.value.length;
		const next = `${el.value.slice(0, start)}{{${key}}}${el.value.slice(caret)}`;
		open = false;
		triggerStart = -1;
		onChange(next);
		// Caret goes right after the inserted `{{key}}` - computed from the
		// saved start, not the field that was just reset.
		const cursor = start + key.length + 4;
		queueMicrotask(() => {
			el.value = next;
			el.setSelectionRange(cursor, cursor);
		});
	}

	function onKeyDown(e: KeyboardEvent) {
		if (!open || matches.length === 0) return;
		if (e.key === "ArrowDown") {
			e.preventDefault();
			highlighted = (highlighted + 1) % matches.length;
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			highlighted = (highlighted - 1 + matches.length) % matches.length;
		} else if (e.key === "Enter" || e.key === "Tab") {
			e.preventDefault();
			apply(matches[highlighted].key);
		} else if (e.key === "Escape") {
			open = false;
		}
	}
</script>

<div class="variable-input">
	<input
		bind:this={input}
		{type}
		{placeholder}
		aria-label={ariaLabel || placeholder}
		class:mono
		{value}
		oninput={(e) => {
			onChange((e.target as HTMLInputElement).value);
			updateSuggestions();
		}}
		onkeydown={onKeyDown}
		onclick={updateSuggestions}
		onblur={() => setTimeout(() => (open = false), 120)}
	/>
	{#if open && matches.length > 0}
		<ul class="suggestions">
			{#each matches as match, i (match.key)}
				<li>
					<button
						type="button"
						class:highlighted={i === highlighted}
						onmousedown={(e) => {
							e.preventDefault();
							apply(match.key);
						}}
					>
						<span class="key">{match.key}</span>
						<span class="value">{match.secret ? "••••" : match.value}</span>
						<span class="scope"
							>{$t(match.scope === "collection" ? "variables.scope.collection" : "variables.scope.global")}</span
						>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.variable-input {
		position: relative;
		flex: 1;
		min-width: 0;
	}
	input {
		width: 100%;
	}
	input.mono {
		font-family: ui-monospace, monospace;
	}
	.suggestions {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		z-index: 25;
		margin: 0.2em 0 0;
		padding: 0.2em;
		list-style: none;
		border-radius: 8px;
		background: var(--modal-bg, #fff);
		border: 1px solid rgba(127, 127, 127, 0.3);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
		max-height: 14em;
		overflow-y: auto;
	}
	.suggestions button {
		display: flex;
		align-items: baseline;
		gap: 0.6em;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.35em 0.5em;
		border-radius: 5px;
		cursor: pointer;
		color: inherit;
		font-size: 0.85em;
	}
	.suggestions button:hover,
	.suggestions button.highlighted {
		background: rgba(57, 108, 216, 0.2);
	}
	.key {
		font-family: ui-monospace, monospace;
		font-weight: 600;
	}
	.value {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		opacity: 0.6;
	}
	.scope {
		opacity: 0.45;
		font-size: 0.8em;
		white-space: nowrap;
	}
</style>
