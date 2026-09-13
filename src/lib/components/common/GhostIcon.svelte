<script lang="ts">
	// Outline, stroked in the current text colour, so it sits with the other
	// glyph icons instead of standing out as a colour emoji the way 👻 does.
	let { size = "1.05em" }: { size?: string } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.6"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	focusable="false"
>
	<path d="M5 20.5V10a7 7 0 0 1 14 0v10.5l-2.3-1.8-2.3 1.8-2.4-1.8-2.4 1.8L7.3 18.7z" />
	<circle cx="9.5" cy="10" r="1" fill="currentColor" stroke="none" />
	<circle cx="14.5" cy="10" r="1" fill="currentColor" stroke="none" />
</svg>
