<script lang="ts">
	import type { AuthSpec } from "../../bindings/types";

	let { auth, onChange }: { auth: AuthSpec; onChange: (auth: AuthSpec) => void } = $props();

	function setType(type: AuthSpec["type"]) {
		if (type === "none") onChange({ type: "none" });
		else if (type === "bearer") onChange({ type: "bearer", token: "" });
		else onChange({ type: "basic", username: "", password: "" });
	}
</script>

<div class="auth-editor">
	<select value={auth.type} onchange={(e) => setType((e.target as HTMLSelectElement).value as AuthSpec["type"])}>
		<option value="none">No Auth</option>
		<option value="bearer">Bearer Token</option>
		<option value="basic">Basic Auth</option>
	</select>

	{#if auth.type === "bearer"}
		<input
			class="mono"
			placeholder="{'{{token}}'} или значение токена"
			value={auth.token}
			oninput={(e) => onChange({ type: "bearer", token: (e.target as HTMLInputElement).value })}
		/>
	{:else if auth.type === "basic"}
		<div class="basic-row">
			<input
				placeholder="username"
				value={auth.username}
				oninput={(e) =>
					onChange({ type: "basic", username: (e.target as HTMLInputElement).value, password: auth.password })}
			/>
			<input
				type="password"
				placeholder="password"
				value={auth.password}
				oninput={(e) =>
					onChange({ type: "basic", username: auth.username, password: (e.target as HTMLInputElement).value })}
			/>
		</div>
	{/if}
</div>

<style>
	.auth-editor {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
		max-width: 28em;
	}
	.mono {
		font-family: ui-monospace, monospace;
	}
	.basic-row {
		display: flex;
		gap: 0.5em;
	}
	.basic-row input {
		flex: 1;
	}
</style>
