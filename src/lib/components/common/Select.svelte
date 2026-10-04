<script lang="ts" module>
	export interface SelectOption {
		value: string;
		label: string;
		/// Nesting depth, for options that stand for a tree (folders).
		depth?: number;
		/// A colour of its own, for options that carry one everywhere else
		/// (HTTP methods).
		color?: string;
	}
</script>

<script lang="ts">
	// A drop-down drawn by the app rather than the webview. The native list
	// is painted by the platform: it ignores the theme's fonts, radii and
	// shadows, and in the dark theme it came out as a white sheet. This one
	// is the same popover as every other menu in the app, and keeps the
	// listbox keyboard model - arrows, Home/End, Enter, Escape, and a typed
	// letter jumping to the next option that starts with it.
	import Icon from "./Icon.svelte";

	let {
		value,
		options,
		onChange,
		id,
		ariaLabel,
		title,
		variant = "field",
		mono = false,
		muted = false,
		class: className = "",
	}: {
		value: string;
		options: SelectOption[];
		onChange: (value: string) => void;
		id?: string;
		ariaLabel?: string;
		title?: string;
		/// "field" looks like an input; "bare" leaves the frame to whatever
		/// the select sits in (the environment pickers, the URL field).
		variant?: "field" | "bare";
		mono?: boolean;
		/// Shows the current value dimmed - for a "none" that should read as
		/// an absence rather than a choice.
		muted?: boolean;
		class?: string;
	} = $props();

	const listId = `select-${Math.random().toString(36).slice(2, 10)}`;

	let open = $state(false);
	let active = $state(0);
	let trigger = $state<HTMLButtonElement>();
	let list = $state<HTMLUListElement>();
	let position = $state({ left: 0, top: 0, minWidth: 0, maxHeight: 0, above: false });

	let selected = $derived(options.find((o) => o.value === value));

	function show() {
		if (!trigger || options.length === 0) return;
		active = Math.max(
			0,
			options.findIndex((o) => o.value === value),
		);
		// Fixed rather than absolute, so a dialog's `overflow: hidden` can't
		// clip the list. It opens downwards unless the window has more room
		// above the trigger than below it.
		const rect = trigger.getBoundingClientRect();
		const below = window.innerHeight - rect.bottom - 8;
		const above = rect.top - 8;
		const up = below < 200 && above > below;
		position = {
			left: rect.left,
			top: up ? rect.top - 4 : rect.bottom + 4,
			minWidth: rect.width,
			maxHeight: Math.max(120, Math.min(320, up ? above : below)),
			above: up,
		};
		open = true;
	}

	function hide(refocus = true) {
		open = false;
		if (refocus) trigger?.focus();
	}

	function pick(index: number) {
		const option = options[index];
		hide();
		if (option && option.value !== value) onChange(option.value);
	}

	// Focus moves into the list while it is open, so the keys below reach it
	// and not the dialog it sits in.
	$effect(() => {
		if (!open || !list) return;
		list.focus();
		list.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
	});

	$effect(() => {
		if (!open) return;
		const onPointerDown = (e: PointerEvent) => {
			const target = e.target as Node;
			if (!list?.contains(target) && !trigger?.contains(target)) hide(false);
		};
		// Anything that moves the trigger out from under the list closes it,
		// as the context menu does - a fixed list would be left behind.
		const onMove = (e: Event) => {
			if (e.target instanceof Node && list?.contains(e.target)) return;
			hide(false);
		};
		window.addEventListener("pointerdown", onPointerDown, true);
		window.addEventListener("resize", onMove);
		window.addEventListener("scroll", onMove, true);
		return () => {
			window.removeEventListener("pointerdown", onPointerDown, true);
			window.removeEventListener("resize", onMove);
			window.removeEventListener("scroll", onMove, true);
		};
	});

	/// Moves the list to the end of <body>. Left where it is, it would sit
	/// inside whatever wraps the select - and inside a <label>, a click on an
	/// option counts as a click on the label, which presses the trigger and
	/// opens the list again.
	function portal(node: HTMLElement) {
		document.body.appendChild(node);
		return { destroy: () => node.remove() };
	}

	function onTriggerKey(e: KeyboardEvent) {
		if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
			e.preventDefault();
			show();
		}
	}

	function onListKey(e: KeyboardEvent) {
		// Kept away from the window, where dialogs close on Escape.
		e.stopPropagation();
		const last = options.length - 1;
		switch (e.key) {
			case "ArrowDown":
				active = Math.min(last, active + 1);
				break;
			case "ArrowUp":
				active = Math.max(0, active - 1);
				break;
			case "Home":
				active = 0;
				break;
			case "End":
				active = last;
				break;
			case "Enter":
			case " ":
				pick(active);
				break;
			case "Escape":
				hide();
				break;
			case "Tab":
				hide(false);
				return;
			default:
				if (e.key.length === 1) {
					const letter = e.key.toLowerCase();
					const order = [...options.keys()].map((i) => (active + 1 + i) % options.length);
					const hit = order.find((i) => options[i].label.trim().toLowerCase().startsWith(letter));
					if (hit !== undefined) active = hit;
				} else return;
		}
		e.preventDefault();
		list?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
	}
