<script lang="ts">
	import { activeRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { activeResponses, markSending, recordResponse } from "../../stores/response";
	import { api } from "../../api/client";
	import { newHttpRequestSpec } from "../../bindings/types";
	import type { HttpMethod, HttpRequestSpec } from "../../bindings/types";
	import VariableInput from "../common/VariableInput.svelte";
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
		// Results are stored against the request's path, so each request keeps
		// its own last response instead of sharing one global slot.
		const path = $activeRequest.path;
		markSending(path);
		try {
			const outcome = await api.sendRequest($activeRequest.request, $activeCollection.path);
			recordResponse(path, { outcome, error: null, at: Date.now() });
		} catch (e) {
			recordResponse(path, { outcome: null, error: String(e), at: Date.now() });
		}
	}

	function onShortcut(e: KeyboardEvent) {
		if (!(e.ctrlKey || e.metaKey)) return;
		if (e.key === "Enter") {
			e.preventDefault();
			send();
		} else if (e.key.toLowerCase() === "s") {
			e.preventDefault();
			if ($activeRequest?.dirty && !saving) save();
		}
	}

	/// Breadcrumb of the open request: collection, any folders, then the
	/// request itself, derived from where the file sits on disk.
	let breadcrumb = $derived.by(() => {
		const request = $activeRequest;
		const collection = $activeCollection;
		if (!request) return [] as string[];
		if (!collection) return [request.request.meta.name];
		const relative = request.path.startsWith(collection.path)
			? request.path.slice(collection.path.length).replace(/^[\/]+/, "")
			: "";
		const folders = relative.split(/[\/]+/).slice(0, -1).filter(Boolean);
		return [collection.name, ...folders, request.request.meta.name];
	});
</script>

<svelte:window onkeydown={onShortcut} />

{#if $activeRequest}
	<div class="editor">
		<div class="request-title">
			<span class="path">
				{#each breadcrumb as part, i (i)}
					{#if i > 0}<span class="sep">/</span>{/if}
					<span class:name={i === breadcrumb.length - 1}>{part}</span>
				{/each}
			</span>
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
			<VariableInput
				value={http.url}
				mono
				ariaLabel="URL запроса"
				placeholder="https://api.example.com/pets или {'{{baseUrl}}'}/pets"
				onChange={(url) => mutate({ url })}
			/>
			<button class="send" onclick={send} disabled={$activeResponses.loading || !$activeCollection}>
				{$activeResponses.loading ? "..." : "Send"}
			</button>
			<button class="save" title="Ctrl+S" onclick={save} disabled={!$activeRequest.dirty || saving}>
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
		flex: 1;
		min-width: 0;
		height: 100%;
		opacity: 0.5;
	}
</style>
