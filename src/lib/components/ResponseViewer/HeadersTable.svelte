<script lang="ts">
	// Response headers, shared by the request and the stream viewers.
	import { t } from "../../i18n";
	import type { KeyValue } from "../../bindings/types";

	let { headers }: { headers: KeyValue[] } = $props();
</script>

<div class="headers">
	<table>
		<thead>
			<tr>
				<th>{$t("response.headerName")}</th>
				<th>{$t("response.headerValue")}</th>
			</tr>
		</thead>
		<tbody>
			<!-- Keyed by position, not by name: HTTP allows a header to
			     repeat (Link, Set-Cookie), and each occurrence is its own
			     row. -->
			{#each headers as header, i (i)}
				<tr>
					<td class="header-key">{header.key}</td>
					<td class="header-value">{header.value}</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.headers {
		flex: 1;
		min-height: 0;
		overflow: auto;
		border: 1px solid var(--line-strong);
		border-radius: 6px;
	}
	.headers table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.headers th {
		position: sticky;
		top: 0;
		text-align: left;
		font-weight: 600;
		padding: 0.45em 0.6em;
		background: var(--surface-sunken);
		border-bottom: 1px solid var(--line-strong);
	}
	.headers td {
		padding: 0.4em 0.6em;
		border-bottom: 1px solid var(--line);
		vertical-align: top;
	}
	.headers tr:last-child td {
		border-bottom: none;
	}
	.header-key {
		font-weight: 600;
		white-space: nowrap;
		font-family: var(--font-mono);
	}
	.header-value {
		font-family: var(--font-mono);
		word-break: break-all;
	}
</style>
