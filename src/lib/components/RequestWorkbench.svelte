<script lang="ts">
	// The request/response pair, split by a draggable divider. Extracted so
	// incognito mode reuses the exact same working area as a workspace does
	// - the two must not drift apart.
	import Splitter from "./common/Splitter.svelte";
	import RequestHeader from "./RequestEditor/RequestHeader.svelte";
	import RequestEditorTabs from "./RequestEditor/RequestEditorTabs.svelte";
	import ResponseViewer from "./ResponseViewer/ResponseViewer.svelte";
	import { t } from "../i18n";
	import { layout, updateLayout } from "../stores/layout";

	let panesHeight = $state(0);
	let panesWidth = $state(0);
	let headerHeight = $state(0);

	let horizontal = $derived($layout.orientation === "horizontal");

	// One grid either way: the orientation only decides which axis the three
	// tracks (pane, splitter, pane) are laid on, so the markup below stays a
	// single copy rather than two branches that can drift apart.
	//
	// The header sits above them in a row of its own, spanning every column.
	// That is the whole reason it is not part of the editor pane: side by
	// side, that pane is half a window wide, and an address bar wants the
	// whole one.
	let tracks = $derived(
		horizontal
			? `grid-template-columns: ${$layout.editorWidth}px auto 1fr; grid-template-rows: auto 1fr`
			: `grid-template-rows: auto ${$layout.editorHeight}px auto 1fr`,
	);

	let splitter = $derived(
		horizontal
			? {
					direction: "vertical" as const,
					value: $layout.editorWidth,
					max: Math.max(320, panesWidth - 260),
					label: $t("app.requestPaneWidth"),
					apply: (v: number) => updateLayout({ editorWidth: v }),
				}
			: {
					direction: "horizontal" as const,
					value: $layout.editorHeight,
					// The header is above the split and keeps its height
					// whatever the divider does, so the room left to share is
					// what remains under it.
					max: Math.max(200, panesHeight - headerHeight - 160),
					label: $t("app.requestPaneHeight"),
					apply: (v: number) => updateLayout({ editorHeight: v }),
				},
	);
</script>

<div class="panes" class:horizontal bind:clientHeight={panesHeight} bind:clientWidth={panesWidth} style={tracks}>
	<div class="header" bind:clientHeight={headerHeight}>
		<RequestHeader />
	</div>
	<section class="pane editor-pane">
		<RequestEditorTabs />
	</section>
	<Splitter
		direction={splitter.direction}
		value={splitter.value}
		min={140}
		max={splitter.max}
		ariaLabel={splitter.label}
		onResize={splitter.apply}
	/>
	<section class="pane response-pane">
		<ResponseViewer />
	</section>
</div>

<style>
	.panes {
		flex: 1;
		display: grid;
		min-height: 0;
		padding: 0.8em;
	}
	/* Full width in both layouts: with one column that is simply the column,
	   with three it is all of them. */
	.header {
		grid-column: 1 / -1;
		min-width: 0;
		padding-bottom: 0.6em;
	}
	.pane {
		min-height: 0;
		overflow: hidden;
	}
	.editor-pane {
		display: flex;
	}
	.response-pane {
		border-top: 1px solid rgba(127, 127, 127, 0.2);
		padding-top: 0.6em;
	}
	/* Side by side: the panes need a width floor of their own (a grid item
	   defaults to min-content, which a long URL would push wide), and the
	   rule between them moves from the top of the response to its left. */
	.panes.horizontal,
	.panes.horizontal .pane {
		min-width: 0;
	}
	.panes.horizontal .response-pane {
		border-top: none;
		padding-top: 0;
		border-left: 1px solid rgba(127, 127, 127, 0.2);
		padding-left: 0.6em;
	}
</style>
