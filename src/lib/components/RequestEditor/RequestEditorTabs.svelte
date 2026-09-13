<script lang="ts">
	import { activeRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { incognito } from "../../stores/incognito";
	import { workspacePath } from "../../stores/workspace";
	import { activeResponses, markSending, recordResponse } from "../../stores/response";
	import { availableVariables } from "../../stores/environments";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { notifyResult, type NoticeKind } from "../../ui/notices";
	import { copyText } from "../../ui/clipboard";
	import { newHttpRequestSpec, newId } from "../../bindings/types";
	import type { HttpMethod, HttpRequestSpec } from "../../bindings/types";
	import VariableInput from "../common/VariableInput.svelte";
	import MethodSelect from "./MethodSelect.svelte";
	import KeyValueTable from "./KeyValueTable.svelte";
	import AuthEditor from "./AuthEditor.svelte";
	import BodyEditor from "./BodyEditor.svelte";
	import SaveIncognitoModal from "./SaveIncognitoModal.svelte";

	type Tab = "params" | "headers" | "body" | "auth";
	let tab = $state<Tab>("params");
	let saving = $state(false);
	let copied = $state(false);
	// An incognito request has nowhere to be saved to yet, so saving means
	// choosing a destination first.
	let saveIncognito = $state(false);

	// Sending needs a collection only to resolve that collection's
	// environment; an incognito request has none by design and can still be
	// sent.
	let canSend = $derived($incognito || $activeCollection != null);

	let http = $derived($activeRequest?.request.http ?? newHttpRequestSpec());

	function mutate(patch: Partial<HttpRequestSpec>) {
		if (!$activeRequest) return;
		activeRequest.set({
			...$activeRequest,
			request: { ...$activeRequest.request, http: { ...http, ...patch } },
			dirty: true,
		});
	}

	async function save() {
		if (!$activeRequest) return;
		if ($incognito) {
			saveIncognito = true;
			return;
		}
		saving = true;
		try {
			const saved = await api.saveRequest($activeRequest.path, $activeRequest.request);
			activeRequest.set({ path: $activeRequest.path, request: saved, dirty: false });
			requestTreeRefresh();
		} finally {
			saving = false;
		}
	}

	async function send() {
		if (!$activeRequest || !canSend) return;
		// Results are stored against the request's path, so each request keeps
		// its own last response instead of sharing one global slot.
		const path = $activeRequest.path;
		const name = $activeRequest.request.meta.name;
		const sendId = newId();
		markSending(path, sendId);
		let summary: string;
		let kind: NoticeKind;
		let inBackground: boolean;
		try {
			const outcome = await api.sendRequest(
				$activeRequest.request,
				$workspacePath,
				$activeCollection?.path ?? null,
				sendId,
			);
			summary = `${outcome.status} ${outcome.status_text}`;
			kind = outcome.status >= 400 ? "error" : "success";
			inBackground = recordResponse(path, { outcome, error: null, at: Date.now() });
		} catch (e) {
			summary = $t("request.sendFailedShort");
			kind = "error";
			inBackground = recordResponse(path, { outcome: null, error: String(e), at: Date.now() });
		}
		// The user moved on to another request while this one was in flight,
		// so the response panel they are looking at won't show the result —
		// the toast and the sidebar marker are the only way they learn of it.
		if (inBackground) notifyResult($t("request.backgroundDone", { name, summary }), kind);
	}

	function onShortcut(e: KeyboardEvent) {
		if (!(e.ctrlKey || e.metaKey)) return;
		if (e.key === "Enter") {
			e.preventDefault();
			send();
			return;
		}
		// `e.key` carries the character the layout produces — on a Russian
		// layout the S key yields "ы", so the shortcut has to match the
		// physical key instead. Unrelated to the UI language: it is the
		// keyboard layout that decides this, not the setting.
		if (e.code === "KeyS" || e.key.toLowerCase() === "s") {
			e.preventDefault();
			// In incognito there is nothing to compare against, so Ctrl+S
			// always offers to save rather than waiting for a dirty flag.
			if (!saving && ($incognito || $activeRequest?.dirty)) save();
		}
	}

	/// Breadcrumb of the open request: collection, the folders it sits in,
	/// then the request itself — derived from where the file lives on disk.
	/// Paths are Windows-style here, so both separators are handled.
	let breadcrumb = $derived.by(() => {
		const request = $activeRequest;
		const collection = $activeCollection;
		if (!request) return [] as string[];
		if ($incognito) return [$t("incognito.badge"), request.request.meta.name];
		if (!collection) return [request.request.meta.name];
		const relative = request.path.startsWith(collection.path)
			? request.path.slice(collection.path.length).replace(/^[\\/]+/, "")
			: "";
		const folders = relative
			.split(/[\\/]+/)
			.slice(0, -1)
			.filter(Boolean);
		return [collection.name, ...folders, request.request.meta.name];
	});

	/// The URL as it will actually be sent: variables substituted and enabled
	/// query parameters appended, mirroring what the core does at send time.
	let urlPreview = $derived.by(() => {
		const values = new Map($availableVariables.map((v) => [v.key, v.secret ? "••••" : v.value]));
		const substitute = (text: string) =>
			text.replace(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g, (whole, key) => values.get(key) ?? whole);

		let url = substitute(http.url);
		const pairs = http.query
			.filter((q) => q.enabled && q.key.trim() !== "")
			.map((q) => `${encodeURIComponent(substitute(q.key))}=${encodeURIComponent(substitute(q.value))}`);
		if (pairs.length > 0) url += (url.includes("?") ? "&" : "?") + pairs.join("&");
		return url;
	});

	async function copyUrl() {
		await copyText(urlPreview);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}

	let paramCount = $derived(http.query.filter((q) => q.key).length);
	let headerCount = $derived(http.headers.filter((h) => h.key).length);
</script>

<svelte:window onkeydown={onShortcut} />

{#if saveIncognito && $activeRequest}
	<SaveIncognitoModal request={$activeRequest.request} onClose={() => (saveIncognito = false)} />
{/if}

{#if $activeRequest}
	<div class="editor">
		<div class="request-title">
			<span class="path">
				{#each breadcrumb as part, i (i)}
					{#if i > 0}<span class="sep">/</span>{/if}
					<span class:name={i === breadcrumb.length - 1}>{part}</span>
				{/each}
			</span>
			{#if $activeRequest.dirty}<span class="dirty" title={$t("request.dirty")}>●</span>{/if}
			{#if !canSend}
				<span class="warn">{$t("request.noCollection")}</span>
			{/if}
		</div>
		<div class="url-bar">
			<MethodSelect value={http.method} onChange={(method: HttpMethod) => mutate({ method })} />
			<VariableInput
				value={http.url}
				mono
				ariaLabel={$t("request.urlAria")}
				placeholder={$t("request.urlPlaceholder")}
				onChange={(url) => mutate({ url })}
			/>
			<button class="send" title="Ctrl+Enter" onclick={send} disabled={$activeResponses.loading || !canSend}>
				{$activeResponses.loading ? $t("request.sending") : $t("request.send")}
			</button>
			<button
				class="save"
				title="Ctrl+S"
				onclick={save}
				disabled={saving || (!$incognito && !$activeRequest.dirty)}
			>
				{#if saving}{$t("common.saving")}{:else if $incognito}{$t("request.saveAs")}{:else}{$t("request.save")}{/if}
			</button>
		</div>

		<div class="tabs">
			<button class:active={tab === "params"} onclick={() => (tab = "params")}
				>{$t("request.tab.params")}{#if paramCount}&nbsp;({paramCount}){/if}</button
			>
			<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
				>{$t("request.tab.headers")}{#if headerCount}&nbsp;({headerCount}){/if}</button
			>
			<button class:active={tab === "body"} onclick={() => (tab = "body")}>{$t("request.tab.body")}</button>
			<button class:active={tab === "auth"} onclick={() => (tab = "auth")}>{$t("request.tab.auth")}</button>
		</div>

		<div class="tab-content">
			{#if tab === "params"}
				<div class="params-tab">
					<div class="url-preview">
						<div class="url-preview-header">
							<span>{$t("request.finalUrl")}</span>
							<button onclick={copyUrl}>{copied ? $t("common.copied") : $t("common.copy")}</button>
						</div>
						<code>{urlPreview || "—"}</code>
					</div>
					<KeyValueTable rows={http.query} onChange={(query) => mutate({ query })} />
				</div>
			{:else if tab === "headers"}
				<KeyValueTable rows={http.headers} onChange={(headers) => mutate({ headers })} />
			{:else if tab === "body"}
				<BodyEditor body={http.body} onChange={(body) => mutate({ body })} />
			{:else if tab === "auth"}
				<AuthEditor auth={http.auth} onChange={(auth) => mutate({ auth })} />
			{/if}
		</div>
	</div>
{:else}
	<div class="empty-state-outer">
		<div class="empty-state">
			<p>{$t("request.emptyState")}</p>
		</div>
	</div>
{/if}

<style>
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
		/* Fill the pane instead of collapsing to content width, so the
		   request settings use the full available width. */
		flex: 1;
		min-width: 0;
		gap: 0.6em;
	}
	.url-bar {
		display: flex;
		gap: 0.4em;
	}
	.request-title {
		display: flex;
		align-items: center;
		gap: 0.5em;
		font-weight: 600;
	}
	.request-title .path {
		display: flex;
		align-items: center;
		gap: 0.35em;
		flex-wrap: wrap;
		font-weight: 400;
		opacity: 0.7;
		min-width: 0;
	}
	.request-title .path .name {
		font-weight: 600;
		opacity: 1;
	}
	.request-title .sep {
		opacity: 0.4;
	}
	.request-title .dirty {
		color: #a37c00;
		font-size: 0.8em;
	}
	.request-title .warn {
		font-weight: 400;
		font-size: 0.8em;
		color: #d1443c;
	}
	.send {
		background: #396cd8;
		color: white;
		border: 1px solid #396cd8;
		border-radius: 6px;
		padding: 0.4em 1.2em;
		cursor: pointer;
		font-weight: 600;
		white-space: nowrap;
	}
	.send:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.save {
		border-radius: 6px;
		padding: 0.4em 1em;
		cursor: pointer;
		white-space: nowrap;
	}
	.save:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.tabs {
		display: flex;
		gap: 0.2em;
		border-bottom: 1px solid rgba(127, 127, 127, 0.25);
	}
	.tabs button {
		background: none;
		border: none;
		padding: 0.4em 0.8em;
		cursor: pointer;
		color: inherit;
		opacity: 0.6;
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		opacity: 1;
		border-bottom-color: #396cd8;
	}
	.tab-content {
		flex: 1;
		overflow: auto;
	}
	.params-tab {
		display: flex;
		flex-direction: column;
		gap: 0.8em;
	}
	.url-preview {
		border: 1px solid rgba(127, 127, 127, 0.35);
		border-radius: 8px;
		padding: 0.6em 0.7em;
		background: rgba(127, 127, 127, 0.08);
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}
	.url-preview-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6em;
		font-size: 0.8em;
		opacity: 0.7;
	}
	.url-preview-header button {
		padding: 0.2em 0.7em;
		font-size: 0.95em;
	}
	.url-preview code {
		font-family: ui-monospace, monospace;
		font-size: 0.85em;
		word-break: break-all;
	}
	.empty-state-outer {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		min-width: 0;
		height: 100%;
		opacity: 0.5;
	}
</style>
