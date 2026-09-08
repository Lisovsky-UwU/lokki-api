<script lang="ts">
	import { untrack } from "svelte";
	import { Annotation, Compartment, EditorState } from "@codemirror/state";
	import {
		EditorView,
		drawSelection,
		dropCursor,
		highlightActiveLine,
		keymap,
		placeholder as placeholderExt,
	} from "@codemirror/view";
	import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
	import {
		autocompletion,
		closeBrackets,
		closeBracketsKeymap,
		completionKeymap,
		type CompletionContext,
		type CompletionResult,
	} from "@codemirror/autocomplete";
	import { json } from "@codemirror/lang-json";
	import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
	import { tags } from "@lezer/highlight";
	import { get } from "svelte/store";
	import { availableVariables } from "../stores/environments";

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

	// Colors come from CSS custom properties (GitHub's palette, defined per
	// theme in +page.svelte) so the editor follows light/dark automatically.
	const highlightStyle = HighlightStyle.define([
		{ tag: [tags.propertyName, tags.definition(tags.propertyName)], color: "var(--cm-property)" },
		{ tag: [tags.string, tags.special(tags.string)], color: "var(--cm-string)" },
		{ tag: [tags.number, tags.bool, tags.null, tags.atom], color: "var(--cm-number)" },
		{ tag: [tags.keyword, tags.operator], color: "var(--cm-keyword)" },
		{ tag: [tags.comment, tags.lineComment, tags.blockComment], color: "var(--cm-comment)", fontStyle: "italic" },
		{ tag: [tags.punctuation, tags.separator, tags.bracket], color: "var(--cm-punctuation)" },
		{ tag: tags.invalid, color: "#cf222e" },
	]);

	// The caret and selection are drawn by CodeMirror, so they need explicit
	// colors: the defaults are tuned for a light theme and were invisible
	// (white on white) here. Blinking is left to CodeMirror's own animation —
	// defining a competing one made the caret flicker erratically.
	const theme = EditorView.theme({
		"&": { fontSize: "0.85em", height: "100%", backgroundColor: "transparent" },
		".cm-content": { fontFamily: "ui-monospace, monospace", padding: "0.5em", caretColor: "currentColor" },
		".cm-scroller": { overflow: "auto" },
		"&.cm-focused": { outline: "none" },
		".cm-cursor, .cm-dropCursor": { borderLeftColor: "currentColor", borderLeftWidth: "2px" },
		".cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground": {
			background: "var(--cm-selection) !important",
		},
		"&.cm-focused .cm-selectionLayer .cm-selectionBackground": { background: "var(--cm-selection) !important" },
		".cm-content ::selection, .cm-line ::selection": { background: "var(--cm-selection) !important" },
		".cm-activeLine": { backgroundColor: "var(--cm-active-line)" },
		".cm-tooltip": {
			background: "var(--modal-bg, #fff)",
			border: "1px solid rgba(127,127,127,0.35)",
			borderRadius: "6px",
			color: "inherit",
		},
		".cm-tooltip-autocomplete ul li[aria-selected]": { background: "rgba(57, 108, 216, 0.35)", color: "inherit" },
	});

	const languageExtension = (lang: "json" | "text") => (lang === "json" ? [json()] : []);

	/// Completes environment variable names right after a `{{`.
	function variableCompletions(context: CompletionContext): CompletionResult | null {
		const match = context.matchBefore(/\{\{[A-Za-z0-9_.-]*/);
		if (!match || (match.from === match.to && !context.explicit)) return null;
		const typed = match.text.slice(2);
		return {
			from: match.from,
			options: get(availableVariables).map((v) => ({
				label: `{{${v.key}}}`,
				detail: v.secret ? "secret" : v.value,
				info: v.scope === "collection" ? "окружение коллекции" : "глобальное окружение",
				type: "variable",
				apply: `{{${v.key}}}`,
			})),
			filter: typed.length > 0,
			validFor: /^\{\{[A-Za-z0-9_.-]*\}?\}?$/,
		};
	}

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
							closeBrackets(),
							autocompletion({ override: [variableCompletions] }),
							keymap.of([
								...closeBracketsKeymap,
								...completionKeymap,
								...defaultKeymap,
								...historyKeymap,
								indentWithTab,
							]),
							syntaxHighlighting(highlightStyle),
							drawSelection(),
							dropCursor(),
							...(readOnly ? [] : [highlightActiveLine()]),
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
