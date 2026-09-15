<script lang="ts">
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import type { ImportResult } from "../../bindings/types";
	import { collections } from "../../stores/workspace";
	import { requestTreeRefresh } from "../../stores/collectionTree";
	import { setExpanded } from "../../stores/expansion";
	import { notifyResult, reportError } from "../../ui/notices";

	let { workspacePath, onClose }: { workspacePath: string; onClose: () => void } = $props();

	let source = $state<"file" | "url">("file");
	let filePath = $state<string | null>(null);
	let url = $state("");
	let running = $state(false);
	// Set once the import is done: the dialog then stops being a form and
	// becomes the report, because the warnings are the whole reason the
	// import has something to say afterwards.
	let result = $state<ImportResult | null>(null);

	let canRun = $derived(!running && (source === "file" ? filePath !== null : url.trim() !== ""));

	async function chooseFile() {
		const picked = await api.pickSpecFile();
		if (picked) filePath = picked;
	}

	async function run() {
		if (!canRun) return;
		running = true;
		try {
			const imported =
				source === "file"
					? await api.importOpenApiFile(workspacePath, filePath!)
					: await api.importOpenApiUrl(workspacePath, url.trim());
			// Re-read rather than appended: the order of collections lives in
			// their files, and the name may have gained a suffix on the way in.
			collections.set(await api.listCollections(workspacePath));
			setExpanded(imported.collection.path, true);
			requestTreeRefresh();
			notifyResult($t("import.done", { name: imported.collection.name }));
			// Kept open rather than closed on success: closing would take the
			// warnings with it, and they are what the user has to act on.
			result = imported;
		} catch (e) {
			reportError($t("import.failed"), e);
		} finally {
			running = false;
		}
	}
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onClose();
	}}
>
	<div class="modal" role="dialog" tabindex="-1" aria-modal="true" aria-label={$t("import.title")}>
		<h2>{$t("import.title")}</h2>

		{#if result}
			<p class="done">{$t("import.done", { name: result.collection.name })}</p>
			<p class="hint">
				{$t("import.summary", {
					requests: result.requests,
					folders: result.folders,
					environments: result.environments,
				})}
			</p>
			{#if result.warnings.length > 0}
				<h3>{$t("import.warnings")}</h3>
				<ul class="warnings">
					{#each result.warnings as warning, index (index)}
						<li>{warning}</li>
					{/each}
				</ul>
			{/if}
			<div class="actions">
				<button class="primary" onclick={onClose}>{$t("common.close")}</button>
			</div>
		{:else}
			<p class="hint">{$t("import.formatHint")}</p>

			<div class="sources">
				<label class="radio">
					<input type="radio" value="file" bind:group={source} />
					<span>{$t("import.fromFile")}</span>
				</label>
				<label class="radio">
					<input type="radio" value="url" bind:group={source} />
					<span>{$t("import.fromUrl")}</span>
				</label>
			</div>

			{#if source === "file"}
				<div class="field">
					<button onclick={chooseFile}>{$t("import.chooseFile")}</button>
					<span class="path" title={filePath ?? ""}>{filePath ?? $t("import.noFileChosen")}</span>
				</div>
			{:else}
				<input class="url" bind:value={url} placeholder={$t("import.urlPlaceholder")} />
			{/if}

			<p class="hint">{$t("import.structureHint")}</p>

			<div class="actions">
				<button onclick={onClose}>{$t("common.cancel")}</button>
				<button class="primary" onclick={run} disabled={!canRun}
					>{running ? $t("import.running") : $t("import.run")}</button
				>
			</div>
		{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 20;
	}
	.modal {
		background: var(--modal-bg, #fff);
		color: inherit;
		border-radius: 10px;
		padding: 1.2em;
		width: min(34em, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.7em;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
	}
	h2 {
		margin: 0;
		font-size: 1.1em;
	}
	h3 {
		margin: 0.3em 0 0;
		font-size: 0.9em;
	}
	.sources {
		display: flex;
		gap: 1.2em;
		font-size: 0.9em;
	}
	.radio {
		display: flex;
		align-items: center;
		gap: 0.35em;
	}
	.field {
		display: flex;
		align-items: center;
		gap: 0.6em;
		font-size: 0.9em;
		min-width: 0;
	}
	.field button {
		cursor: pointer;
		white-space: nowrap;
	}
	/* The interesting end of a long path is the file name, so the middle is
	   what scrolls out of sight. */
	.path {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		direction: rtl;
		text-align: left;
		opacity: 0.7;
	}
	.url {
		width: 100%;
	}
	.done {
		margin: 0;
		font-size: 0.95em;
	}
	.hint {
		margin: 0;
		font-size: 0.85em;
		opacity: 0.6;
	}
	.warnings {
		margin: 0;
		padding-left: 1.2em;
		max-height: 14em;
		overflow-y: auto;
		font-size: 0.85em;
		display: flex;
		flex-direction: column;
		gap: 0.3em;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5em;
		margin-top: 0.3em;
	}
	.actions button {
		cursor: pointer;
	}
	.primary {
		background: #396cd8;
		color: white;
		border-color: #396cd8;
	}
	.primary:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
