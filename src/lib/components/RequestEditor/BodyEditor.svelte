<script lang="ts">
	import type { BodySpec, EditorLanguage, TextFormat } from "../../bindings/types";
	import { api } from "../../api/client";
	import { reportError } from "../../ui/notices";
	import KeyValueTable from "./KeyValueTable.svelte";
	import CodeEditor from "../CodeEditor.svelte";

	let { body, onChange }: { body: BodySpec; onChange: (body: BodySpec) => void } = $props();

	// The value in the picker is the body kind for the structural cases, and
	// the text format for everything typed into the editor — from the user's
	// side "JSON" and "форма" are one choice, not two.
	type BodyChoice = "none" | TextFormat | "form" | "file";

	const TEXT_FORMATS: { value: TextFormat; label: string; placeholder: string }[] = [
		{ value: "json", label: "JSON", placeholder: '{"key": "{{value}}"}' },
		{ value: "xml", label: "XML", placeholder: "<root>{{value}}</root>" },
		{ value: "yaml", label: "YAML", placeholder: "key: {{value}}" },
		{ value: "edn", label: "EDN", placeholder: '{:key "{{value}}"}' },
		{ value: "html", label: "HTML", placeholder: "<p>{{value}}</p>" },
		{ value: "css", label: "CSS", placeholder: "body { color: #000 }" },
		{ value: "javascript", label: "JavaScript", placeholder: "console.log(1)" },
		{ value: "plain", label: "Текст", placeholder: "произвольный текст" },
	];

	/// Bodies written before formats existed keep their own tags; they read
	/// as the format they always were.
	let format = $derived<TextFormat>(
		body.type === "text" ? body.format : body.type === "json" ? "json" : "plain",
	);
	let isText = $derived(body.type === "text" || body.type === "json" || body.type === "raw");
	let choice = $derived<BodyChoice>(
		body.type === "none" || body.type === "form" || body.type === "file" ? body.type : format,
	);
	let language = $derived<EditorLanguage>(format === "plain" ? "text" : format);
	let placeholder = $derived(TEXT_FORMATS.find((f) => f.value === format)?.placeholder ?? "");
	let content = $derived(isText && body.type !== "form" && body.type !== "none" && body.type !== "file" ? body.content : "");

	function select(value: BodyChoice) {
		if (value === "none") return onChange({ type: "none" });
		if (value === "form") return onChange({ type: "form", fields: [] });
		if (value === "file") return onChange({ type: "file", path: "" });
		// Switching between text formats keeps what was typed: the payload is
		// the same text, only its declared type changed.
		onChange({ type: "text", content, format: value });
	}

	async function pickFile() {
		try {
			const path = await api.pickBodyFile();
			if (path) onChange({ type: "file", path });
		} catch (e) {
			reportError("Не удалось выбрать файл", e);
		}
	}
</script>

<div class="body-editor">
	<div class="toolbar">
		<select value={choice} onchange={(e) => select((e.target as HTMLSelectElement).value as BodyChoice)}>
			<option value="none">Без тела</option>
			{#each TEXT_FORMATS as item (item.value)}
				<option value={item.value}>{item.label}</option>
			{/each}
			<option value="form">Форма (urlencoded)</option>
			<option value="file">Файл с компьютера</option>
		</select>

		{#if body.type === "file"}
			<button onclick={pickFile}>Выбрать файл...</button>
		{/if}
	</div>

	{#if isText}
		<div class="editor-container">
			<CodeEditor
				value={content}
				{language}
				{placeholder}
				onChange={(text) => onChange({ type: "text", content: text, format })}
			/>
		</div>
	{:else if body.type === "form"}
		<KeyValueTable rows={body.fields} onChange={(fields) => onChange({ type: "form", fields })} />
	{:else if body.type === "file"}
		{#if body.path}
			<p class="file-path" title={body.path}>{body.path}</p>
			<p class="hint">
				Файл читается в момент отправки, а не сохраняется в запрос. Тип содержимого определяется по расширению, если
				заголовок Content-Type не задан вручную.
			</p>
		{:else}
			<p class="hint">Файл не выбран.</p>
		{/if}
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
	.toolbar {
		display: flex;
		align-items: center;
		gap: 0.5em;
	}
	.toolbar button {
		cursor: pointer;
		font-size: 0.85em;
	}
	.editor-container {
		flex: 1;
		min-height: 10em;
	}
	.file-path {
		margin: 0;
		font-family: ui-monospace, monospace;
		font-size: 0.85em;
		word-break: break-all;
	}
	.hint {
		margin: 0;
		font-size: 0.85em;
		opacity: 0.6;
	}
</style>
