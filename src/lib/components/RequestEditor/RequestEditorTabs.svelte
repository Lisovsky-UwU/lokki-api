<script lang="ts">
	import { activeRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { responseState } from "../../stores/response";
	import { api } from "../../api/client";
	import { newHttpRequestSpec } from "../../bindings/types";
	import type { HttpMethod, HttpRequestSpec } from "../../bindings/types";
	import KeyValueTable from "./KeyValueTable.svelte";
	import AuthEditor from "./AuthEditor.svelte";
	import BodyEditor from "./BodyEditor.svelte";

	type Tab = "params" | "headers" | "body" | "auth";
	let tab = $state<Tab>("params");
	let saving = $state(false);

	const methods: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

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
		if (!$activeRequest || !$activeCollection) return;
		responseState.set({ outcome: null, error: null, loading: true });
		try {
			const outcome = await api.sendRequest($activeRequest.request, $activeCollection.path);
			responseState.set({ outcome, error: null, loading: false });
		} catch (e) {
			responseState.set({ outcome: null, error: String(e), loading: false });
		}
	}

	function sendOnShortcut(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
			e.preventDefault();
			send();
		}
	}
</script>

<svelte:window onkeydown={sendOnShortcut} />

{#if $activeRequest}
	<div class="editor">
		<div class="request-title">
			<span class="name">{$activeRequest.request.meta.name}</span>
			{#if $activeRequest.dirty}<span class="dirty" title="Есть несохранённые изменения">●</span>{/if}
			{#if !$activeCollection}
				<span class="warn">Коллекция не определена — отправка недоступна</span>
			{/if}
		</div>
		<div class="url-bar">
			<select value={http.method} onchange={(e) => mutate({ method: (e.target as HTMLSelectElement).value as HttpMethod })}>
				{#each methods as m}
					<option value={m}>{m}</option>
				{/each}
			</select>
			<input
				class="url mono"
				placeholder="https://api.example.com/pets или {'{{baseUrl}}'}/pets"
				value={http.url}
				oninput={(e) => mutate({ url: (e.target as HTMLInputElement).value })}
			/>
			<button class="send" onclick={send} disabled={$responseState.loading || !$activeCollection}>
				{$responseState.loading ? "..." : "Send"}
			</button>
			<button class="save" onclick={save} disabled={!$activeRequest.dirty || saving}>
				{saving ? "Сохранение..." : "Save"}
			</button>
		</div>

		<div class="tabs">
			<button class:active={tab === "params"} onclick={() => (tab = "params")}
				>Params{#if http.query.length}&nbsp;({http.query.filter((q) => q.key).length}){/if}</button
			>
			<button class:active={tab === "headers"} onclick={() => (tab = "headers")}
				>Headers{#if http.headers.length}&nbsp;({http.headers.filter((h) => h.key).length}){/if}</button
			>
			<button class:active={tab === "body"} onclick={() => (tab = "body")}>Body</button>
			<button class:active={tab === "auth"} onclick={() => (tab = "auth")}>Auth</button>
		</div>

		<div class="tab-content">
			{#if tab === "params"}
				<KeyValueTable rows={http.query} onChange={(query) => mutate({ query })} />
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
	<div class="empty-state">
		<p>Выберите запрос слева или создайте новый.</p>
	</div>
{/if}

<style>
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
		gap: 0.6em;
	}
	.url-bar {
		display: flex;
		gap: 0.4em;
	}
	.url {
		flex: 1;
		min-width: 0;
	}
	.mono {
		font-family: ui-monospace, monospace;
	}
	.request-title {
		display: flex;
		align-items: center;
		gap: 0.5em;
		font-weight: 600;
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
	}
	.send:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.save {
		border-radius: 6px;
		padding: 0.4em 1em;
		cursor: pointer;
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
	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		opacity: 0.5;
	}
</style>
