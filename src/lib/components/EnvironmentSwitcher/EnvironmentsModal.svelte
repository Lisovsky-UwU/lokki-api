<script lang="ts">
	import type { EnvironmentEntry, EnvironmentScope } from "../../bindings/types";
	import { newId } from "../../bindings/types";
	import { isValidVariableName, requestEnvironmentsRefresh, sanitizeVariableName } from "../../stores/environments";
	import { api } from "../../api/client";
	import { t } from "../../i18n";
	import { confirmAction, promptForText } from "../../ui/dialogs";
	import { reportError } from "../../ui/notices";

	let {
		rootPath,
		scope,
		title,
		workspacePath,
		onClose,
	}: {
		rootPath: string;
		scope: EnvironmentScope;
		/// Whose environments these are — the workspace or the collection.
		title: string;
		/// Secrets always live in the workspace store, even for a collection
		/// environment, so both paths are needed.
		workspacePath: string;
		onClose: () => void;
	} = $props();

	let entries = $state<EnvironmentEntry[]>([]);
	// Which environment this scope currently resolves variables from. Shown
	// only as a marker — switching stays in the top-bar selector, where it is
	// part of the send flow rather than of editing.
	let activeId = $state<string | null>(null);
	let loading = $state(true);
	let saving = $state(false);

	// The right-hand pane edits a detached copy; the list on the left keeps
	// showing what is actually on disk until a save goes through.
	let draft = $state<EnvironmentEntry | null>(null);
	// Secret values never travel inside the environment file (that one gets
	// written to the possibly-synced .env.toml) — they are read from and
	// written to the local SecretStore separately.
	let secretValues = $state<Record<string, string>>({});
	// Snapshot of the draft as last loaded or saved. Comparing against it is
	// what makes "unsaved changes" reliable without threading a flag through
	// every input on the form.
	let baseline = $state("");

	let fingerprint = $derived(
		draft ? JSON.stringify($state.snapshot(draft)) + JSON.stringify($state.snapshot(secretValues)) : "",
	);
	let dirty = $derived(draft != null && fingerprint !== baseline);
	let canSave = $derived(draft != null && draft.meta.name.trim() !== "" && dirty && !saving);

	async function load(selectPath?: string) {
		try {
			entries = await api.listEnvironments(rootPath);
			activeId = (await api.getActiveEnvironment(rootPath))?.meta.id ?? null;
			const wanted = selectPath ?? draft?.path ?? entries[0]?.path;
			await beginEdit(entries.find((e) => e.path === wanted) ?? entries[0] ?? null);
		} catch (e) {
			reportError($t("env.loadFailed"), e);
		} finally {
			loading = false;
		}
	}
	load();

	async function beginEdit(entry: EnvironmentEntry | null) {
		if (!entry) {
			draft = null;
			secretValues = {};
			baseline = "";
			return;
		}
		// Must go through $state.snapshot: the entry is a reactive proxy and
		// structuredClone throws DataCloneError on proxies.
		draft = structuredClone($state.snapshot(entry)) as EnvironmentEntry;
		const values: Record<string, string> = {};
		for (const v of draft.variables) {
			if (v.secret) values[v.id] = (await api.revealSecret(workspacePath, v.id)) ?? "";
		}
		secretValues = values;
		// Only now: the secrets that just loaded are part of the unchanged state.
		baseline = JSON.stringify($state.snapshot(draft)) + JSON.stringify(values);
	}

	async function selectEnvironment(entry: EnvironmentEntry) {
		if (entry.path === draft?.path) return;
		if (dirty) {
			const proceed = await confirmAction(
				$t("env.switchConfirm", { name: draft?.meta.name ?? "" }),
				{ title: $t("common.unsavedChanges"), confirmLabel: $t("env.switchConfirmLabel") },
			);
			if (!proceed) return;
		}
		await beginEdit(entry);
	}

	async function createEnvironment() {
		const name = await promptForText($t("env.newEnvironment"), $t("env.namePlaceholder"), $t("env.newEnvironment"));
		if (!name) return;
		try {
			const created = await api.createEnvironment(rootPath, name, scope);
			// A scope with nothing selected resolves no variables at all, so
			// the first environment created becomes the active one. An
			// existing choice is left alone.
			if (!(await api.getActiveEnvironment(rootPath))) {
				await api.setActiveEnvironment(rootPath, created.meta.id);
			}
			requestEnvironmentsRefresh();
			await load(created.path);
		} catch (e) {
			reportError($t("env.createFailed"), e);
		}
	}

	async function deleteEnvironment() {
		const target = draft;
		if (!target) return;
		const confirmed = await confirmAction(
			$t("env.deleteConfirm", { name: target.meta.name }),
			{ title: $t("env.deleteConfirmTitle"), confirmLabel: $t("common.delete"), danger: true },
		);
		if (!confirmed) return;
		try {
			await api.deleteEnvironment(workspacePath, target.path);
			// Nothing is selected any more, so let load() fall back to the
			// first remaining environment instead of the deleted path.
			draft = null;
			requestEnvironmentsRefresh();
			await load();
		} catch (e) {
			reportError($t("env.deleteFailed"), e);
		}
	}

	function addVariable() {
		if (!draft) return;
		const id = newId();
		draft.variables = [...draft.variables, { id, key: "", value: "", enabled: true, secret: false }];
		secretValues[id] = "";
	}

	function removeVariable(i: number) {
		if (!draft) return;
		draft.variables = draft.variables.filter((_, idx) => idx !== i);
	}

	async function save() {
		if (!draft) return;
		saving = true;
		try {
			const plain = structuredClone($state.snapshot(draft)) as EnvironmentEntry;
			await Promise.all(
				plain.variables
					.filter((v) => v.secret)
					.map((v) => api.setSecret(workspacePath, v.id, secretValues[v.id] ?? "")),
			);
			const saved = await api.saveEnvironment(plain.path, {
				meta: plain.meta,
				variables: plain.variables.map((v) => (v.secret ? { ...v, value: "" } : v)),
			});
			// A renamed environment moves to a new file, so both the list and
			// the draft have to pick up the path the save landed on.
			entries = entries.map((e) => (e.path === plain.path ? saved : e));
			draft = { ...saved, variables: plain.variables };
			baseline = JSON.stringify($state.snapshot(draft)) + JSON.stringify($state.snapshot(secretValues));
			requestEnvironmentsRefresh();
		} catch (e) {
			reportError($t("env.saveFailed"), e);
		} finally {
			saving = false;
		}
	}

	async function requestClose() {
		if (dirty) {
			const proceed = await confirmAction($t("env.closeConfirm"), {
				title: $t("common.unsavedChanges"),
				confirmLabel: $t("common.close"),
			});
			if (!proceed) return;
		}
		onClose();
	}

	let heading = $derived(
		scope === "global" ? $t("env.headingGlobal", { title }) : $t("env.headingCollection", { title }),
	);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && requestClose()} />

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => {
		// Only a click on the backdrop itself closes; clicks inside the
		// dialog bubble up here but must be ignored.
		if (e.target === e.currentTarget) requestClose();
	}}
