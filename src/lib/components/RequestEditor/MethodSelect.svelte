<script lang="ts">
	// The method picker: the shared drop-down, set in the mono face and
	// coloured with the method palette, so a method looks the same here, in
	// its own list and in the collection tree.
	import type { HttpMethod } from "../../bindings/types";
	import { t } from "../../i18n";
	import { HTTP_METHODS, methodColor } from "../../ui/methods";
	import Select from "../common/Select.svelte";

	let { value, onChange }: { value: HttpMethod; onChange: (method: HttpMethod) => void } = $props();

	const options = HTTP_METHODS.map((method) => ({ value: method, label: method, color: methodColor(method) }));
</script>

<Select
	class="method-select"
	variant="bare"
	mono
	{value}
	{options}
	ariaLabel={$t("request.methodAria")}
	onChange={(method) => onChange(method as HttpMethod)}
/>
