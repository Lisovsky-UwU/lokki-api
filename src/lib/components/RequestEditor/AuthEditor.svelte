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
		<option value="none">Без авторизации</option>
		<option value="bearer">Bearer-токен</option>
		<option value="basic">Basic-авторизация</option>
	</select>

	{#if auth.type === "bearer"}
		<dl class="auth-params">
			<dt>Токен</dt>
			<dd>
				<VariableInput
					mono
					ariaLabel="Bearer token"
					placeholder="{'{{token}}'} или значение токена"
					value={auth.token}
					onChange={(token) => onChange({ type: "bearer", token })}
				/>
			</dd>
		</dl>
	{:else if auth.type === "basic"}
		<dl class="auth-params">
			<dt>Логин</dt>
			<dd>
				<VariableInput
					placeholder="admin"
					value={auth.username}
					onChange={(username) => onChange({ type: "basic", username, password: auth.password })}
				/>
			</dd>
			<dt>Пароль</dt>
			<dd>
				<VariableInput
					type="password"
					placeholder="********"
					value={auth.password}
					onChange={(password) => onChange({ type: "basic", username: auth.username, password })}
				/>
			</dd>
		</dl>
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
	.auth-params {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.35em 0.9em;
		margin: 0;
		font-size: 0.85em;
		margin-bottom: 1.2em;
		align-items: center;
	}
</style>
