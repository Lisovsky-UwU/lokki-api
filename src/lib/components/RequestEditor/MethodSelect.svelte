<script lang="ts">
	import type { HttpMethod } from "../../bindings/types";
	import { HTTP_METHODS, methodColor } from "../../ui/methods";

	let { value, onChange }: { value: HttpMethod; onChange: (method: HttpMethod) => void } = $props();

	let open = $state(false);
	let root = $state<HTMLDivElement>();

	// A native <select> can't render a solid colored control consistently,
	// so this is a small custom dropdown using the shared method palette.
	$effect(() => {
		if (!open) return;
		const onPointerDown = (e: PointerEvent) => {
			if (root && !root.contains(e.target as Node)) open = false;
		};
		const onKey = (e: KeyboardEvent) => {
			if (e.key === "Escape") open = false;
		};
		window.addEventListener("pointerdown", onPointerDown, true);
		window.addEventListener("keydown", onKey);
		return () => {
			window.removeEventListener("pointerdown", onPointerDown, true);
			window.removeEventListener("keydown", onKey);
		};
	});

	function pick(method: HttpMethod) {
		open = false;
		onChange(method);
	}
</script>

<div class="method-select" bind:this={root}>
	<button
		class="trigger"
		style="background: {methodColor(value)}"
		aria-haspopup="listbox"
		aria-expanded={open}
		onclick={() => (open = !open)}
	>
		{value}
		<span class="caret">▾</span>
	</button>
	{#if open}
		<ul class="options" role="listbox">
			{#each HTTP_METHODS as method (method)}
				<li>
					<button
						role="option"
						aria-selected={method === value}
						class:current={method === value}
						style="color: {methodColor(method)}"
						onclick={() => pick(method)}
					>
						{method}
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.method-select {
		position: relative;
		flex-shrink: 0;
	}
	.trigger {
		display: flex;
		align-items: center;
		gap: 0.4em;
		border: none;
		border-radius: 6px;
		padding: 0.45em 0.7em;
		min-width: 7em;
		color: #fff;
		font-weight: 700;
		font-size: 0.8em;
		letter-spacing: 0.03em;
		cursor: pointer;
	}
	.caret {
		margin-left: auto;
		opacity: 0.85;
		font-size: 0.9em;
	}
	.options {
		position: absolute;
		top: 100%;
		left: 0;
		z-index: 30;
		margin: 0.25em 0 0;
		padding: 0.25em;
		list-style: none;
		min-width: 100%;
		border-radius: 8px;
		background: var(--modal-bg, #fff);
		border: 1px solid rgba(127, 127, 127, 0.3);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
	}
	.options button {
		display: block;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.35em 0.6em;
		border-radius: 5px;
		cursor: pointer;
		/* Same type treatment as the method badge in the collection tree. */
		font-weight: 700;
		font-size: 0.8em;
		letter-spacing: 0.03em;
	}
	.options button:hover,
	.options button.current {
		background: rgba(127, 127, 127, 0.18);
	}
</style>
