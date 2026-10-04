<script lang="ts">
	// Where the open request lives and what kind it is, shown in the top bar
	// rather than above the URL: there it shares a row the environment
	// pickers leave mostly empty, and the working area gets that row back.
	import { activeRequest, setProtocol } from "../../stores/activeRequest";
	import { activeCollection } from "../../stores/collectionTree";
	import { incognito } from "../../stores/incognito";
	import { activeResponses } from "../../stores/response";
	import { t } from "../../i18n";
	import ProtocolSwitch from "./ProtocolSwitch.svelte";

	// Same rule as the send button: a collection is needed only to resolve
	// its environment, and an incognito request has none by design.
	let canSend = $derived($incognito || $activeCollection != null);

	/// Breadcrumb of the open request: collection, the folders it sits in,
	/// then the request itself - derived from where the file lives on disk.
	/// Paths are Windows-style here, so both separators are handled. In
	/// incognito the top bar already carries the badge, so only the name is
	/// left to say.
	let breadcrumb = $derived.by(() => {
		const request = $activeRequest;
		const collection = $activeCollection;
		if (!request) return [] as string[];
		if ($incognito || !collection) return [request.request.meta.name];
		const relative = request.path.startsWith(collection.path)
			? request.path.slice(collection.path.length).replace(/^[\\/]+/, "")
			: "";
		const folders = relative
			.split(/[\\/]+/)
			.slice(0, -1)
			.filter(Boolean);
		return [collection.name, ...folders, request.request.meta.name];
	});
</script>

{#if $activeRequest}
	<div class="request-title">
		<span class="path" title={breadcrumb.join(" / ")}>
			{#each breadcrumb as part, i (i)}
				{#if i > 0}<span class="sep">/</span>{/if}
				<span class:name={i === breadcrumb.length - 1}>{part}</span>
			{/each}
		</span>
		{#if $activeRequest.dirty}<span class="dirty" title={$t("request.dirty")}></span>{/if}
		{#if !canSend}
			<span class="warn">{$t("request.noCollection")}</span>
		{/if}
		<ProtocolSwitch
			value={$activeRequest.request.meta.protocol}
			disabled={$activeResponses.loading}
			onChange={setProtocol}
		/>
	</div>
{/if}

<style>
	.request-title {
		display: flex;
		align-items: center;
		gap: 0.6em;
		min-width: 0;
	}
	/* One line that gives way from the left: the request's own name is the
	   part worth keeping when the bar runs out of room. */
	.path {
		display: flex;
		align-items: center;
		gap: 0.4em;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		color: var(--text-muted);
	}
	.path > span {
		overflow: hidden;
		text-overflow: ellipsis;
		flex-shrink: 1;
	}
	.path .sep {
		flex-shrink: 0;
		color: var(--line-strong);
	}
	.path .name {
		flex-shrink: 0;
		max-width: 100%;
		font-weight: 600;
		color: var(--text);
	}
	.dirty {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
		background: var(--warn);
	}
	.warn {
		flex-shrink: 0;
		font-size: var(--fs-sm);
		color: var(--danger);
	}
</style>
