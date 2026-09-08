<script lang="ts">
	export interface MenuItem {
		label: string;
		action: () => void;
		danger?: boolean;
	}

	let { items, label = "Действия" }: { items: MenuItem[]; label?: string } = $props();

	let open = $state(false);
	let root = $state<HTMLDivElement>();

	function toggle(e: MouseEvent) {
		e.stopPropagation();
		open = !open;
	}

	function run(item: MenuItem, e: MouseEvent) {
		e.stopPropagation();
		open = false;
		item.action();
	}

	// Any click outside (or Escape) dismisses the menu.
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
</script>

<div class="node-menu" bind:this={root}>
	<button class="trigger" title={label} aria-label={label} aria-expanded={open} onclick={toggle}>⋯</button>
	{#if open}
		<div class="menu" role="menu">
			{#each items as item (item.label)}
				<button role="menuitem" class:danger={item.danger} onclick={(e) => run(item, e)}>{item.label}</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.node-menu {
		position: relative;
		flex-shrink: 0;
	}
	.trigger {
		background: none;
		border: none;
		cursor: pointer;
		color: inherit;
		opacity: 0.55;
		padding: 0.1em 0.35em;
		border-radius: 4px;
		font-size: 0.95em;
		line-height: 1;
	}
	.trigger:hover {
		opacity: 1;
		background: rgba(127, 127, 127, 0.2);
	}
	.menu {
		position: absolute;
		right: 0;
		top: 100%;
		z-index: 30;
		min-width: 12em;
		display: flex;
		flex-direction: column;
		padding: 0.25em;
		border-radius: 8px;
		background: var(--modal-bg, #fff);
		border: 1px solid rgba(127, 127, 127, 0.3);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
	}
	.menu button {
		background: none;
		border: none;
		text-align: left;
		padding: 0.45em 0.6em;
		border-radius: 5px;
		cursor: pointer;
		color: inherit;
		font-size: 0.85em;
		white-space: nowrap;
	}
	.menu button:hover {
		background: rgba(127, 127, 127, 0.18);
	}
	.menu button.danger {
		color: #d1443c;
	}
</style>
