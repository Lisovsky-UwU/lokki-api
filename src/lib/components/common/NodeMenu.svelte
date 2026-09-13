<script lang="ts">
	import type { Snippet } from "svelte";
	// One definition for both ways of opening the same actions: the ⋯ button
	// and the right-click menu.
	import { t } from "../../i18n";
	import type { Menu, MenuItem } from "../../ui/contextMenu";
	import Icon from "./Icon.svelte";
	import MenuList from "./MenuList.svelte";

	// `trigger` replaces the ⋯ button, so a row can *be* the menu button (the
	// workspace name opens the workspace menu) instead of carrying one.
	let {
		menu,
		label,
		trigger,
		align = "right",
	}: { menu: Menu; label?: string; trigger?: Snippet; align?: "left" | "right" } = $props();

	// Derived rather than a default prop value, so the fallback follows a
	// language change like every other string does.
	let title = $derived(label ?? $t("common.actions"));

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
	<button
		class="trigger"
		class:custom={trigger}
		title={title}
		aria-label={title}
		aria-expanded={open}
		onclick={toggle}
	>
		{#if trigger}{@render trigger()}{:else}<Icon name="more" size="1.2em" stroke={2.4} />{/if}
	</button>
	{#if open}
		<div class="menu" class:align-left={align === "left"} role="menu">
			<MenuList {menu} onselect={run} />
		</div>
	{/if}
</div>

<style>
	.node-menu {
		position: relative;
		flex-shrink: 0;
	}
	.trigger {
		display: flex;
		align-items: center;
		background: none;
		border: none;
		cursor: pointer;
		color: inherit;
		opacity: 0.55;
		padding: 0.25em 0.35em;
		border-radius: 4px;
		font-size: 0.95em;
		line-height: 1;
	}
	.trigger:hover {
		opacity: 1;
		background: rgba(127, 127, 127, 0.2);
	}
	/* A custom trigger brings its own layout; only the button chrome is
	   reused. */
	.trigger.custom {
		display: flex;
		align-items: center;
		gap: 0.4em;
		width: 100%;
		opacity: 1;
		font-size: inherit;
		padding: 0;
	}
	.menu {
		position: absolute;
		right: 0;
		left: auto;
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
	.menu.align-left {
		left: 0;
		right: auto;
	}
</style>
