<script lang="ts">
	import { t } from "../../i18n";
	import type { SubtreeActivity } from "../../stores/response";

	// `group` is a folder or collection row standing in for the requests
	// inside it, so it gets wording about what's in there rather than about
	// itself.
	let { activity, group = false }: { activity: SubtreeActivity; group?: boolean } = $props();

	let record = $derived(activity.unseen);
	// A stream that ended by itself has no `outcome`, only the head it opened
	// with.
	let head = $derived(record?.outcome ?? record?.stream ?? null);
	let failed = $derived(record != null && (record.error != null || (head?.status ?? 0) >= 400));
	let doneTitle = $derived.by(() => {
		if (!record) return "";
		if (group) return $t(failed ? "activity.groupUnseenFailed" : "activity.groupUnseen");
		if (head?.status != null && record.error == null)
			return $t("activity.done", { status: head.status, statusText: head.status_text });
		return $t("activity.failed");
	});
</script>

{#if activity.running}
	<span class="indicator spinner" title={$t(group ? "activity.groupRunning" : "activity.running")}></span>
{:else if record}
	<span class="indicator dot" class:failed title={doneTitle}></span>
{/if}

<style>
	.indicator {
		margin-left: auto;
		flex-shrink: 0;
	}
	/* A filled dot marks a result that landed while the user was on another
	   request and hasn't been opened since. */
	.dot {
		width: 0.5em;
		height: 0.5em;
		border-radius: 50%;
		background: var(--ok);
	}
	.dot.failed {
		background: var(--danger);
	}
	.spinner {
		width: 0.7em;
		height: 0.7em;
		border: 2px solid var(--line-strong);
		border-top-color: var(--accent-text);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation-duration: 2.5s;
		}
	}
</style>
