<script lang="ts">
	import { base } from "$app/paths";
	import { api } from "../../api/client";
	import type { AppInfo, RequestSettings } from "../../bindings/types";
	import { copyText } from "../../ui/clipboard";
	import { reportError } from "../../ui/notices";

	let { onClose }: { onClose: () => void } = $props();

	const DESCRIPTION = "Локальный, файловый клиент для тестирования API — лёгкая альтернатива Postman и Insomnia.";
	const UNKNOWN = "неизвестно";

	// Sections are listed in one place so adding the next one is a single
	// entry plus a branch in the pane below.
	const TABS = [
		{ id: "requests", label: "Запросы" },
		{ id: "about", label: "О программе" },
	] as const;
	type TabId = (typeof TABS)[number]["id"];
	let tab = $state<TabId>("requests");

	let info = $state<AppInfo | null>(null);
	let copied = $state(false);
	let settings = $state<RequestSettings | null>(null);

	async function load() {
		try {
			info = await api.appInfo();
		} catch (e) {
			reportError("Не удалось получить сведения о сборке", e);
		}
		try {
			settings = await api.getRequestSettings();
		} catch (e) {
			reportError("Не удалось загрузить настройки запросов", e);
		}
	}
	load();

	// Stored in milliseconds, edited in seconds: nobody reasons about a
	// timeout in milliseconds, and the extra precision only invites typos.
	function seconds(ms: number): number {
		return Math.round(ms / 100) / 10;
	}

	/// Saved on every change rather than behind a Save button: these are
	/// switches, and each takes effect on the next request.
	async function persist() {
		if (!settings) return;
		try {
			settings = await api.saveRequestSettings($state.snapshot(settings));
		} catch (e) {
			reportError("Не удалось сохранить настройки", e);
		}
	}

	function setTimeoutSeconds(field: "connect_timeout_ms" | "read_timeout_ms" | "total_timeout_ms", raw: string) {
		if (!settings) return;
		// Negative or unparseable input becomes 0, which is exactly what the
		// field hint promises: no limit.
		const value = Math.max(0, Number(raw.replace(",", ".")) || 0);
		settings[field] = Math.round(value * 1000);
		persist();
	}

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
		const text = [info.name, "", ...rows.map(([label, value]) => `${label}: ${value || UNKNOWN}`)].join("\n");
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

		<div class="layout">
			<nav class="tabs">
				{#each TABS as item (item.id)}
					<button class:selected={tab === item.id} onclick={() => (tab = item.id)}>{item.label}</button>
				{/each}
			</nav>

			<section class="pane">
				{#if tab === "requests"}
					{#if settings}
						<div class="settings-form">
							<label class="row check">
								<input type="checkbox" bind:checked={settings.verify_tls} onchange={persist} />
								<span>Проверять TLS-сертификаты</span>
							</label>
							{#if !settings.verify_tls}
								<p class="warning">
									Проверка отключена: принимается любой сертификат, включая подменённый. Только для тестовых стендов.
								</p>
							{/if}

							<label class="row">
								<span>Таймаут подключения, с</span>
								<input
									type="number"
									min="0"
									step="1"
									value={seconds(settings.connect_timeout_ms)}
									onchange={(e) => setTimeoutSeconds("connect_timeout_ms", (e.target as HTMLInputElement).value)}
								/>
							</label>
							<label class="row">
								<span>Таймаут чтения, с</span>
								<input
									type="number"
									min="0"
									step="1"
									value={seconds(settings.read_timeout_ms)}
									onchange={(e) => setTimeoutSeconds("read_timeout_ms", (e.target as HTMLInputElement).value)}
								/>
							</label>
							<label class="row">
								<span>Общий таймаут, с</span>
								<input
									type="number"
									min="0"
									step="1"
									value={seconds(settings.total_timeout_ms)}
									onchange={(e) => setTimeoutSeconds("total_timeout_ms", (e.target as HTMLInputElement).value)}
								/>
							</label>
							<p class="hint">
								Таймаут чтения — ожидание между частями ответа, общий — на весь запрос. 0 — без ограничения.
							</p>

							<label class="row check">
								<input type="checkbox" bind:checked={settings.follow_redirects} onchange={persist} />
								<span>Следовать перенаправлениям</span>
							</label>
							{#if settings.follow_redirects}
								<label class="row">
									<span>Максимум перенаправлений</span>
									<input
										type="number"
										min="0"
										step="1"
										value={settings.max_redirects}
										onchange={(e) => {
											if (!settings) return;
											settings.max_redirects = Math.max(
												0,
												Math.round(Number((e.target as HTMLInputElement).value) || 0),
											);
											persist();
										}}
									/>
								</label>
							{/if}

							<label class="row">
								<span>User-Agent</span>
								<input class="mono" placeholder="по умолчанию" bind:value={settings.user_agent} onchange={persist} />
							</label>
							<p class="hint">Настройки применяются со следующего запроса и хранятся на этом компьютере.</p>
						</div>
					{:else}
						<p class="hint">Загрузка…</p>
					{/if}
				{:else}
					<div class="identity">
						<img class="logo" src="{base}/logo-256.png" alt="" />
						<div>
							<div class="app-name">{info?.name ?? "LokkiAPI"}</div>
							<p class="description">{DESCRIPTION}</p>
						</div>
					</div>

					<div class="section-head">
						<h3>Сборка</h3>
						<button onclick={copyAll} disabled={!info} class:copied>
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

					<div class="section-head">
						<h3>GitHub</h3>
					</div>
					<dl class="facts">
						<dt>Автор</dt>
						<dd><a href="https://github.com/Lisovsky-UwU" target="_blank" rel="noopener">Lisovsky-UwU</a></dd>
						<dt>Репозиторий</dt>
						<dd><a href="https://github.com/Lisovsky-UwU/lokki-api" target="_blank" rel="noopener">lokki-api</a></dd>
					</dl>
				{/if}
			</section>
		</div>

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
		width: min(56em, 92vw);
		/* Fixed height rather than fit-to-content: switching tabs must not
		   make the dialog jump around, and there are more tabs to come. */
		height: min(38em, 84vh);
		display: flex;
		flex-direction: column;
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
	.layout {
		display: grid;
		grid-template-columns: 12em 1fr;
		gap: 1em;
		flex: 1;
		min-height: 0;
	}
	.tabs {
		display: flex;
		flex-direction: column;
		gap: 0.2em;
		overflow-y: auto;
		border-right: 1px solid rgba(127, 127, 127, 0.25);
		padding-right: 0.6em;
	}
	.tabs button {
		text-align: left;
		background: none;
		border: none;
		padding: 0.45em 0.6em;
		border-radius: 4px;
		cursor: pointer;
		color: inherit;
		font-size: 0.9em;
	}
	.tabs button:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.tabs button.selected {
		background: rgba(57, 108, 216, 0.2);
	}
	.pane {
		overflow-y: auto;
		min-height: 0;
		padding-right: 0.2em;
	}
	.identity {
		display: flex;
		align-items: center;
		gap: 0.9em;
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
	.section-head button.copied {
		border-color: #2e9e5b;
		color: #2e9e5b;
	}
	/* Plain label/value list: a two-column grid keeps the values aligned
	   without turning it into a table. */
	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.35em 0.9em;
		margin: 0;
		font-size: 0.85em;
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
	.settings-form {
		display: flex;
		flex-direction: column;
		gap: 0.5em;
		font-size: 0.9em;
		max-width: 34em;
	}
	.settings-form .row {
		display: flex;
		align-items: center;
		gap: 0.6em;
	}
	.settings-form .row > span {
		flex: 1;
		min-width: 0;
	}
	.settings-form .row.check > span {
		flex: none;
	}
	.settings-form .row input[type="number"] {
		width: 7em;
	}
	.settings-form .row input.mono {
		flex: 1;
		min-width: 0;
		font-family: ui-monospace, monospace;
	}
	.settings-form .hint {
		margin: 0;
	}
	.warning {
		margin: 0;
		color: #a37c00;
		background: rgba(163, 124, 0, 0.1);
		border-radius: 6px;
		padding: 0.4em 0.6em;
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
</style>
