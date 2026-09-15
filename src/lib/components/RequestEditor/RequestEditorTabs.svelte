<script lang="ts">
	// Everything about the request that is not its address: the four tabs and
	// whichever one is open. The method, the URL and the send/save buttons
	// live in `RequestHeader`, which the workbench draws across the full
	// width above both panes.
	import { activeRequest, mutateHttp } from "../../stores/activeRequest";
	import { availableVariables } from "../../stores/environments";
	import { t } from "../../i18n";
	import { copyText } from "../../ui/clipboard";
	import { newHttpRequestSpec } from "../../bindings/types";
	import KeyValueTable from "./KeyValueTable.svelte";
	import AuthEditor from "./AuthEditor.svelte";
	import BodyEditor from "./BodyEditor.svelte";

	type Tab = "params" | "headers" | "body" | "auth";
	let tab = $state<Tab>("params");
	let copied = $state(false);

	let http = $derived($activeRequest?.request.http ?? newHttpRequestSpec());

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

{#if $activeRequest}
	<div class="editor">
		<div class="tabs">
			<button class:active={tab === "params"} onclick={() => (tab = "params")}
				>{$t("request.tab.params")}{#if paramCount}&nbsp;({paramCount}){/if}</button
			>
			<button class:active={tab === "body"} onclick={() => (tab = "body")}>{$t("request.tab.body")}</button>
			<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
				>{$t("request.tab.headers")}{#if headerCount}&nbsp;({headerCount}){/if}</button
			>
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
						<code>{urlPreview || "-"}</code>
					</div>
					<KeyValueTable rows={http.query} onChange={(query) => mutateHttp({ query })} />
				</div>
			{:else if tab === "headers"}
				<KeyValueTable rows={http.headers} onChange={(headers) => mutateHttp({ headers })} />
			{:else if tab === "body"}
				<BodyEditor body={http.body} onChange={(body) => mutateHttp({ body })} />
			{:else if tab === "auth"}
				<AuthEditor auth={http.auth} onChange={(auth) => mutateHttp({ auth })} />
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
