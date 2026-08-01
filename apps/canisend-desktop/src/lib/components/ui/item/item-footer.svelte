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
	data-slot="item-footer"
	class={cn("min-w-0 flex-wrap gap-2 flex basis-full items-center justify-between", className)}
	{...restProps}
>
	{@render children?.()}
</div>
