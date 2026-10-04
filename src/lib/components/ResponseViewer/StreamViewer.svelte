<script lang="ts">
	// A Server-Sent Events stream, while it is open and after it has ended:
	// what state it is in, then the events as they arrived.
	import { tick } from "svelte";
	import { SvelteSet } from "svelte/reactivity";
	import { locale, t } from "../../i18n";
	import { STREAM_ENTRY_LIMIT, type ResponseRecord, type StreamRecord } from "../../stores/response";
	import type { SseComment, SseEvent } from "../../bindings/types";
	import { cancelActiveSend } from "../../ui/sending";
	import { copyText } from "../../ui/clipboard";
	import { formatDuration, formatElapsed } from "../../ui/format";
	import HeadersTable from "./HeadersTable.svelte";

	let {
		stream,
		live = false,
		startedAt = null,
		cancelling = false,
		record = null,
	}: {
		stream: StreamRecord;
		/// Still receiving; `startedAt` and `cancelling` only matter then.
		live?: boolean;
		startedAt?: number | null;
		cancelling?: boolean;
		/// The finished send, once there is one.
		record?: ResponseRecord | null;
	} = $props();

	type Entry = SseEvent | SseComment;

	let tab = $state<"events" | "headers">("events");
	let filter = $state("");
	// Keyed by the entry itself rather than its position: once the cap starts
	// letting old entries go, every position shifts.
	const expanded = new SvelteSet<Entry>();
	let list = $state<HTMLDivElement>();
	// Follow new events only while the user is at the bottom - scrolling up
	// to read something must not be undone by the next event.
	let following = $state(true);

	let now = $state(Date.now());
	$effect(() => {
		if (!live) return;
		now = Date.now();
		const ticker = setInterval(() => (now = Date.now()), 100);
		return () => clearInterval(ticker);
	});

	let eventCount = $derived(stream.entries.reduce((n, entry) => n + (entry.kind === "event" ? 1 : 0), 0));
	let visible = $derived.by(() => {
		const needle = filter.trim().toLowerCase();
		if (!needle) return stream.entries;
		return stream.entries.filter((entry) =>
			entry.kind === "event"
				? entry.event.toLowerCase().includes(needle) ||
					entry.data.toLowerCase().includes(needle) ||
					(entry.id ?? "").toLowerCase().includes(needle)
				: entry.text.toLowerCase().includes(needle),
		);
	});

	$effect(() => {
		void visible.length;
		if (!following || !list || tab !== "events") return;
		tick().then(() => list?.scrollTo({ top: list.scrollHeight }));
	});

	function onScroll() {
		if (!list) return;
		following = list.scrollHeight - list.scrollTop - list.clientHeight < 24;
	}

	type Phase = { label: string; tone: "pending" | "open" | "ended" | "failed" };
	let phase = $derived.by((): Phase => {
		if (live) {
			if (cancelling) return { label: $t("stream.disconnecting"), tone: "pending" };
			if (stream.status == null) return { label: $t("stream.connecting"), tone: "pending" };
			return { label: $t("stream.open"), tone: "open" };
		}
		switch (stream.end?.type) {
			case "cancelled":
				return { label: $t("stream.disconnected"), tone: "ended" };
			case "failed":
				return { label: $t("stream.failed"), tone: "failed" };
			default:
				return { label: $t("stream.closed"), tone: "ended" };
		}
	});

	function statusClass(status: number): string {
		if (status < 300) return "status-ok";
		if (status < 400) return "status-redirect";
		return "status-error";
	}

	function offset(ms: number): string {
		return $t("unit.seconds", { value: (ms / 1000).toFixed(3) });
	}

	/// Pretty-printed when the data is JSON, which most streams send; any
	/// other text is shown exactly as it arrived.
	function readable(data: string): string {
		try {
			return JSON.stringify(JSON.parse(data), null, 2);
		} catch {
			return data;
		}
	}

	function toggle(entry: Entry) {
		if (expanded.has(entry)) expanded.delete(entry);
		else expanded.add(entry);
	}
</script>

