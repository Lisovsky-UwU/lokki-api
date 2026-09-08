<script lang="ts">
	import type { BodySpec } from "../../bindings/types";
	import KeyValueTable from "./KeyValueTable.svelte";
	import CodeEditor from "../CodeEditor.svelte";

	let { body, onChange }: { body: BodySpec; onChange: (body: BodySpec) => void } = $props();

	function setType(type: BodySpec["type"]) {
		if (type === "none") onChange({ type: "none" });
		else if (type === "form") onChange({ type: "form", fields: [] });
		else onChange({ type, content: "" } as BodySpec);
	}
</script>

<div class="body-editor">
	<select value={body.type} onchange={(e) => setType((e.target as HTMLSelectElement).value as BodySpec["type"])}>
		<option value="none">No Body</option>
		<option value="json">JSON</option>
		<option value="raw">Raw</option>
		<option value="form">Form (urlencoded)</option>
	</select>

	{#if body.type === "json" || body.type === "raw"}
		<div class="editor-container">
			<CodeEditor
				value={body.content}
				language={body.type === "json" ? "json" : "text"}
				placeholder={body.type === "json" ? '{"key": "{{value}}"}' : "raw body"}
				onChange={(content) => onChange({ type: body.type, content })}
			/>
		</div>
	{:else if body.type === "form"}
		<KeyValueTable rows={body.fields} onChange={(fields) => onChange({ type: "form", fields })} />
	{/if}
</div>

<style>
	.body-editor {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		height: 100%;
		min-height: 0;
	}
	.editor-container {
		flex: 1;
		min-height: 10em;
	}
</style>
