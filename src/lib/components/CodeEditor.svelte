<script lang="ts">
	import { untrack } from "svelte";
	import { Annotation, Compartment, EditorState } from "@codemirror/state";
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

	// Marks doc changes pushed in from outside (opening a different request,
	// rendering a new response) so they don't echo back through `onChange` —
	// otherwise merely opening a request would mark it as edited.
	const External = Annotation.define<boolean>();

	// Swappable config, so language/readOnly changes reconfigure the running
	// editor instead of tearing it down and rebuilding it (which would drop
	// the user's cursor and undo history mid-edit).
	const languageConf = new Compartment();
	const readOnlyConf = new Compartment();

	// Colors come from CSS custom properties so the editor follows the page's
	// light/dark theme without pulling in a full CodeMirror theme package.
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

	const languageExtension = (lang: "json" | "text") => (lang === "json" ? [json()] : []);

	$effect(() => {
		if (!container) return;
		// Everything except the container is read untracked: this effect is
		// setup/teardown only and must not re-run when props change.
		const editorView = untrack(
			() =>
				new EditorView({
					state: EditorState.create({
						doc: value,
						extensions: [
							history(),
							keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
							syntaxHighlighting(highlightStyle),
							theme,
							EditorView.lineWrapping,
							languageConf.of(languageExtension(language)),
							readOnlyConf.of(EditorState.readOnly.of(readOnly)),
							...(placeholderText ? [placeholderExt(placeholderText)] : []),
							EditorView.updateListener.of((update) => {
								if (!update.docChanged) return;
								if (update.transactions.some((t) => t.annotation(External))) return;
								onChange?.(update.state.doc.toString());
							}),
						],
					}),
					parent: container,
				}),
		);
		view = editorView;
		return () => {
			editorView.destroy();
			view = undefined;
		};
	});

	$effect(() => {
		const lang = language;
		const ro = readOnly;
		untrack(() => {
			view?.dispatch({
				effects: [
					languageConf.reconfigure(languageExtension(lang)),
					readOnlyConf.reconfigure(EditorState.readOnly.of(ro)),
				],
			});
		});
	});

	// Push external `value` changes into the editor, but never when they
	// already match — that would fight the user's cursor as they type.
	$effect(() => {
		const current = value;
		untrack(() => {
			if (view && current !== view.state.doc.toString()) {
				view.dispatch({
					changes: { from: 0, to: view.state.doc.length, insert: current },
					annotations: External.of(true),
				});
			}
		});
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
