<script lang="ts">
	// What the request is and how to fire it: the breadcrumb, the method, the
	// URL and the two buttons. Kept apart from the tabs below because the
	// workbench lays it across the full width - in the side-by-side layout
	// the editor pane is only half the window, which is not much of a URL
	// field.
	import { activeRequest, mutateHttp } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { incognito } from "../../stores/incognito";
	import { workspacePath } from "../../stores/workspace";
	import { activeResponses, markSending, recordResponse } from "../../stores/response";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { notifyResult, type NoticeKind } from "../../ui/notices";
	import { newHttpRequestSpec, newId } from "../../bindings/types";
	import type { HttpMethod } from "../../bindings/types";
	import VariableInput from "../common/VariableInput.svelte";
	import MethodSelect from "./MethodSelect.svelte";
	import SaveIncognitoModal from "./SaveIncognitoModal.svelte";

	let saving = $state(false);
	// An incognito request has nowhere to be saved to yet, so saving means
	// choosing a destination first.
	let saveIncognito = $state(false);

	// Sending needs a collection only to resolve that collection's
	// environment; an incognito request has none by design and can still be
	// sent.
	let canSend = $derived($incognito || $activeCollection != null);

	let http = $derived($activeRequest?.request.http ?? newHttpRequestSpec());

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
		// so the response panel they are looking at won't show the result -
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
		// `e.key` carries the character the layout produces - on a Russian
		// layout the S key yields "ы", so the shortcut has to match the
		// physical key instead.
		if (e.code === "KeyS" || e.key.toLowerCase() === "s") {
			e.preventDefault();
			// In incognito there is nothing to compare against, so Ctrl+S
			// always offers to save rather than waiting for a dirty flag.
			if (!saving && ($incognito || $activeRequest?.dirty)) save();
		}
	}

	/// Breadcrumb of the open request: collection, the folders it sits in,
	/// then the request itself - derived from where the file lives on disk.
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
</script>

<svelte:window onkeydown={onShortcut} />

{#if saveIncognito && $activeRequest}
	<SaveIncognitoModal request={$activeRequest.request} onClose={() => (saveIncognito = false)} />
{/if}

{#if $activeRequest}
	<div class="request-header">
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
			<MethodSelect value={http.method} onChange={(method: HttpMethod) => mutateHttp({ method })} />
			<VariableInput
				value={http.url}
				mono
				ariaLabel={$t("request.urlAria")}
				placeholder={$t("request.urlPlaceholder")}
				onChange={(url) => mutateHttp({ url })}
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
	</div>
{/if}

<style>
	/* The gap below the header is the component's own rather than a grid gap
	   in the workbench: with no request open this renders nothing at all, and
	   a grid gap would leave a blank stripe where the header would have
	   been. */
	.request-header {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		padding-bottom: 0.6em;
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
</style>
