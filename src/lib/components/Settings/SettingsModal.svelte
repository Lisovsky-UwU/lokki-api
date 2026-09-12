<script lang="ts">
	import { base } from "$app/paths";
	import { api } from "../../api/client";
	import type { AppInfo } from "../../bindings/types";
	import { copyText } from "../../ui/clipboard";
	import { reportError } from "../../ui/notices";

	let { onClose }: { onClose: () => void } = $props();

	const DESCRIPTION = "Локальный, файловый клиент для тестирования API — лёгкая альтернатива Postman и Insomnia.";
	const UNKNOWN = "неизвестно";

	let info = $state<AppInfo | null>(null);
	let copied = $state(false);

	async function load() {
		try {
			info = await api.appInfo();
		} catch (e) {
			reportError("Не удалось получить сведения о сборке", e);
		}
	}
	load();

	function osName(os: string): string {
		if (os === "windows") return "Windows";
		if (os === "macos") return "macOS";
		if (os === "linux") return "Linux";
		return os;
	}

	// One list feeds both the rendered rows and the copied text, so what the
	// user pastes into a bug report is exactly what they were looking at.
	let rows = $derived<[string, string][]>(
		info
			? [
					["Версия", info.version],
					["Дата сборки", info.build_date],
					["Коммит", info.commit],
					["Операционная система", info.os ? `${osName(info.os)} (${info.arch})` : ""],
					["Rust", info.rust_version],
					["Tauri", info.tauri_version],
					["Node.js", info.node_version],
					["WebView", info.webview_version],
				]
			: [],
	);

	async function copyAll() {
		if (!info) return;
		const text = [
			info.name,
			"",
			...rows.map(([label, value]) => `${label}: ${value || UNKNOWN}`),
		].join("\n");
		await copyText(text);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => {
		// Only a click on the backdrop itself closes; clicks inside the
		// dialog bubble up here but must be ignored.
		if (e.target === e.currentTarget) onClose();
	}}
>
	<div class="modal" role="dialog" tabindex="-1" aria-modal="true" aria-label="Настройки">
		<h2>Настройки</h2>

		<div class="identity">
			<img class="logo" src="{base}/logo-256.png" alt="" />
			<div>
				<div class="app-name">{info?.name ?? "LokkiAPI"}</div>
				<p class="description">{DESCRIPTION}</p>
			</div>
		</div>

		<hr class="splitter"/>

		<div class="section-head">
			<h3>О программе</h3>
			<button onclick={copyAll} disabled={!info} class:copied={copied}>
				{copied ? "Скопировано" : "Скопировать"}
			</button>
		</div>

		{#if info}
			<dl class="facts">
				{#each rows as [label, value] (label)}
					<dt>{label}</dt>
					<dd class:unknown={!value}>{value || UNKNOWN}</dd>
				{/each}
			</dl>
		{:else}
			<p class="hint">Загрузка…</p>
		{/if}
		<hr class="splitter"/>
		<div class="section-head">
			<h3>GitHub</h3>
		</div>

		<dl class="facts">
			<dt>Автор</dt>
			<dd><a href="https://github.com/Lisovsky-UwU" target="_blank" rel="noopener">Lisovsky-UwU</a></dd>
			<dt>Репозиторий</dt>
			<dd><a href="https://github.com/Lisovsky-UwU/lokki-api" target="_blank" rel="noopener">lokki-api</a></dd>
		</dl>

		<div class="actions">
			<button onclick={onClose}>Закрыть</button>
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
	h3 {
		margin: 0;
		font-size: 0.75em;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		opacity: 0.7;
	}
	.identity {
		display: flex;
		align-items: center;
		gap: 0.9em;
		margin-bottom: 1.2em;
	}
	.logo {
		width: 56px;
		height: 56px;
		flex-shrink: 0;
	}
	.app-name {
		font-size: 1.15em;
		font-weight: 700;
	}
	.description {
		margin: 0.2em 0 0;
		font-size: 0.85em;
		opacity: 0.7;
	}
	.section-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.6em;
		margin: 1.2em 0 0.5em;
	}
	.section-head button {
		font-size: 0.8em;
		cursor: pointer;
	}
	/* Plain label/value list: a two-column grid keeps the values aligned
	   without turning it into a table. */
	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.35em 0.9em;
		margin: 0;
		font-size: 0.85em;
		margin-bottom: 1.2em;
	}
	dt {
		opacity: 0.6;
	}
	dd {
		margin: 0;
		font-family: ui-monospace, monospace;
		word-break: break-all;
	}
	dd.unknown {
		font-family: inherit;
		opacity: 0.5;
	}
	.hint {
		opacity: 0.6;
		font-size: 0.85em;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5em;
		margin-top: 1em;
	}
	.actions button {
		cursor: pointer;
	}
	.copied {
		font-size: 0.8em;
		background-color: var(--success-color, green);
	}
	.splitter {
		border: none;
		background: rgba(127, 127, 127, 0.25);
		height: 1px;
	}
</style>
