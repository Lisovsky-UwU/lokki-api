<script lang="ts">
	import { untrack } from "svelte";
	import type { CollectionSummary, CollectionTreeNode, RequestFile } from "../../bindings/types";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { activeRequest } from "../../stores/activeRequest";
	import { activeCollection, requestTreeRefresh } from "../../stores/collectionTree";
	import { exitIncognito } from "../../stores/incognito";
	import { collections, workspace } from "../../stores/workspace";
	import { notifyResult, reportError } from "../../ui/notices";

	let { request, onClose }: { request: RequestFile; onClose: () => void } = $props();

	interface FolderChoice {
		path: string;
		label: string;
		depth: number;
	}

	// Saving into a workspace is only on offer when one is open: incognito
	// started from the welcome screen has nowhere to file a request.
	let canUseWorkspace = $derived($workspace != null && $collections.length > 0);
	let target = $state<"workspace" | "file">("workspace");
	// Seeded once and then owned by the field: the dialog is a snapshot of
	// the request as it was opened, and retyping the name must not be undone
	// by an edit behind the modal.
	let name = $state(untrack(() => request.meta.name));
	let collection = $state<CollectionSummary | null>(null);
	let folders = $state<FolderChoice[]>([]);
	let folderPath = $state<string | null>(null);
	let saving = $state(false);

	$effect(() => {
		if (!canUseWorkspace) target = "file";
	});

	// Default to the collection the user was last in, else the first one.
	$effect(() => {
		if (collection || $collections.length === 0) return;
		collection = $activeCollection ?? $collections[0];
	});

	/// Flattens the collection into the folders a request can be filed into,
	/// the collection root included — folder depth is kept only to indent the
	/// list.
	function flatten(node: CollectionTreeNode, depth: number, into: FolderChoice[]) {
		if (node.kind !== "Folder") return;
		for (const child of node.children) {
			if (child.kind === "Folder") {
				into.push({ path: child.path, label: child.name, depth });
				flatten(child, depth + 1, into);
			}
		}
	}

	$effect(() => {
		const chosen = collection;
		if (!chosen) return;
		void (async () => {
			try {
				const tree = await api.loadCollectionTree(chosen.path);
				const list: FolderChoice[] = [{ path: chosen.path, label: $t("saveIncognito.collectionRoot"), depth: 0 }];
				flatten(tree, 1, list);
				folders = list;
				folderPath = chosen.path;
			} catch (e) {
				reportError($t("saveIncognito.loadFoldersFailed"), e);
				folders = [];
				folderPath = null;
			}
		})();
	});

	let canSave = $derived(
		name.trim() !== "" && !saving && (target === "file" || (collection != null && folderPath != null)),
	);

	async function saveToWorkspace() {
		if (!collection || !folderPath) return;
		const saved = await api.adoptRequest(folderPath, name.trim(), $state.snapshot(request));
		// The request now exists on disk, so incognito has done its job: drop
		// the mode and open the saved request where it landed.
		exitIncognito();
		activeCollection.set(collection);
		activeRequest.set({ path: saved.path, request: saved, dirty: false });
		requestTreeRefresh();
		notifyResult($t("saveIncognito.savedToCollection", { name: collection.name }));
	}

	async function saveToFile() {
		const filePath = await api.pickRequestFile(name.trim());
		if (!filePath) return;
		await api.exportRequest(filePath, name.trim(), $state.snapshot(request));
		// Incognito continues: the file is a copy, not a home for the request.
		const current = $activeRequest;
		if (current) {
			activeRequest.set({ ...current, request: { ...current.request, meta: { ...current.request.meta, name: name.trim() } } });
		}
		notifyResult($t("saveIncognito.savedToFile", { path: filePath }));
	}

	async function save() {
		saving = true;
		try {
			if (target === "workspace") await saveToWorkspace();
			else await saveToFile();
			onClose();
		} catch (e) {
			reportError($t("request.saveFailed"), e);
		} finally {
			saving = false;
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
	<div class="modal" role="dialog" tabindex="-1" aria-modal="true" aria-label={$t("saveIncognito.title")}>
		<h2>{$t("saveIncognito.title")}</h2>

		<label class="field">
			<span>{$t("common.name")}</span>
			<input bind:value={name} placeholder={$t("saveIncognito.namePlaceholder")} />
		</label>

		<div class="targets">
			<label class="radio" class:disabled={!canUseWorkspace}>
				<input type="radio" value="workspace" bind:group={target} disabled={!canUseWorkspace} />
				<span>{$t("saveIncognito.toWorkspace")}</span>
			</label>
			<label class="radio">
				<input type="radio" value="file" bind:group={target} />
				<span>{$t("saveIncognito.toFile")}</span>
			</label>
		</div>

		{#if target === "workspace"}
			{#if canUseWorkspace}
				<label class="field">
					<span>{$t("saveIncognito.collection")}</span>
					<select
						value={collection?.path ?? ""}
						onchange={(e) => {
							const path = (e.target as HTMLSelectElement).value;
							collection = $collections.find((c) => c.path === path) ?? null;
						}}
					>
						{#each $collections as item (item.path)}
							<option value={item.path}>{item.name}</option>
						{/each}
					</select>
				</label>
				<label class="field">
					<span>{$t("saveIncognito.folder")}</span>
					<select bind:value={folderPath}>
						{#each folders as folder (folder.path)}
							<option value={folder.path}>{"  ".repeat(folder.depth) + folder.label}</option>
						{/each}
					</select>
				</label>
			{:else}
				<p class="hint">{$t("saveIncognito.noWorkspace")}</p>
			{/if}
		{:else}
			<p class="hint">
				{$t("saveIncognito.fileHint")}
			</p>
		{/if}

		<div class="actions">
			<button onclick={onClose}>{$t("common.cancel")}</button>
			<button class="primary" onclick={save} disabled={!canSave}
				>{saving ? $t("common.saving") : $t("common.save")}</button
			>
		</div>
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
		width: min(30em, 92vw);
		display: flex;
		flex-direction: column;
		gap: 0.7em;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
	}
	h2 {
		margin: 0;
		font-size: 1.1em;
	}
	.field {
		display: flex;
		align-items: center;
		gap: 0.6em;
		font-size: 0.9em;
	}
	.field > span {
		width: 7em;
		opacity: 0.6;
		font-size: 0.9em;
	}
	.field input,
	.field select {
		flex: 1;
		min-width: 0;
	}
	.targets {
		display: flex;
		gap: 1.2em;
		font-size: 0.9em;
	}
	.radio {
		display: flex;
		align-items: center;
		gap: 0.35em;
	}
	.radio.disabled {
		opacity: 0.5;
	}
	.hint {
		margin: 0;
		font-size: 0.85em;
		opacity: 0.6;
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
