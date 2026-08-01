<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
	bind:this={ref}
	data-slot="field-group"
	class={cn(
		"min-w-0 gap-[var(--density-section-gap)] data-[slot=checkbox-group]:gap-3 *:data-[slot=field-group]:gap-3 group/field-group @container/field-group flex w-full flex-col",
		className
	)}
	{...restProps}
>
	{@render children?.()}
</div>
