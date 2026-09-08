<script lang="ts">
	import type { AuthSpec } from "../../bindings/types";
	import VariableInput from "../common/VariableInput.svelte";

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
		<VariableInput
			mono
			ariaLabel="Bearer token"
			placeholder="{'{{token}}'} или значение токена"
			value={auth.token}
			onChange={(token) => onChange({ type: "bearer", token })}
		/>
	{:else if auth.type === "basic"}
		<div class="basic-row">
			<VariableInput
				placeholder="username"
				value={auth.username}
				onChange={(username) => onChange({ type: "basic", username, password: auth.password })}
			/>
			<VariableInput
				type="password"
				placeholder="password"
				value={auth.password}
				onChange={(password) => onChange({ type: "basic", username: auth.username, password })}
			/>
		</div>
	{/if}
</div>

<style>
	.auth-editor {
		display: flex;
		flex-direction: column;
		gap: 0.6em;
	}
	.auth-editor select {
		align-self: flex-start;
		min-width: 12em;
	}
	.basic-row {
		display: flex;
		gap: 0.5em;
	}
</style>