<div class="stream-viewer">
	<div class="status-bar">
		<span class="state {phase.tone}">
			{#if phase.tone === "pending"}<span class="spinner" aria-hidden="true"></span>{/if}
			{#if phase.tone === "open"}<span class="pulse" aria-hidden="true"></span>{/if}
			{phase.label}
		</span>
		{#if stream.status != null}
			<span class="status {statusClass(stream.status)}">{stream.status} {stream.status_text}</span>
		{/if}
		{#if live && startedAt != null}
			<span class="meta elapsed" title={$t("stream.duration")}>{formatElapsed($t, Math.max(0, now - startedAt))}</span>
		{:else if record?.elapsedMs != null}
			<span class="meta" title={$t("stream.duration")}>{formatDuration($t, record.elapsedMs)}</span>
		{/if}
		<span class="meta">{$t("stream.eventCount", { count: eventCount + stream.dropped })}</span>
		{#if record}
			<span class="meta time" title={$t("response.sentAt")}>{new Date(record.at).toLocaleTimeString($locale)}</span>
		{/if}
		{#if live}
			<button class="disconnect" class:time={!record} onclick={cancelActiveSend} disabled={cancelling}>
				{cancelling ? $t("stream.disconnecting") : $t("stream.disconnect")}
			</button>
		{/if}
	</div>

	{#if record?.error}
		<p class="error">{record.error}</p>
	{/if}
	{#if stream.unresolved_variables.length > 0}
		<p class="warning">
			{$t("response.unresolved", { names: stream.unresolved_variables.map((v) => `{{${v}}}`).join(", ") })}
		</p>
	{/if}

	<div class="tabs">
		<button class:active={tab === "events"} onclick={() => (tab = "events")}>{$t("stream.tab.events")}</button>
		<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
			>{$t("response.tab.headers", { count: stream.headers.length })}</button
		>
		{#if tab === "events"}
			<input
				class="filter"
				type="search"
				bind:value={filter}
				placeholder={$t("stream.filter")}
				aria-label={$t("stream.filter")}
			/>
		{/if}
	</div>

	{#if tab === "headers"}
		<HeadersTable headers={stream.headers} />
	{:else}
		<div class="entries" bind:this={list} onscroll={onScroll}>
			{#if stream.dropped > 0}
				<p class="note">{$t("stream.dropped", { count: stream.dropped, limit: STREAM_ENTRY_LIMIT })}</p>
			{/if}
			{#each visible as entry, i (i)}
				{#if entry.kind === "event"}
					{@const open = expanded.has(entry)}
					<div class="entry" class:open>
						<button
							class="toggle"
							aria-expanded={open}
							aria-label={$t(open ? "stream.collapse" : "stream.expand")}
							onclick={() => toggle(entry)}>{open ? "▾" : "▸"}</button
						>
						<span class="at">{offset(entry.at_ms)}</span>
						<span class="type" class:named={entry.event !== "message"}>{entry.event}</span>
						{#if entry.id != null}<span class="id" title={$t("stream.eventId")}>#{entry.id}</span>{/if}
						{#if open}
							<button class="copy" onclick={() => copyText(entry.data)}>{$t("common.copy")}</button>
						{/if}
						<pre class="data">{open ? readable(entry.data) : entry.data}</pre>
					</div>
				{:else}
					<div class="entry comment" title={$t("stream.commentHint")}>
						<span class="toggle-space"></span>
						<span class="at">{offset(entry.at_ms)}</span>
						<pre class="data">: {entry.text}</pre>
					</div>
				{/if}
			{:else}
				<p class="empty">
					{#if filter.trim()}{$t("stream.noMatches")}{:else if live}{$t("stream.waiting")}{:else}{$t("stream.noEvents")}{/if}
				</p>
			{/each}
		</div>
	{/if}
</div>

<style>
	.stream-viewer {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		height: 100%;
		min-height: 0;
	}
	.status-bar {
		display: flex;
		align-items: center;
		gap: 0.9em;
		flex-wrap: wrap;
	}
	.state {
		display: inline-flex;
		align-items: center;
		gap: 0.45em;
		font-weight: 700;
	}
	.state.open {
		color: var(--ok);
	}
	.state.failed {
		color: var(--danger);
	}
	.state.ended {
		opacity: 0.7;
	}
	.status {
		font-weight: 600;
		font-size: 0.9em;
	}
	.status-ok {
		color: var(--ok);
	}
	.status-redirect {
		color: var(--warn);
	}
	.status-error {
		color: var(--danger);
	}
	.meta {
		opacity: 0.6;
		font-size: 0.9em;
	}
	.elapsed {
		font-family: ui-monospace, monospace;
		font-variant-numeric: tabular-nums;
	}
	.time {
		margin-left: auto;
	}
	.disconnect {
		background: none;
		border: 1px solid var(--line-strong);
		color: inherit;
		border-radius: 6px;
		padding: 0.15em 0.7em;
		font-size: 0.8em;
		cursor: pointer;
	}
	.disconnect:hover:not(:disabled) {
		border-color: var(--danger);
		color: var(--danger);
	}
	.disconnect:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.spinner {
		width: 0.8em;
		height: 0.8em;
		border: 2px solid var(--line-strong);
		border-top-color: var(--accent-text);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	.pulse {
		width: 0.55em;
		height: 0.55em;
		border-radius: 50%;
		background: currentColor;
		animation: pulse 1.6s ease-in-out infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@keyframes pulse {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation-duration: 2.5s;
		}
		.pulse {
			animation: none;
		}
	}
	.error {
		margin: 0;
		color: var(--danger);
		white-space: pre-wrap;
	}
	.warning {
		color: var(--warn);
		background: color-mix(in srgb, var(--warn) 10%, transparent);
		border-radius: 6px;
		padding: 0.4em 0.6em;
		font-size: 0.85em;
		margin: 0;
	}
	.tabs {
		display: flex;
		align-items: flex-end;
		gap: 0.2em;
		border-bottom: 1px solid var(--line);
	}
	.tabs button {
		background: none;
		border: none;
		border-radius: 0;
		padding: 0.35em 0.8em;
		cursor: pointer;
		color: inherit;
		opacity: 0.6;
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		opacity: 1;
		border-bottom-color: var(--accent-text);
	}
	.filter {
		margin: 0 0 0.25em auto;
		width: 14em;
		max-width: 40%;
		padding: 0.2em 0.5em;
		font-size: 0.85em;
	}
	.entries {
		flex: 1;
		min-height: 0;
		overflow: auto;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		font-size: 0.85em;
	}
	.entry {
		display: grid;
		grid-template-columns: auto auto auto auto 1fr auto;
		align-items: baseline;
		column-gap: 0.6em;
		padding: 0.3em 0.6em;
		border-bottom: 1px solid var(--line);
	}
	.entry:last-child {
		border-bottom: none;
	}
	.toggle {
		background: none;
		border: none;
		padding: 0;
		width: 1em;
		color: inherit;
		opacity: 0.6;
		cursor: pointer;
	}
	.toggle-space {
		width: 1em;
	}
	.at {
		font-family: ui-monospace, monospace;
		font-variant-numeric: tabular-nums;
		opacity: 0.55;
		white-space: nowrap;
	}
	.type {
		font-weight: 700;
		font-size: 0.9em;
		opacity: 0.6;
	}
	.type.named {
		opacity: 1;
		color: var(--accent-text);
	}
	.id {
		font-family: ui-monospace, monospace;
		opacity: 0.6;
	}
	.copy {
		grid-column: 6;
		padding: 0 0.5em;
		font-size: 0.9em;
	}
	/* The data gets a row of its own under the event's details, so a long
	   payload wraps across the full width instead of a narrow column. */
	.data {
		grid-column: 2 / -1;
		margin: 0.15em 0 0;
		font-family: ui-monospace, monospace;
		white-space: pre-wrap;
		word-break: break-word;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.entry.open .data {
		display: block;
	}
	.entry.comment {
		opacity: 0.55;
		font-style: italic;
	}
	.entry.comment .data {
		grid-column: auto / -1;
		margin: 0;
	}
	.note,
	.empty {
		margin: 0;
		padding: 0.5em 0.6em;
		opacity: 0.6;
	}
</style>
