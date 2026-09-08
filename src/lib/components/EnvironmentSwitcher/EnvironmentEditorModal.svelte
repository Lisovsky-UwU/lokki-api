<script lang="ts">
	import type { EnvironmentEntry } from "../../bindings/types";
	import { api } from "../../api/client";

	let {
		environment,
		workspacePath,
		onClose,
		onSaved,
	}: {
		environment: EnvironmentEntry;
		workspacePath: string;
		onClose: () => void;
		onSaved: (env: EnvironmentEntry) => void;
	} = $props();

	let draft = $state<EnvironmentEntry>(structuredClone(environment));
	let saving = $state(false);
	// Secret variable values never travel through `draft.variables[i].value`
	// (that gets written to the on-disk, potentially-synced .env.toml) —
	// they live only here, loaded from/written to the local SecretStore.
	let secretValues = $state<Record<string, string>>({});

	async function loadSecrets() {
		for (const v of draft.variables) {
			if (v.secret) {
				secretValues[v.id] = (await api.revealSecret(workspacePath, v.id)) ?? "";
			}
		}
	}
	loadSecrets();

	function addVariable() {
		const id = crypto.randomUUID();
		draft.variables = [...draft.variables, { id, key: "", value: "", enabled: true, secret: false }];
		secretValues[id] = "";
	}

	function removeVariable(i: number) {
		draft.variables = draft.variables.filter((_, idx) => idx !== i);
	}

	async function save() {
		saving = true;
		try {
			const toPersist: EnvironmentEntry = {
				...draft,
				variables: draft.variables.map((v) => (v.secret ? { ...v, value: "" } : v)),
			};
			await Promise.all(
				draft.variables.filter((v) => v.secret).map((v) => api.setSecret(workspacePath, v.id, secretValues[v.id] ?? "")),
			);
			const saved = await api.saveEnvironment(draft.path, toPersist);
			onSaved({ ...saved, path: draft.path });
			onClose();
		} finally {
			saving = false;
		}
	}
</script>

<div class="backdrop" role="presentation" onclick={onClose} onkeydown={(e) => e.key === "Escape" && onClose()}>
	<div class="modal" role="dialog" aria-modal="true" aria-label={draft.meta.name} onclick={(e) => e.stopPropagation()}>
		<h2>{draft.meta.name}</h2>
		<div class="variables">
			{#each draft.variables as v, i (v.id)}
				<div class="var-row">
					<input
						type="checkbox"
						checked={v.enabled}
						onchange={(e) => (v.enabled = (e.target as HTMLInputElement).checked)}
					/>
					<input class="mono" placeholder="key" bind:value={v.key} />
					{#if v.secret}
						<input
							class="mono"
							type="password"
							placeholder="secret value"
							value={secretValues[v.id] ?? ""}
							oninput={(e) => (secretValues[v.id] = (e.target as HTMLInputElement).value)}
						/>
					{:else}
						<input class="mono" placeholder="value" bind:value={v.value} />
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
						secret
					</label>
					<button class="remove" onclick={() => removeVariable(i)}>×</button>
				</div>
			{/each}
			<button class="add" onclick={addVariable}>+ переменная</button>
		</div>
		<div class="actions">
			<button onclick={onClose}>Отмена</button>
			<button class="primary" onclick={save} disabled={saving}>{saving ? "Сохранение..." : "Сохранить"}</button>
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
		width: min(32em, 90vw);
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
	}
	h2 {
		margin: 0 0 0.8em;
		font-size: 1.1em;
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
		border-radius: 6px;
		padding: 0.3em 0.8em;
		cursor: pointer;
		color: inherit;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5em;
		margin-top: 1em;
	}
	.primary {
		background: #396cd8;
		color: white;
		border: 1px solid #396cd8;
		border-radius: 6px;
		padding: 0.4em 1.2em;
		cursor: pointer;
	}
</style>
