<script lang="ts">
	import { responseState } from "../../stores/response";
	import CodeEditor from "../CodeEditor.svelte";

	function decodeBody(base64: string): string {
		try {
			const binary = atob(base64);
			const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
			return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
		} catch {
			return "(binary or unreadable body)";
		}
	}

	function isJson(text: string): boolean {
		try {
			JSON.parse(text);
			return true;
		} catch {
			return false;
		}
	}

	function prettyPrint(text: string): string {
		try {
			return JSON.stringify(JSON.parse(text), null, 2);
		} catch {
			return text;
		}
	}

	let bodyText = $derived($responseState.outcome ? decodeBody($responseState.outcome.body_base64) : "");
	let prettyBody = $derived(bodyText ? prettyPrint(bodyText) : "");
	let bodyIsJson = $derived(bodyText ? isJson(bodyText) : false);

	function statusClass(status: number): string {
		if (status < 300) return "status-ok";
		if (status < 400) return "status-redirect";
		if (status < 500) return "status-client-error";
		return "status-server-error";
	}
</script>

<div class="response-viewer">
	{#if $responseState.loading}
		<p class="hint">Отправка...</p>
	{:else if $responseState.error}
		<p class="error">{$responseState.error}</p>
	{:else if $responseState.outcome}
		{@const outcome = $responseState.outcome}
		<div class="status-bar">
			<span class="status {statusClass(outcome.status)}">{outcome.status} {outcome.status_text}</span>
			<span class="timing">{outcome.duration_ms} ms</span>
		</div>
		{#if outcome.unresolved_variables.length > 0}
			<p class="warning">
				Не найдено значение для: {outcome.unresolved_variables.map((v) => `{{${v}}}`).join(", ")} — проверьте активное
				окружение.
			</p>
		{/if}
		<details class="headers" open={false}>
			<summary>Headers ({outcome.headers.length})</summary>
			<table>
				<tbody>
					{#each outcome.headers as header}
						<tr>
							<td class="header-key">{header.key}</td>
							<td>{header.value}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</details>
		<div class="body">
			<CodeEditor value={prettyBody} language={bodyIsJson ? "json" : "text"} readOnly />
		</div>
	{:else}
		<p class="hint">Отправьте запрос, чтобы увидеть ответ.</p>
	{/if}
</div>

<style>
	.response-viewer {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		height: 100%;
		overflow-y: auto;
	}
	.hint {
		opacity: 0.6;
	}
	.error {
		color: #d1443c;
		white-space: pre-wrap;
	}
	.warning {
		color: #a37c00;
		background: rgba(163, 124, 0, 0.1);
		border-radius: 6px;
		padding: 0.4em 0.6em;
		font-size: 0.85em;
		margin: 0;
	}
	.status-bar {
		display: flex;
		align-items: baseline;
		gap: 0.8em;
	}
	.status {
		font-weight: 700;
	}
	.status-ok {
		color: #2e9e5b;
	}
	.status-redirect {
		color: #a37c00;
	}
	.status-client-error,
	.status-server-error {
		color: #d1443c;
	}
	.timing {
		opacity: 0.6;
		font-size: 0.9em;
	}
	.headers table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85em;
	}
	.header-key {
		font-weight: 600;
		padding-right: 1em;
		white-space: nowrap;
		vertical-align: top;
	}
	.body {
		flex: 1;
		min-height: 8em;
	}
</style>
