<script lang="ts">
	import { activeResponses, markCancelling } from "../../stores/response";
	import { api } from "../../api/client";
	import { reportError } from "../../ui/notices";
	import { activeRequest } from "../../stores/activeRequest";
	import CodeEditor from "../CodeEditor.svelte";

	type Tab = "body" | "headers";
	let tab = $state<Tab>("body");

	function decodeBody(base64: string): string {
		try {
			const binary = atob(base64);
			const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
			return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
		} catch {
			return "(двоичное или нечитаемое тело)";
		}
	}

	/// Byte length of the response, recovered from the base64 payload without
	/// decoding it: 4 encoded chars per 3 bytes, minus the padding.
	function byteLength(base64: string): number {
		if (!base64) return 0;
		const padding = base64.endsWith("==") ? 2 : base64.endsWith("=") ? 1 : 0;
		return Math.max(0, (base64.length / 4) * 3 - padding);
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} Б`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} КБ`;
		return `${(bytes / (1024 * 1024)).toFixed(2)} МБ`;
	}

	function formatDuration(ms: number): string {
		if (ms < 1000) return `${ms} мс`;
		if (ms < 60_000) return `${(ms / 1000).toFixed(2)} с`;
		const minutes = Math.floor(ms / 60_000);
		const seconds = Math.round((ms % 60_000) / 1000);
		return `${minutes} мин ${seconds} с`;
	}

	/// Live counter while the request is in flight. Kept at one decimal: at a
	/// 100 ms tick, millisecond precision would just flicker.
	function formatElapsed(ms: number): string {
		if (ms < 60_000) return `${(ms / 1000).toFixed(1)} с`;
		const minutes = Math.floor(ms / 60_000);
		const seconds = Math.floor((ms % 60_000) / 1000);
		return `${minutes} мин ${String(seconds).padStart(2, "0")} с`;
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

	function statusClass(status: number): string {
		if (status < 300) return "status-ok";
		if (status < 400) return "status-redirect";
		if (status < 500) return "status-client-error";
		return "status-server-error";
	}

	// Ticks only while a request is in flight, so an idle app isn't running a
	// timer; the interval is torn down as soon as the response lands or the
	// user switches to another request.
	let now = $state(Date.now());
	$effect(() => {
		if (!$activeResponses.loading) return;
		now = Date.now();
		const ticker = setInterval(() => (now = Date.now()), 100);
		return () => clearInterval(ticker);
	});
	let elapsed = $derived(
		$activeResponses.startedAt != null ? Math.max(0, now - $activeResponses.startedAt) : 0,
	);

	/// Stops the in-flight send. The request stays "loading" until the
	/// backend comes back with the cancellation — the connection is torn down
	/// there, and reporting it as finished any earlier would let a second
	/// send start while the first is still unwinding.
	async function cancel() {
		const sendId = $activeResponses.sendId;
		if (!sendId) return;
		markCancelling($activeRequest!.path);
		try {
			await api.cancelSend(sendId);
		} catch (e) {
			reportError("Не удалось отменить запрос", e);
		}
	}

	// Only the newest record is kept today (see HISTORY_LIMIT), but reading
	// it as "the first of a list" keeps the viewer ready for real history.
	let latest = $derived($activeResponses.history[0] ?? null);
	let bodyText = $derived(latest?.outcome ? decodeBody(latest.outcome.body_base64) : "");
	let prettyBody = $derived(bodyText ? prettyPrint(bodyText) : "");
	let bodyIsJson = $derived(bodyText ? isJson(bodyText) : false);
</script>

<div class="response-viewer">
	{#if !$activeRequest}
		<p class="hint">Выберите запрос.</p>
	{:else if $activeResponses.loading}
		<div class="loading-state">
			<div class="loading-row">
				<span class="spinner" aria-hidden="true"></span>
				<span class="hint">Отправка...</span>
			</div>
			<span class="elapsed" aria-live="off">{formatElapsed(elapsed)}</span>
			<button class="cancel" onclick={cancel} disabled={$activeResponses.cancelling}>
				{$activeResponses.cancelling ? "Отмена..." : "Отменить"}
			</button>
		</div>
	{:else if latest?.error}
		<div class="status-bar">
			{#if latest.cancelled}
				<span class="status status-cancelled">Отменён</span>
			{:else}
				<span class="status status-server-error">Ошибка</span>
			{/if}
			{#if latest.elapsedMs != null}<span class="meta" title="Продолжительность запроса">{formatDuration(latest.elapsedMs)}</span>{/if}
			<span class="meta time" title="Когда был отправлен запрос">{new Date(latest.at).toLocaleTimeString()}</span>
		</div>
		<p class:error={!latest.cancelled} class:hint={latest.cancelled}>{latest.error}</p>
	{:else if latest?.outcome}
		{@const outcome = latest.outcome}
		<div class="status-bar">
			<span class="status {statusClass(outcome.status)}">{outcome.status} {outcome.status_text}</span>
			<span class="meta" title="Продолжительность запроса">{formatDuration(outcome.trace.total_ms)}</span>
			<span class="meta" title="Размер тела ответа">{formatSize(byteLength(outcome.body_base64))}</span>
			<span class="meta time" title="Когда был отправлен запрос">{new Date(latest.at).toLocaleTimeString()}</span>
		</div>

		{#if outcome.unresolved_variables.length > 0}
			<p class="warning">
				Не найдено значение для: {outcome.unresolved_variables.map((v) => `{{${v}}}`).join(", ")} — проверьте активное
				окружение.
			</p>
		{/if}

		<div class="tabs">
			<button class:active={tab === "body"} onclick={() => (tab = "body")}>Тело</button>
			<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
				>Заголовки&nbsp;({outcome.headers.length})</button
			>
		</div>

		{#if tab === "body"}
			<div class="body">
				<CodeEditor value={prettyBody} language={bodyIsJson ? "json" : "text"} readOnly />
			</div>
		{:else}
			<div class="headers">
				<table>
					<thead>
						<tr>
							<th>Заголовок</th>
							<th>Значение</th>
						</tr>
					</thead>
					<tbody>
						{#each outcome.headers as header (header.key)}
							<tr>
								<td class="header-key">{header.key}</td>
								<td class="header-value">{header.value}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{:else}
		<div class="hint-outer">
			<p class="hint">Запрос еще не был отправлен</p>
		</div>
	{/if}
</div>

<style>
	.response-viewer {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		height: 100%;
		min-height: 0;
	}
	.hint-outer {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
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
		gap: 0.9em;
	}
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.6em;
		height: 100%;
	}
	.cancel {
		background: none;
		border: 1px solid rgba(127, 127, 127, 0.45);
		color: inherit;
		border-radius: 6px;
		padding: 0.3em 0.9em;
		font-size: 0.85em;
		cursor: pointer;
	}
	.cancel:hover:not(:disabled) {
		border-color: #d1443c;
		color: #d1443c;
	}
	.cancel:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.status-cancelled {
		opacity: 0.6;
	}
	.loading-row {
		display: flex;
		align-items: baseline;
		gap: 0.6em;
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
	.meta {
		opacity: 0.6;
		font-size: 0.9em;
	}
	.elapsed {
		font-family: ui-monospace, monospace;
		/* Fixed-width digits so the counter doesn't jiggle as it ticks. */
		font-variant-numeric: tabular-nums;
		opacity: 0.75;
	}
	.spinner {
		width: 0.85em;
		height: 0.85em;
		border: 2px solid rgba(127, 127, 127, 0.35);
		border-top-color: #396cd8;
		border-radius: 50%;
		align-self: center;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation-duration: 2.5s;
		}
	}
	.meta.time {
		margin-left: auto;
	}
	.tabs {
		display: flex;
		gap: 0.2em;
		border-bottom: 1px solid rgba(127, 127, 127, 0.25);
	}
	.tabs button {
		background: none;
		border: none;
		padding: 0.35em 0.8em;
		cursor: pointer;
		color: inherit;
		opacity: 0.6;
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		opacity: 1;
		border-bottom-color: #396cd8;
	}
	.body {
		flex: 1;
		min-height: 8em;
	}
	.headers {
		flex: 1;
		min-height: 0;
		overflow: auto;
		border: 1px solid rgba(127, 127, 127, 0.3);
		border-radius: 6px;
	}
	.headers table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85em;
	}
	.headers th {
		position: sticky;
		top: 0;
		text-align: left;
		font-weight: 600;
		padding: 0.45em 0.6em;
		background: var(--modal-bg, #f6f8fa);
		border-bottom: 1px solid rgba(127, 127, 127, 0.35);
	}
	.headers td {
		padding: 0.4em 0.6em;
		border-bottom: 1px solid rgba(127, 127, 127, 0.18);
		vertical-align: top;
	}
	.headers tr:last-child td {
		border-bottom: none;
	}
	.header-key {
		font-weight: 600;
		white-space: nowrap;
		font-family: ui-monospace, monospace;
	}
	.header-value {
		font-family: ui-monospace, monospace;
		word-break: break-all;
	}
</style>
