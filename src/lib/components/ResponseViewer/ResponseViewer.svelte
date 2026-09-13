<script lang="ts">
	import { activeResponses, markCancelling } from "../../stores/response";
	import { api } from "../../api/client";
	import { locale, t } from "../../i18n";
	import { reportError } from "../../ui/notices";
	import { activeRequest } from "../../stores/activeRequest";
	import CodeEditor from "../CodeEditor.svelte";
	import { detectFormat, isTextualMediaType, mediaTypeOf } from "../../ui/contentType";

	// Tabs depend on what came back: a page gets a rendered view, a picture
	// gets shown, a file only makes sense saved.
	let tab = $state("body");
	let saving = $state(false);

	function decodeBody(base64: string): string {
		try {
			const binary = atob(base64);
			const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
			return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
		} catch {
			return $t("response.binaryBody");
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
		if (bytes < 1024) return $t("unit.bytes", { value: bytes });
		if (bytes < 1024 * 1024) return $t("unit.kilobytes", { value: (bytes / 1024).toFixed(1) });
		return $t("unit.megabytes", { value: (bytes / (1024 * 1024)).toFixed(2) });
	}

	function formatDuration(ms: number): string {
		if (ms < 1000) return $t("unit.milliseconds", { value: ms });
		if (ms < 60_000) return $t("unit.seconds", { value: (ms / 1000).toFixed(2) });
		const minutes = Math.floor(ms / 60_000);
		const seconds = Math.round((ms % 60_000) / 1000);
		return $t("unit.minutesSeconds", { minutes, seconds });
	}

	/// Live counter while the request is in flight. Kept at one decimal: at a
	/// 100 ms tick, millisecond precision would just flicker.
	function formatElapsed(ms: number): string {
		if (ms < 60_000) return $t("unit.seconds", { value: (ms / 1000).toFixed(1) });
		const minutes = Math.floor(ms / 60_000);
		const seconds = Math.floor((ms % 60_000) / 1000);
		return $t("unit.minutesSeconds", { minutes, seconds: String(seconds).padStart(2, "0") });
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
	/// backend comes back with the cancellation - the connection is torn down
	/// there, and reporting it as finished any earlier would let a second
	/// send start while the first is still unwinding.
	async function cancel() {
		const sendId = $activeResponses.sendId;
		if (!sendId) return;
		markCancelling($activeRequest!.path);
		try {
			await api.cancelSend(sendId);
		} catch (e) {
			reportError($t("response.cancelFailed"), e);
		}
	}

	// Only the newest record is kept today (see HISTORY_LIMIT), but reading
	// it as "the first of a list" keeps the viewer ready for real history.
	let latest = $derived($activeResponses.history[0] ?? null);
	// Decoding is skipped for bodies that were never text - see
	// isTextualMediaType; the file view only needs the size and the type.
	let bodyText = $derived(
		latest?.outcome && isTextualMediaType(mediaTypeOf(latest.outcome.headers))
			? decodeBody(latest.outcome.body_base64)
			: "",
	);
	let format = $derived(latest?.outcome ? detectFormat(latest.outcome.headers, bodyText) : null);
	// Only JSON is reformatted: reflowing XML or HTML would change what the
	// server actually sent, which is the thing being inspected.
	let sourceText = $derived(format?.language === "json" && isJson(bodyText) ? prettyPrint(bodyText) : bodyText);
	let dataUrl = $derived(
		latest?.outcome && format ? `data:${format.mediaType};base64,${latest.outcome.body_base64}` : "",
	);

	let tabs = $derived.by(() => {
		const outcome = latest?.outcome;
		if (!outcome || !format) return [] as { id: string; label: string }[];
		const list: { id: string; label: string }[] = [];
		if (format.kind === "html")
			list.push({ id: "preview", label: $t("response.tab.preview") }, { id: "body", label: $t("response.tab.source") });
		else if (format.kind === "image") list.push({ id: "image", label: $t("response.tab.image") });
		else if (format.kind === "binary") list.push({ id: "file", label: $t("response.tab.file") });
		else list.push({ id: "body", label: $t("response.tab.body") });
		// SVG is markup as well as a picture, so its source stays reachable.
		if (format.mediaType === "image/svg+xml") list.push({ id: "body", label: $t("response.tab.source") });
		list.push({ id: "headers", label: $t("response.tab.headers", { count: outcome.headers.length }) });
		return list;
	});

	// A new response can leave the selected tab pointing at something this
	// one doesn't have (source view of a picture, say).
	$effect(() => {
		const ids = tabs.map((t) => t.id);
		if (ids.length > 0 && !ids.includes(tab)) tab = ids[0];
	});

	/// Writes the body to disk as it arrived - decoding happens in the core,
	/// which is the only side that can write files anyway.
	async function saveBody() {
		const outcome = latest?.outcome;
		if (!outcome || !format) return;
		saving = true;
		try {
			const target = await api.pickDownloadTarget(format.fileName);
			if (target) await api.saveResponseBody(target, outcome.body_base64);
		} catch (e) {
			reportError($t("response.saveFailed"), e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="response-viewer">
	{#if !$activeRequest}
		<p class="hint">{$t("response.pickRequest")}</p>
	{:else if $activeResponses.loading}
		<div class="loading-state">
			<div class="loading-row">
				<span class="spinner" aria-hidden="true"></span>
				<span class="hint">{$t("response.sending")}</span>
			</div>
			<span class="elapsed" aria-live="off">{formatElapsed(elapsed)}</span>
			<button class="cancel" onclick={cancel} disabled={$activeResponses.cancelling}>
				{$activeResponses.cancelling ? $t("response.cancelling") : $t("response.cancel")}
			</button>
		</div>
	{:else if latest?.error}
		<div class="status-bar">
			{#if latest.cancelled}
				<span class="status status-cancelled">{$t("response.cancelled")}</span>
			{:else}
				<span class="status status-server-error">{$t("response.error")}</span>
			{/if}
			{#if latest.elapsedMs != null}<span class="meta" title={$t("response.duration")}
					>{formatDuration(latest.elapsedMs)}</span
				>{/if}
			<span class="meta time" title={$t("response.sentAt")}
				>{new Date(latest.at).toLocaleTimeString($locale)}</span
			>
		</div>
		<p class:error={!latest.cancelled} class:hint={latest.cancelled}>{latest.error}</p>
	{:else if latest?.outcome}
		{@const outcome = latest.outcome}
		<div class="status-bar">
			<span class="status {statusClass(outcome.status)}">{outcome.status} {outcome.status_text}</span>
			<span class="meta" title={$t("response.duration")}>{formatDuration(outcome.trace.total_ms)}</span>
			<span class="meta" title={$t("response.bodySize")}>{formatSize(byteLength(outcome.body_base64))}</span>
			{#if format}<span class="meta" title={$t("response.contentType")}>{format.mediaType}</span>{/if}
			<span class="meta time" title={$t("response.sentAt")}
				>{new Date(latest.at).toLocaleTimeString($locale)}</span
			>
			<button class="save-body" onclick={saveBody} disabled={saving} title={$t("response.saveBody")}>
				{saving ? $t("common.saving") : $t("common.save")}
			</button>
		</div>

		{#if outcome.unresolved_variables.length > 0}
			<p class="warning">
				{$t("response.unresolved", { names: outcome.unresolved_variables.map((v) => `{{${v}}}`).join(", ") })}
			</p>
		{/if}

		<div class="tabs">
			{#each tabs as item (item.id + item.label)}
				<button class:active={tab === item.id} onclick={() => (tab = item.id)}>{item.label}</button>
			{/each}
		</div>

		{#if tab === "preview"}
			<!-- Sandboxed with nothing allowed: a response is untrusted content,
			     and a preview has no business running its scripts. -->
			<iframe class="preview" title={$t("response.previewTitle")} sandbox="" srcdoc={bodyText}></iframe>
		{:else if tab === "image"}
			<div class="image-view">
				<img src={dataUrl} alt={$t("response.imageAlt")} />
			</div>
		{:else if tab === "file"}
			<div class="file-view">
				<p class="file-line">{format?.mediaType}</p>
				<p class="hint">{$t("response.binaryHint", { size: formatSize(byteLength(outcome.body_base64)) })}</p>
				<button onclick={saveBody} disabled={saving}>{saving ? $t("common.saving") : $t("response.saveAsFile")}</button>
			</div>
		{:else if tab === "body"}
			<div class="body">
				<CodeEditor value={sourceText} language={format?.language ?? "text"} readOnly />
			</div>
		{:else}
			<div class="headers">
				<table>
					<thead>
						<tr>
							<th>{$t("response.headerName")}</th>
							<th>{$t("response.headerValue")}</th>
						</tr>
					</thead>
					<tbody>
						<!-- Keyed by position, not by name: HTTP allows a header to
						     repeat (Link, Set-Cookie), and each occurrence is its own
						     row. -->
						{#each outcome.headers as header, i (i)}
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
			<p class="hint">{$t("response.notSent")}</p>
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
	.hint {
		margin: 0;
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
	.preview {
		flex: 1;
		min-height: 8em;
		border: 1px solid rgba(127, 127, 127, 0.3);
		border-radius: 6px;
		background: white;
	}
	.image-view {
		flex: 1;
		min-height: 0;
		overflow: auto;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid rgba(127, 127, 127, 0.3);
		border-radius: 6px;
		/* Chequerboard, so transparency in the image is visible as such. */
		background-image: linear-gradient(45deg, rgba(127, 127, 127, 0.15) 25%, transparent 25%),
			linear-gradient(-45deg, rgba(127, 127, 127, 0.15) 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, rgba(127, 127, 127, 0.15) 75%),
			linear-gradient(-45deg, transparent 75%, rgba(127, 127, 127, 0.15) 75%);
		background-size: 16px 16px;
		background-position:
			0 0,
			0 8px,
			8px -8px,
			-8px 0;
	}
	.image-view img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.file-view {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5em;
	}
	.file-view button {
		cursor: pointer;
	}
	.file-line {
		margin: 0;
		font-family: ui-monospace, monospace;
		font-size: 0.9em;
	}
	.save-body {
		background: none;
		border: 1px solid rgba(127, 127, 127, 0.4);
		border-radius: 6px;
		color: inherit;
		font-size: 0.8em;
		padding: 0.15em 0.6em;
		cursor: pointer;
	}
	.save-body:hover:not(:disabled) {
		background: rgba(127, 127, 127, 0.15);
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
