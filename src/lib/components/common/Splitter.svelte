<script lang="ts">
	let {
		direction,
		value,
		min,
		max,
		onResize,
		ariaLabel,
	}: {
		/// "vertical" = a vertical bar that resizes width; "horizontal" = a
		/// horizontal bar that resizes height.
		direction: "vertical" | "horizontal";
		value: number;
		min: number;
		max: number;
		onResize: (value: number) => void;
		ariaLabel: string;
	} = $props();

	let dragging = $state(false);
	let origin = 0;
	let startValue = 0;

	function clamp(v: number) {
		return Math.min(max, Math.max(min, v));
	}

	function onPointerDown(e: PointerEvent) {
		dragging = true;
		origin = direction === "vertical" ? e.clientX : e.clientY;
		startValue = value;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!dragging) return;
		const delta = (direction === "vertical" ? e.clientX : e.clientY) - origin;
		onResize(clamp(startValue + delta));
	}

	function onPointerUp(e: PointerEvent) {
		dragging = false;
		(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
	}

	function onKeyDown(e: KeyboardEvent) {
		const step = e.shiftKey ? 40 : 12;
		const back = direction === "vertical" ? "ArrowLeft" : "ArrowUp";
		const forward = direction === "vertical" ? "ArrowRight" : "ArrowDown";
		if (e.key === back) {
			e.preventDefault();
			onResize(clamp(value - step));
		} else if (e.key === forward) {
			e.preventDefault();
			onResize(clamp(value + step));
		}
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="splitter {direction}"
	class:dragging
	role="separator"
	tabindex="0"
	aria-label={ariaLabel}
	aria-orientation={direction === "vertical" ? "vertical" : "horizontal"}
	aria-valuenow={Math.round(value)}
	aria-valuemin={min}
	aria-valuemax={max}
	onpointerdown={onPointerDown}
	onpointermove={onPointerMove}
	onpointerup={onPointerUp}
	onkeydown={onKeyDown}
></div>

<style>
	.splitter {
		flex-shrink: 0;
		background: transparent;
		transition: background 0.12s;
	}
	.splitter:hover,
	.splitter.dragging,
	.splitter:focus-visible {
		background: rgba(57, 108, 216, 0.45);
		outline: none;
	}
	.vertical {
		width: 5px;
		cursor: col-resize;
	}
	.horizontal {
		height: 5px;
		cursor: row-resize;
	}
</style>
