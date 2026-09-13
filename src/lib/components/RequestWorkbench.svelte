<script lang="ts">
	// The request/response pair, split by a draggable divider. Extracted so
	// incognito mode reuses the exact same working area as a workspace does
	// — the two must not drift apart.
	import Splitter from "./common/Splitter.svelte";
	import RequestEditorTabs from "./RequestEditor/RequestEditorTabs.svelte";
	import ResponseViewer from "./ResponseViewer/ResponseViewer.svelte";
	import { t } from "../i18n";
	import { layout, updateLayout } from "../stores/layout";

	let panesHeight = $state(0);
</script>

<div class="panes" bind:clientHeight={panesHeight} style="grid-template-rows: {$layout.editorHeight}px auto 1fr">
	<section class="pane editor-pane">
		<RequestEditorTabs />
	</section>
	<Splitter
		direction="horizontal"
		value={$layout.editorHeight}
		min={140}
		max={Math.max(200, panesHeight - 160)}
		ariaLabel={$t("app.requestPaneHeight")}
		onResize={(v) => updateLayout({ editorHeight: v })}
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
</style>
