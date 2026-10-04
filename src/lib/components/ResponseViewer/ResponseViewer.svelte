<script lang="ts">
	import { activeResponses } from "../../stores/response";
	import { api } from "../../api/client";
	import { locale, t } from "../../i18n";
	import { reportError } from "../../ui/notices";
	import { activeRequest } from "../../stores/activeRequest";
	import { cancelActiveSend } from "../../ui/sending";
	import { formatDuration, formatElapsed, formatSize } from "../../ui/format";
	import CodeEditor from "../CodeEditor.svelte";
	import HeadersTable from "./HeadersTable.svelte";
	import StreamViewer from "./StreamViewer.svelte";
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

	// Only the newest record is kept today (see HISTORY_LIMIT), but reading
	// it as "the first of a list" keeps the viewer ready for real history.
	let latest = $derived($activeResponses.history[0] ?? null);
	// A stream has a viewer of its own - while it is open, and afterwards
	// unless the server answered with an ordinary response instead, which
	// reads best the usual way.
	let liveStream = $derived($activeResponses.loading ? $activeResponses.live : null);
	let finishedStream = $derived(
		latest?.stream && latest.stream.end?.type !== "not_a_stream" ? latest.stream : null,
	);
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
	{:else if liveStream}
		<StreamViewer stream={liveStream} live startedAt={$activeResponses.startedAt} cancelling={$activeResponses.cancelling} />
	{:else if $activeResponses.loading}
		<div class="loading-state">
			<div class="loading-row">
				<span class="spinner" aria-hidden="true"></span>
				<span class="hint">{$t("response.sending")}</span>
			</div>
			<span class="elapsed" aria-live="off">{formatElapsed($t, elapsed)}</span>
			<button class="cancel" onclick={cancelActiveSend} disabled={$activeResponses.cancelling}>
				{$activeResponses.cancelling ? $t("response.cancelling") : $t("response.cancel")}
			</button>
		</div>
	{:else if finishedStream && latest}
		<StreamViewer stream={finishedStream} record={latest} />
	{:else if latest?.error}
		<div class="status-bar">
			{#if latest.cancelled}
				<span class="status status-cancelled">{$t("response.cancelled")}</span>
			{:else}
				<span class="status status-server-error">{$t("response.error")}</span>
			{/if}
			{#if latest.elapsedMs != null}<span class="meta" title={$t("response.duration")}
					>{formatDuration($t, latest.elapsedMs)}</span
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
			<span class="meta" title={$t("response.duration")}>{formatDuration($t, outcome.trace.total_ms)}</span>
			<span class="meta" title={$t("response.bodySize")}>{formatSize($t, byteLength(outcome.body_base64))}</span>
			{#if format}<span class="meta" title={$t("response.contentType")}>{format.mediaType}</span>{/if}
			<span class="meta time" title={$t("response.sentAt")}
				>{new Date(latest.at).toLocaleTimeString($locale)}</span
			>
			<button class="save-body" onclick={saveBody} disabled={saving} title={$t("response.saveBodyHint")}>
				{saving ? $t("common.saving") : $t("response.saveBodyButton")}
			</button>
		</div>

		{#if latest.stream}
			<p class="warning">{$t("stream.notAStream")}</p>
		{/if}
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
				<p class="hint">{$t("response.binaryHint", { size: formatSize($t, byteLength(outcome.body_base64)) })}</p>
				<button onclick={saveBody} disabled={saving}>{saving ? $t("common.saving") : $t("response.saveAsFile")}</button>
			</div>
		{:else if tab === "body"}
			<div class="body">
				<CodeEditor value={sourceText} language={format?.language ?? "text"} readOnly />
			</div>
		{:else}
			<HeadersTable headers={outcome.headers} />
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
		color: var(--text-muted);
	}
	.error {
		color: var(--danger);
		white-space: pre-wrap;
	}
	.warning {
		color: var(--warn);
		background: color-mix(in srgb, var(--warn) 10%, transparent);
		border-radius: 6px;
		padding: 0.4em 0.6em;
		font-size: var(--fs-sm);
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
		border: 1px solid var(--line-strong);
		color: inherit;
		border-radius: 6px;
		padding: 0.3em 0.9em;
		font-size: var(--fs-sm);
		cursor: pointer;
	}
	.cancel:hover:not(:disabled) {
		border-color: var(--danger);
		color: var(--danger);
	}
	.cancel:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.status-cancelled {
		color: var(--text-muted);
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
		color: var(--ok);
	}
	.status-redirect {
		color: var(--warn);
	}
	.status-client-error,
	.status-server-error {
		color: var(--danger);
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--fs-md);
	}
	.elapsed {
		font-family: var(--font-mono);
		/* Fixed-width digits so the counter doesn't jiggle as it ticks. */
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
	}
	.spinner {
		width: 0.85em;
		height: 0.85em;
		border: 2px solid var(--line-strong);
		border-top-color: var(--accent-text);
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
		border-bottom: 1px solid var(--line);
	}
	.tabs button {
		background: none;
		border: none;
		padding: 0.35em 0.8em;
		cursor: pointer;
		color: inherit;
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		color: var(--text);
		border-bottom-color: var(--accent-text);
	}
	.body {
		flex: 1;
		min-height: 8em;
	}
	.preview {
		flex: 1;
		min-height: 8em;
		border: 1px solid var(--line-strong);
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
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		/* Chequerboard, so transparency in the image is visible as such. */
		background-image: linear-gradient(45deg, var(--hover) 25%, transparent 25%),
			linear-gradient(-45deg, var(--hover) 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, var(--hover) 75%),
			linear-gradient(-45deg, transparent 75%, var(--hover) 75%);
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
		font-family: var(--font-mono);
		font-size: var(--fs-md);
	}
	.save-body {
		background: none;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		color: inherit;
		font-size: var(--fs-sm);
		padding: 0.15em 0.6em;
		cursor: pointer;
	}
	.save-body:hover:not(:disabled) {
		background: var(--hover);
	}
</style>