</script>

<button
	bind:this={trigger}
	{id}
	type="button"
	class="trigger {variant} {className}"
	class:mono
	class:muted
	class:open
	{title}
	aria-label={ariaLabel}
	aria-haspopup="listbox"
	aria-expanded={open}
	aria-controls={open ? listId : undefined}
	style:color={selected?.color}
	onclick={() => (open ? hide() : show())}
	onkeydown={onTriggerKey}
>
	<span class="label">{selected?.label ?? ""}</span>
	<span class="caret"><Icon name="chevron" size="14px" /></span>
</button>

{#if open}
	<ul
		use:portal
		bind:this={list}
		id={listId}
		class="list"
		class:mono
		class:above={position.above}
		role="listbox"
		tabindex="-1"
		aria-label={ariaLabel}
		aria-activedescendant="{listId}-{active}"
		style:left="{position.left}px"
		style:top="{position.top}px"
		style:min-width="{position.minWidth}px"
		style:max-height="{position.maxHeight}px"
		onkeydown={onListKey}
	>
		{#each options as option, i (option.value)}
			<!-- The keys are handled once, on the list: options are pointed at
			     through aria-activedescendant and never take focus themselves. -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<li
				id="{listId}-{i}"
				data-index={i}
				role="option"
				aria-selected={option.value === value}
				class:active={i === active}
				style:padding-left={option.depth ? `${0.6 + option.depth * 1.1}em` : undefined}
				style:color={option.color}
				onpointermove={() => (active = i)}
				onclick={() => pick(i)}
			>
				<span class="option-label">{option.label}</span>
				{#if option.value === value}<span class="check"><Icon name="check" size="14px" /></span>{/if}
			</li>
		{/each}
	</ul>
{/if}

<style>
	.trigger {
		display: flex;
		align-items: center;
		gap: 0.5em;
		min-width: 0;
		cursor: pointer;
		text-align: left;
	}
	.trigger.field {
		padding: 0.4em 0.5em 0.4em 0.6em;
	}
	.trigger.bare {
		border: none;
		background: none;
		border-radius: 0;
		padding: 0 6px;
		height: 100%;
	}
	.trigger.mono,
	.list.mono {
		font-family: var(--font-mono);
		font-weight: 600;
	}
	.trigger.muted {
		color: var(--text-muted);
	}
	.label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.caret {
		display: inline-flex;
		color: var(--text-muted);
		transition: transform 0.15s;
	}
	.trigger.open .caret {
		transform: rotate(180deg);
	}
	/* The same popover as the menus: raised surface, hairline, soft shadow. */
	.list {
		position: fixed;
		z-index: 70;
		margin: 0;
		padding: 4px;
		list-style: none;
		overflow-y: auto;
		border-radius: 8px;
		background: var(--surface-raised);
		border: 1px solid var(--line-strong);
		box-shadow: var(--shadow-popover);
		font-size: var(--fs-md);
		color: var(--text);
		outline: none;
	}
	.list.above {
		transform: translateY(-100%);
	}
	li {
		display: flex;
		align-items: center;
		gap: 0.8em;
		padding: 0.4em 0.6em;
		border-radius: 5px;
		cursor: pointer;
		white-space: nowrap;
	}
	li.active {
		background: var(--pressed);
	}
	li[aria-selected="true"] {
		font-weight: 600;
	}
	.option-label {
		flex: 1;
	}
	.check {
		display: inline-flex;
		color: var(--accent-text);
	}
	@media (prefers-reduced-motion: reduce) {
		.caret {
			transition: none;
		}
	}
</style>
