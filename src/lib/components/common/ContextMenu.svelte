<script lang="ts">
	import { closeContextMenu, contextMenu, type MenuItem } from "../../ui/contextMenu";
	import MenuList from "./MenuList.svelte";

	let menu = $state<HTMLDivElement>();
	let position = $state({ x: 0, y: 0 });

	// Placed at the pointer, then pulled back inside the window once its real
	// size is known - a menu opened near the bottom edge would otherwise hang
	// off the screen with no way to scroll to it.
	$effect(() => {
		const state = $contextMenu;
		if (!state) return;
		position = { x: state.x, y: state.y };
		if (!menu) return;
		const { width, height } = menu.getBoundingClientRect();
		position = {
			x: Math.max(4, Math.min(state.x, window.innerWidth - width - 4)),
			y: Math.max(4, Math.min(state.y, window.innerHeight - height - 4)),
		};
	});

	$effect(() => {
		if (!$contextMenu) return;
		// Capture phase, and pointerdown rather than click: the menu must
		// close on the way down for clicks elsewhere, while its own buttons
		// still get to fire.
		const onPointerDown = (e: PointerEvent) => {
			if (menu && !menu.contains(e.target as Node)) closeContextMenu();
		};
		const onKey = (e: KeyboardEvent) => {
			if (e.key === "Escape") closeContextMenu();
		};
		window.addEventListener("pointerdown", onPointerDown, true);
		window.addEventListener("keydown", onKey);
		// Anything that moves the page out from under the menu dismisses it.
		window.addEventListener("resize", closeContextMenu);
		window.addEventListener("scroll", closeContextMenu, true);
		return () => {
			window.removeEventListener("pointerdown", onPointerDown, true);
			window.removeEventListener("keydown", onKey);
			window.removeEventListener("resize", closeContextMenu);
			window.removeEventListener("scroll", closeContextMenu, true);
		};
	});

	function run(item: MenuItem) {
		closeContextMenu();
		item.action();
	}
</script>

{#if $contextMenu}
	<div
		class="context-menu"
		bind:this={menu}
		role="menu"
		tabindex="-1"
		style="left: {position.x}px; top: {position.y}px"
	>
		<MenuList menu={$contextMenu.menu} onselect={run} />
	</div>
{/if}

<style>
	.context-menu {
		position: fixed;
		z-index: 50;
		min-width: 12em;
		display: flex;
		flex-direction: column;
		padding: 0.25em;
		border-radius: 8px;
		background: var(--modal-bg, #fff);
		border: 1px solid rgba(127, 127, 127, 0.3);
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
	}
</style>
