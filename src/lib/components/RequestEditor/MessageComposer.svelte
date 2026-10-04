<script lang="ts">
	// The message a WebSocket request sends. It is part of the request and
	// saved with it, and goes out over the open socket as many times as the
	// button is pressed - what crossed the socket is the log in the response
	// pane, not this editor.
	import type { EditorLanguage, TextFormat, WebSocketSpec } from "../../bindings/types";
	import { t } from "../../i18n";
	import { activeResponses } from "../../stores/response";
	import { sendSocketMessage } from "../../ui/sending";
	import { MOD_KEY } from "../../ui/keys";
	import CodeEditor from "../CodeEditor.svelte";
	import Select from "../common/Select.svelte";

	let { spec, onChange }: { spec: WebSocketSpec; onChange: (patch: Partial<WebSocketSpec>) => void } = $props();

	/// Only the formats a socket API plausibly speaks. The format picks the
	/// highlighting and nothing else - there is no Content-Type to set.
	let formats = $derived<{ value: TextFormat; label: string; placeholder: string }[]>([
		{ value: "json", label: "JSON", placeholder: '{"type": "subscribe", "channel": "{{channel}}"}' },
		{ value: "xml", label: "XML", placeholder: "<subscribe>{{channel}}</subscribe>" },
		{ value: "yaml", label: "YAML", placeholder: "type: subscribe" },
		{ value: "edn", label: "EDN", placeholder: '{:type "subscribe"}' },
		{ value: "plain", label: $t("body.format.plain"), placeholder: $t("body.format.plainPlaceholder") },
	]);

	let language = $derived<EditorLanguage>(spec.format === "plain" ? "text" : spec.format);
	let placeholder = $derived(formats.find((f) => f.value === spec.format)?.placeholder ?? "");

	let live = $derived($activeResponses.live?.protocol === "websocket" ? $activeResponses.live : null);
	let open = $derived(live?.status === 101 && !$activeResponses.cancelling);
	let state = $derived(open ? "" : live ? $t("socket.composerConnecting") : $t("socket.composerClosed"));
</script>

<div class="composer">
	<div class="toolbar">
		<Select
			value={spec.format}
			ariaLabel={$t("socket.formatAria")}
			options={formats.map((f) => ({ value: f.value, label: f.label }))}
			onChange={(value) => onChange({ format: value as TextFormat })}
		/>
		{#if state}<span class="state">{state}</span>{/if}
		<button class="send" title="{MOD_KEY}+Enter" disabled={!open} onclick={sendSocketMessage}>
			{$t("socket.send")}<kbd>{MOD_KEY} ↵</kbd>
		</button>
	</div>
	<div class="editor-container">
		<CodeEditor value={spec.message} {language} {placeholder} onChange={(message) => onChange({ message })} />
	</div>
</div>

<style>
	.composer {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		height: 100%;
		min-height: 0;
	}
	.toolbar {
		display: flex;
		align-items: center;
		gap: 0.6em;
	}
	.state {
		font-size: var(--fs-sm);
		color: var(--text-muted);
	}
	.send {
		margin-left: auto;
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--accent);
		color: var(--accent-ink);
		border: 1px solid var(--accent);
		font-weight: 600;
		white-space: nowrap;
	}
	.send kbd {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		font-weight: 500;
		padding: 0 4px;
		border-radius: 4px;
		background: color-mix(in srgb, currentColor 12%, transparent);
	}
	.send:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.editor-container {
		flex: 1;
		min-height: 10em;
	}
</style>
