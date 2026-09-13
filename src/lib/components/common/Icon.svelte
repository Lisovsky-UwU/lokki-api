<script lang="ts">
	// Draws one icon from `ui/icons`. Outline, stroked in the current text
	// colour, so an icon inherits whatever colours the row it sits in - the
	// red of a destructive menu item included - without a palette of its own.
	import { ICONS, type IconName } from "../../ui/icons";

	// `stroke` is a way out for shapes the shared weight does not suit - the
	// dots of "more" are zero-length segments, so at 1.6 they come out as
	// specks. Everything else stays on the default.
	let { name, size = "1em", stroke = 1.6 }: { name: IconName; size?: string; stroke?: number } = $props();
</script>

<svg
	class="icon"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width={stroke}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	focusable="false"
>
	{#each ICONS[name] as d (d)}
		<path {d} />
	{/each}
</svg>

<style>
	.icon {
		flex-shrink: 0;
	}
</style>
