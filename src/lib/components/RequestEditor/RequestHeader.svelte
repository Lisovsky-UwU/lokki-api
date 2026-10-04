<script lang="ts">
	// How to fire the request: the method, the URL and the two buttons. Kept
	// apart from the tabs below because the workbench lays it across the full
	// width - in the side-by-side layout the editor pane is only half the
	// window, which is not much of a URL field. Where the request lives is
	// `RequestTitle`, up in the top bar.
	import { activeRequest, mutateHttp } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { incognito } from "../../stores/incognito";
	import { activeResponses } from "../../stores/response";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { cancelActiveSend, sendActiveRequest } from "../../ui/sending";
	import { newHttpRequestSpec } from "../../bindings/types";
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
	let isStream = $derived($activeRequest?.request.meta.protocol === "sse");

	// The handler takes either; the hint names the one this keyboard has.
	const modKey = /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘" : "Ctrl";

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

	function send() {
		if (canSend) sendActiveRequest();
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

</script>

<svelte:window onkeydown={onShortcut} />

{#if saveIncognito && $activeRequest}
	<SaveIncognitoModal request={$activeRequest.request} onClose={() => (saveIncognito = false)} />
{/if}

{#if $activeRequest}
	<div class="request-header">
		<!-- One field, as far as the eye is concerned: the method is the first
		     thing in it rather than a control standing beside it. -->
		<div class="url-field">
			<MethodSelect value={http.method} onChange={(method: HttpMethod) => mutateHttp({ method })} />
			<VariableInput
				value={http.url}
				mono
				ariaLabel={$t("request.urlAria")}
				placeholder={$t("request.urlPlaceholder")}
				onChange={(url) => mutateHttp({ url })}
			/>
		</div>
			{#if isStream && $activeResponses.loading}
				<!-- An open stream has no natural end, so the button that opened
				     it is the one that closes it. -->
				<button class="send disconnect" onclick={cancelActiveSend} disabled={$activeResponses.cancelling}>
					{$activeResponses.cancelling ? $t("stream.disconnecting") : $t("stream.disconnect")}
				</button>
			{:else if isStream}
				<button class="send" title="{modKey}+Enter" onclick={send} disabled={!canSend}>
					{$t("stream.connect")}<kbd>{modKey} ↵</kbd>
				</button>
			{:else}
				<button class="send" title="{modKey}+Enter" onclick={send} disabled={$activeResponses.loading || !canSend}>
					{$activeResponses.loading ? $t("request.sending") : $t("request.send")}<kbd>{modKey} ↵</kbd>
				</button>
			{/if}
			<button
				class="save"
				title="{modKey}+S"
				onclick={save}
				disabled={saving || (!$incognito && !$activeRequest.dirty)}
			>
				{#if saving}{$t("common.saving")}{:else if $incognito}{$t("request.saveAs")}{:else}{$t("request.save")}{/if}
			</button>
	</div>
{/if}

<style>
	.request-header {
		display: flex;
		gap: 8px;
		height: 36px;
	}
	.url-field {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: stretch;
		border: 1px solid var(--line-strong);
		border-radius: 8px;
		background: var(--surface-raised);
	}
	/* The ring goes round the whole field, whichever half has focus. */
	.url-field:focus-within {
		border-color: var(--accent-text);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 22%, transparent);
	}
	.url-field :global(.method-select .trigger) {
		height: 100%;
		border: none;
		border-right: 1px solid var(--line);
		border-radius: 7px 0 0 7px;
		background: none;
	}
	.url-field :global(input) {
		height: 100%;
		border: none;
		background: none;
		font-size: var(--fs-lg);
	}
	.url-field :global(:focus-visible) {
		outline: none;
	}
	.send {
		display: flex;
		align-items: center;
		gap: 10px;
		background: var(--accent);
		color: var(--accent-ink);
		border: 1px solid var(--accent);
		border-radius: 8px;
		padding: 0 12px 0 16px;
		cursor: pointer;
		font-weight: 600;
		font-size: var(--fs-lg);
		white-space: nowrap;
	}
	.send kbd {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		font-weight: 500;
		padding: 1px 5px;
		border-radius: 4px;
		background: color-mix(in srgb, currentColor 12%, transparent);
	}
	.send:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.send.disconnect {
		padding: 0 16px;
		background: var(--danger-fill);
		border-color: var(--danger-fill);
		color: var(--on-fill);
	}
	.save {
		border-radius: 8px;
		padding: 0 14px;
		cursor: pointer;
		white-space: nowrap;
	}
	.save:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
