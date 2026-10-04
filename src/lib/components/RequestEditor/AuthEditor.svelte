<script lang="ts">
	import type { AuthSpec } from "../../bindings/types";
	import { t } from "../../i18n";
	import VariableInput from "../common/VariableInput.svelte";
	import Select from "../common/Select.svelte";

	let { auth, onChange }: { auth: AuthSpec; onChange: (auth: AuthSpec) => void } = $props();

	function setType(type: AuthSpec["type"]) {
		if (type === "none") onChange({ type: "none" });
		else if (type === "bearer") onChange({ type: "bearer", token: "" });
		else onChange({ type: "basic", username: "", password: "" });
	}
</script>

<div class="auth-editor">
	<Select
		class="auth-type"
		value={auth.type}
		ariaLabel={$t("auth.typeAria")}
		options={[
			{ value: "none", label: $t("auth.none") },
			{ value: "bearer", label: $t("auth.bearer") },
			{ value: "basic", label: $t("auth.basic") },
		]}
		onChange={(type) => setType(type as AuthSpec["type"])}
	/>

	{#if auth.type === "bearer"}
		<dl class="auth-params">
			<dt>{$t("auth.token")}</dt>
			<dd>
				<VariableInput
					mono
					ariaLabel="Bearer token"
					placeholder={$t("auth.tokenPlaceholder")}
					value={auth.token}
					onChange={(token) => onChange({ type: "bearer", token })}
				/>
			</dd>
		</dl>
	{:else if auth.type === "basic"}
		<dl class="auth-params">
			<dt>{$t("auth.login")}</dt>
			<dd>
				<VariableInput
					placeholder="admin"
					value={auth.username}
					onChange={(username) => onChange({ type: "basic", username, password: auth.password })}
				/>
			</dd>
			<dt>{$t("auth.password")}</dt>
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
	.auth-editor :global(.auth-type) {
		align-self: flex-start;
		min-width: 12em;
	}
	.auth-params {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.35em 0.9em;
		margin: 0;
		font-size: var(--fs-sm);
		margin-bottom: 1.2em;
		align-items: center;
	}
</style>
