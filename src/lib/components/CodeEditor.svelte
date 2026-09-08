<script lang="ts">
	import { untrack } from "svelte";
	import { EditorState } from "@codemirror/state";
	import { EditorView, keymap, placeholder as placeholderExt } from "@codemirror/view";
	import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
	import { json } from "@codemirror/lang-json";
	import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
	import { tags } from "@lezer/highlight";

	let {
		value = "",
		onChange,
		language = "json",
		readOnly = false,
		placeholder: placeholderText = "",
	}: {
		value: string;
		onChange?: (v: string) => void;
		language?: "json" | "text";
		readOnly?: boolean;
		placeholder?: string;
	} = $props();

	let container = $state<HTMLDivElement>();
	let view: EditorView | undefined;

	// Colors pull from CSS custom properties so the editor follows the
	// page's light/dark theme without pulling in a full CodeMirror theme
	// package.
	const highlightStyle = HighlightStyle.define([
		{ tag: tags.propertyName, color: "var(--cm-property, #a37c00)" },
		{ tag: tags.string, color: "var(--cm-string, #2e9e5b)" },
		{ tag: [tags.number, tags.bool, tags.null], color: "var(--cm-literal, #396cd8)" },
		{ tag: tags.punctuation, opacity: "0.6" },
	]);

	const theme = EditorView.theme({
		"&": { fontSize: "0.85em", height: "100%" },
		".cm-content": { fontFamily: "ui-monospace, monospace", padding: "0.5em" },
		".cm-scroller": { overflow: "auto" },
		"&.cm-focused": { outline: "none" },
	});

	function buildExtensions() {
		const exts = [
			history(),
			keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
			syntaxHighlighting(highlightStyle),
			theme,
			EditorView.lineWrapping,
			EditorView.updateListener.of((update) => {
				if (update.docChanged && onChange) onChange(update.state.doc.toString());
			}),
		];
		if (language === "json") exts.push(json());
		if (readOnly) exts.push(EditorState.readOnly.of(true));
		if (placeholderText) exts.push(placeholderExt(placeholderText));
		return exts;
	}

	$effect(() => {
		if (!container) return;
		const initialDoc = untrack(() => value);
		const editorView = new EditorView({
			state: EditorState.create({ doc: initialDoc, extensions: buildExtensions() }),
			parent: container,
		});
		view = editorView;
		return () => {
			editorView.destroy();
			view = undefined;
		};
	});

	// Sync external value changes (switching to a different request/response)
	// without fighting the user's cursor mid-edit — only dispatches when the
	// doc actually diverges from `value` (so our own onChange round trip is a
	// no-op here).
	$effect(() => {
		const current = value;
		if (view && current !== view.state.doc.toString()) {
			view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: current } });
		}
	});
</script>

<div class="code-editor" bind:this={container}></div>

<style>
	.code-editor {
		height: 100%;
		min-height: 8em;
		border: 1px solid rgba(127, 127, 127, 0.35);
		border-radius: 6px;
		overflow: hidden;
	}
	.code-editor :global(.cm-editor) {
		height: 100%;
	}
</style>