>
	<div class="modal" role="dialog" tabindex="-1" aria-modal="true" aria-label={heading}>
		<h2>{heading}</h2>

		<div class="layout">
			<aside class="list">
				{#each entries as entry (entry.path)}
					<button class="env" class:selected={entry.path === draft?.path} onclick={() => selectEnvironment(entry)}>
						<span class="env-name">{entry.meta.name}</span>
						{#if entry.meta.id === activeId}
							<span class="active-mark" title={$t("env.active")}>✓</span>
						{/if}
						{#if entry.path === draft?.path && dirty}
							<span class="dot" title={$t("env.unsaved")}>●</span>
						{/if}
					</button>
				{/each}
				{#if entries.length === 0 && !loading}
					<p class="hint">{$t("common.empty")}</p>
				{/if}
				<button class="add-env" onclick={createEnvironment}>{$t("env.add")}</button>
			</aside>

			<section class="editor">
				{#if loading}
					<p class="hint">{$t("common.loading")}</p>
				{:else if draft}
					<div class="name-field">
						<label>
							<span>{$t("common.name")}</span>
							<input bind:value={draft.meta.name} placeholder={$t("env.namePlaceholder")} />
						</label>
						<button class="delete-env" onclick={deleteEnvironment}>{$t("env.deleteEnvironment")}</button>
					</div>

					<div class="variables">
						{#each draft.variables as v, i (v.id)}
							<div class="var-row">
								<input
									type="checkbox"
									checked={v.enabled}
									onchange={(e) => (v.enabled = (e.target as HTMLInputElement).checked)}
								/>
								<input
									class="mono"
									class:invalid={v.key !== "" && !isValidVariableName(v.key)}
									placeholder={$t("kv.keyPlaceholder")}
									title={$t("env.variableNameTitle")}
									value={v.key}
									oninput={(e) => {
										// Names outside this set never resolve at send time, so
										// disallowed characters are dropped as they are typed.
										const cleaned = sanitizeVariableName((e.target as HTMLInputElement).value);
										(e.target as HTMLInputElement).value = cleaned;
										v.key = cleaned;
									}}
								/>
								{#if v.secret}
									<input
										class="mono"
										type="password"
										placeholder={$t("env.secretPlaceholder")}
										value={secretValues[v.id] ?? ""}
										oninput={(e) => (secretValues[v.id] = (e.target as HTMLInputElement).value)}
									/>
								{:else}
									<input class="mono" placeholder={$t("kv.valuePlaceholder")} bind:value={v.value} />
								{/if}
								<label class="secret-toggle">
									<input
										type="checkbox"
										checked={v.secret}
										onchange={(e) => {
											const nowSecret = (e.target as HTMLInputElement).checked;
											if (nowSecret) secretValues[v.id] = secretValues[v.id] ?? v.value;
											else v.value = secretValues[v.id] ?? v.value;
											v.secret = nowSecret;
										}}
									/>
									{$t("env.secret")}
								</label>
								<button class="remove" title={$t("env.removeVariable")} onclick={() => removeVariable(i)}>×</button>
							</div>
						{/each}
						<button class="add" onclick={addVariable}>{$t("env.addVariable")}</button>
					</div>
				{:else}
					<p class="hint">{$t("env.noEnvironments")}</p>
				{/if}
			</section>
		</div>

		<div class="actions">
			{#if dirty}<span class="unsaved">{$t("env.unsaved")}</span>{/if}
			<button onclick={requestClose}>{$t("common.close")}</button>
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
		z-index: 10;
	}
	.modal {
		background: var(--modal-bg, #fff);
		color: inherit;
		border-radius: 10px;
		padding: 1.2em;
		width: min(52em, 92vw);
		max-height: 82vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
	}
	h2 {
		margin: 0 0 0.8em;
		font-size: 1.1em;
	}
	.layout {
		display: grid;
		grid-template-columns: 12em 1fr;
		gap: 1em;
		flex: 1;
		min-height: 14em;
	}
	.list {
		display: flex;
		flex-direction: column;
		gap: 0.2em;
		overflow-y: auto;
		border-right: 1px solid rgba(127, 127, 127, 0.25);
		padding-right: 0.6em;
	}
	.env {
		display: flex;
		align-items: center;
		gap: 0.4em;
		text-align: left;
		background: none;
		border: none;
		padding: 0.35em 0.5em;
		border-radius: 4px;
		cursor: pointer;
		color: inherit;
		font-size: 0.9em;
	}
	.env-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.env:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.env.selected {
		background: rgba(57, 108, 216, 0.2);
	}
	.active-mark {
		margin-left: auto;
		color: #2e9e5b;
		font-size: 0.8em;
	}
	.dot {
		margin-left: auto;
		color: #a37c00;
		font-size: 0.7em;
	}
	.add-env {
		margin-top: 0.3em;
		background: none;
		border: 1px dashed rgba(127, 127, 127, 0.5);
		cursor: pointer;
		font-size: 0.85em;
		color: inherit;
	}
	.editor {
		overflow-y: auto;
		min-height: 0;
	}
	.name-field {
		display: flex;
		align-items: center;
		gap: 0.5em;
		margin-bottom: 0.8em;
	}
	.name-field label {
		display: flex;
		align-items: center;
		gap: 0.5em;
		flex: 1;
		min-width: 0;
	}
	.name-field span {
		font-size: 0.8em;
		opacity: 0.6;
	}
	.name-field input {
		flex: 1;
		min-width: 0;
	}
	.delete-env {
		background: none;
		border: 1px solid rgba(209, 68, 60, 0.5);
		color: #d1443c;
		cursor: pointer;
		font-size: 0.8em;
		white-space: nowrap;
	}
	.delete-env:hover {
		background: rgba(209, 68, 60, 0.1);
	}
	.variables {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}
	.var-row {
		display: flex;
		align-items: center;
		gap: 0.4em;
	}
	.var-row input.invalid {
		border-color: #d1443c;
	}
	.var-row input.mono {
		flex: 1;
		min-width: 0;
		font-family: ui-monospace, monospace;
	}
	.secret-toggle {
		display: flex;
		align-items: center;
		gap: 0.2em;
		font-size: 0.75em;
		white-space: nowrap;
		opacity: 0.7;
	}
	.remove {
		background: none;
		border: none;
		cursor: pointer;
		opacity: 0.5;
		font-size: 1.1em;
	}
	.add {
		align-self: flex-start;
		margin-top: 0.4em;
		background: none;
		border: 1px dashed rgba(127, 127, 127, 0.5);
		cursor: pointer;
		font-size: 0.85em;
		color: inherit;
	}
	.hint {
		opacity: 0.6;
		font-size: 0.9em;
	}
	.actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.5em;
		margin-top: 1em;
	}
	.unsaved {
		margin-right: auto;
		font-size: 0.8em;
		color: #a37c00;
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
