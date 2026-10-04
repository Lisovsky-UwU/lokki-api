<script lang="ts">
	// An open connection - a Server-Sent Events stream or a WebSocket - while
	// it is open and after it has ended: what state it is in, then what came
	// through it, in order. The two differ only in what an entry is.
	import { tick } from "svelte";
	import Icon from "../common/Icon.svelte";
	import { SvelteSet } from "svelte/reactivity";
	import { locale, t } from "../../i18n";
	import { STREAM_ENTRY_LIMIT, type ResponseRecord, type StreamEntry, type StreamRecord } from "../../stores/response";
	import type { WsFrame } from "../../bindings/types";
	import { cancelActiveSend } from "../../ui/sending";
	import { copyText } from "../../ui/clipboard";
	import { formatDuration, formatElapsed, formatSize, hexPreview } from "../../ui/format";
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

	type Entry = StreamEntry;

	let socket = $derived(stream.protocol === "websocket");

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
		return stream.entries.filter((entry) => {
			if (entry.kind === "event")
				return (
					entry.event.toLowerCase().includes(needle) ||
					entry.data.toLowerCase().includes(needle) ||
					(entry.id ?? "").toLowerCase().includes(needle)
				);
			if (entry.kind === "comment") return entry.text.toLowerCase().includes(needle);
			// Base64 is not worth searching, so binary frames match by kind.
			return entry.opcode.includes(needle) || (entry.opcode === "text" && entry.data.toLowerCase().includes(needle));
		});
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
			return { label: $t(socket ? "socket.open" : "stream.open"), tone: "open" };
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

	/// What the server's close frame said, for a socket it closed.
	let closeNote = $derived.by(() => {
		const end = stream.end;
		if (end?.type !== "closed" || !("code" in end)) return "";
		const code = end.code != null ? $t("socket.closeCode", { code: end.code }) : $t("socket.noCloseCode");
		return end.reason ? `${code} · ${end.reason}` : code;
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

	/// A frame's payload as it is shown: the text as it came, binary as hex.
	/// Pings and pongs usually carry nothing, and then show nothing. A
	/// collapsed row shows three lines at most, so it only builds those.
	function framePayload(frame: WsFrame, open: boolean): string {
		if (frame.opcode === "text") return open ? readable(frame.data) : frame.data;
		const { text, truncated } = hexPreview(frame.data, open ? 1024 : 48);
		return truncated ? `${text}\n…` : text;
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
		{#if socket}
			<span class="meta">{$t("socket.counts", { sent: stream.sent, received: stream.received })}</span>
		{:else}
			<span class="meta">{$t("stream.eventCount", { count: eventCount + stream.dropped })}</span>
		{/if}
		{#if closeNote}<span class="meta" title={$t("socket.closeCodeHint")}>{closeNote}</span>{/if}
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
		<button class:active={tab === "events"} onclick={() => (tab = "events")}
			>{$t(socket ? "socket.tab.messages" : "stream.tab.events")}</button
		>
		<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
			>{$t("response.tab.headers", { count: stream.headers.length })}</button
		>
		{#if tab === "events"}
			<input
				class="filter"
				type="search"
				bind:value={filter}
				placeholder={$t(socket ? "socket.filter" : "stream.filter")}
				aria-label={$t(socket ? "socket.filter" : "stream.filter")}
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
							onclick={() => toggle(entry)}><span class="chevron" class:collapsed={!open}><Icon name="chevron" size="12px" /></span></button
						>
						<span class="at">{offset(entry.at_ms)}</span>
						<span class="type" class:named={entry.event !== "message"}>{entry.event}</span>
						{#if entry.id != null}<span class="id" title={$t("stream.eventId")}>#{entry.id}</span>{/if}
						{#if open}
							<button class="copy" onclick={() => copyText(entry.data)}>{$t("common.copy")}</button>
						{/if}
						<pre class="data">{open ? readable(entry.data) : entry.data}</pre>
					</div>
				{:else if entry.kind === "frame"}
					{@const open = expanded.has(entry)}
					{@const payload = framePayload(entry, open)}
					{@const control = entry.opcode === "ping" || entry.opcode === "pong"}
					<div class="entry frame {entry.direction}" class:open class:control>
						{#if payload}
							<button
								class="toggle"
								aria-expanded={open}
								aria-label={$t(open ? "socket.collapse" : "socket.expand")}
								onclick={() => toggle(entry)}><span class="chevron" class:collapsed={!open}><Icon name="chevron" size="12px" /></span></button
							>
						{:else}
							<span class="toggle-space"></span>
						{/if}
						<span class="at">{offset(entry.at_ms)}</span>
						<span class="type direction" title={$t(entry.direction === "sent" ? "socket.sent" : "socket.received")}
							>{entry.direction === "sent" ? "↑" : "↓"}{#if entry.opcode !== "text"}&nbsp;{entry.opcode}{/if}</span
						>
						<span class="id">
							{formatSize($t, entry.size)}
							{#if entry.unresolved_variables?.length}
								<span
									class="unresolved"
									title={$t("response.unresolved", {
										names: entry.unresolved_variables.map((v) => `{{${v}}}`).join(", "),
									})}>!</span
								>
							{/if}
						</span>
						{#if open}
							<button class="copy" onclick={() => copyText(entry.data)}
								>{$t(entry.opcode === "text" ? "common.copy" : "socket.copyBase64")}</button
							>
						{/if}
						{#if payload}
							<pre class="data" class:hex={entry.opcode !== "text"} title={entry.opcode === "text" ? undefined : $t("socket.binaryHint")}>{payload}</pre>
						{/if}
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
					{#if socket}
						{#if filter.trim()}{$t("socket.noMatches")}{:else if live}{$t("socket.waiting")}{:else}{$t("socket.noMessages")}{/if}
					{:else if filter.trim()}{$t("stream.noMatches")}{:else if live}{$t("stream.waiting")}{:else}{$t("stream.noEvents")}{/if}
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
		color: var(--text-muted);
	}
	/* Same chip as a plain response's status. */
	.status {
		display: inline-flex;
		align-items: center;
		height: 24px;
		padding: 0 10px;
		border-radius: 12px;
		font-weight: 600;
		white-space: nowrap;
		background: color-mix(in srgb, currentColor 12%, transparent);
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
		color: var(--text-muted);
		font-size: var(--fs-md);
	}
	.elapsed {
		font-family: var(--font-mono);
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
		font-size: var(--fs-sm);
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
		font-size: var(--fs-sm);
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
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		color: var(--text);
		border-bottom-color: var(--accent-text);
	}
	.filter {
		margin: 0 0 0.25em auto;
		width: 14em;
		max-width: 40%;
		padding: 0.2em 0.5em;
		font-size: var(--fs-sm);
	}
	.entries {
		flex: 1;
		min-height: 0;
		overflow: auto;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
		font-size: var(--fs-sm);
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
		display: flex;
		align-items: center;
		width: 1em;
		color: var(--text-muted);
		cursor: pointer;
	}
	.toggle-space {
		width: 1em;
	}
	.at {
		font-family: var(--font-mono);
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
		white-space: nowrap;
	}
	.type {
		font-weight: 700;
		font-size: var(--fs-md);
		color: var(--text-muted);
	}
	.type.named {
		color: var(--text);
		color: var(--accent-text);
	}
	.id {
		font-family: var(--font-mono);
		color: var(--text-muted);
	}
	.copy {
		grid-column: 6;
		padding: 0 0.5em;
		font-size: var(--fs-md);
	}
	/* The data gets a row of its own under the event's details, so a long
	   payload wraps across the full width instead of a narrow column. */
	.data {
		grid-column: 2 / -1;
		margin: 0.15em 0 0;
		font-family: var(--font-mono);
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
	.direction {
		font-family: var(--font-mono);
		white-space: nowrap;
	}
	.frame.sent .direction {
		color: var(--accent-text);
	}
	.frame.received .direction {
		color: var(--method-get);
	}
	.frame.control {
		color: var(--text-muted);
	}
	.unresolved {
		margin-left: 0.3em;
		font-weight: 700;
		color: var(--warn);
		cursor: help;
	}
	.data.hex {
		color: var(--text-muted);
	}
	.entry.comment {
		color: var(--text-muted);
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
		color: var(--text-muted);
	}
	.chevron {
		display: inline-flex;
		transition: transform 0.15s;
	}
	.chevron.collapsed {
		transform: rotate(-90deg);
	}
</style>
