<script lang="ts">
	import type { SubtreeActivity } from "../../stores/response";

	// `group` is a folder or collection row standing in for the requests
	// inside it, so it gets wording about what's in there rather than about
	// itself.
	let { activity, group = false }: { activity: SubtreeActivity; group?: boolean } = $props();

	let record = $derived(activity.unseen);
	let failed = $derived(record != null && (record.error != null || (record.outcome?.status ?? 0) >= 400));
	let doneTitle = $derived.by(() => {
		if (!record) return "";
		if (group) return failed ? "Внутри есть новый ответ с ошибкой" : "Внутри есть новый ответ";
		if (record.outcome) return `Запрос выполнен: ${record.outcome.status} ${record.outcome.status_text}`;
		return "Запрос завершился ошибкой";
	});
</script>

{#if activity.running}
	<span class="indicator spinner" title={group ? "Внутри выполняется запрос" : "Запрос выполняется"}></span>
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
		background: #2e9e5b;
	}
	.dot.failed {
		background: #d1443c;
	}
	.spinner {
		width: 0.7em;
		height: 0.7em;
		border: 2px solid rgba(127, 127, 127, 0.35);
		border-top-color: #396cd8;
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
