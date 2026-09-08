<script lang="ts">
	import type { CollectionTreeNode } from "../../bindings/types";
	import TreeNode from "./TreeNode.svelte";
	import { activeRequest } from "../../stores/activeRequest";
	import { api } from "../../api/client";

	let { node }: { node: CollectionTreeNode } = $props();

	let expanded = $state(true);


	async function openRequest(path: string) {
		try {
			const request = await api.loadRequest(path);
			activeRequest.set({ path, request, dirty: false });
		} catch (e) {
			console.error("failed to load request", e);
		}
	}

	let currentPath = $derived($activeRequest?.path);
</script>

{#if node.kind === "Folder"}
	<div class="folder">
		<button class="folder-label" onclick={() => (expanded = !expanded)}>
			<span class="chevron" class:collapsed={!expanded}>▾</span>
			{node.name}
		</button>
		{#if expanded}
			<div class="children">
				{#each node.children as child (child.path)}
					<TreeNode node={child} />
				{/each}
			</div>
		{/if}
	</div>
{:else}
	<button
		class="request-label"
		class:active={currentPath === node.path}
		onclick={() => openRequest(node.path)}
	>
		<span class="method method-{(node.method ?? node.protocol).toLowerCase()}">{node.method ?? node.protocol}</span>
		{node.name}
	</button>
{/if}

<style>
	.folder-label,
	.request-label {
		display: flex;
		align-items: center;
		gap: 0.4em;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.3em 0.4em;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.9em;
		color: inherit;
	}
	.folder-label:hover,
	.request-label:hover {
		background: rgba(127, 127, 127, 0.15);
	}
	.request-label.active {
		background: rgba(57, 108, 216, 0.2);
	}
	.chevron {
		display: inline-block;
		transition: transform 0.15s;
	}
	.chevron.collapsed {
		transform: rotate(-90deg);
	}
	.children {
		padding-left: 1.1em;
	}
	.method {
		font-size: 0.7em;
		font-weight: 700;
		min-width: 2.8em;
	}
	.method-get {
		color: #2e9e5b;
	}
	.method-post {
		color: #a37c00;
	}
	.method-put,
	.method-patch {
		color: #c26b0f;
	}
	.method-delete {
		color: #d1443c;
	}
</style>
