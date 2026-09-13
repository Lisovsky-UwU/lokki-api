<script lang="ts">
	// The contents of a menu, shared by the two things that can open one: the
	// ⋯ button (`NodeMenu`) and right-click (`ContextMenu`). Only the box
	// around it - where it is positioned and how it is dismissed - differs
	// between them, so only that is left to them.
	import { menuGroups, type Menu, type MenuItem } from "../../ui/contextMenu";
	import Icon from "./Icon.svelte";

	let { menu, onselect }: { menu: Menu; onselect: (item: MenuItem, event: MouseEvent) => void } = $props();

	let groups = $derived(menuGroups(menu));
</script>

{#each groups as group, index (index)}
	{#if index > 0}
		<div class="separator" role="separator"></div>
	{/if}
	{#each group as item (item.label)}
		<button role="menuitem" class:danger={item.danger} onclick={(e) => onselect(item, e)}>
			<Icon name={item.icon} size="1.05em" />
			<span>{item.label}</span>
		</button>
	{/each}
{/each}

<style>
	button {
		display: flex;
		align-items: center;
		gap: 0.55em;
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
	button:hover {
		background: rgba(127, 127, 127, 0.18);
	}
	/* The icon reads as decoration next to its label, not as a second focus
	   point - until the row is pointed at. */
	button :global(.icon) {
		opacity: 0.7;
	}
	button:hover :global(.icon) {
		opacity: 1;
	}
	button.danger {
		color: #d1443c;
	}
	.separator {
		height: 1px;
		margin: 0.25em 0.4em;
		background: rgba(127, 127, 127, 0.25);
	}
</style>
