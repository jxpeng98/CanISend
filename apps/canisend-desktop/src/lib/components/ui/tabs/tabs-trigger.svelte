<script lang="ts">
	import { Tabs as TabsPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: TabsPrimitive.TriggerProps = $props();
</script>

<TabsPrimitive.Trigger
	bind:ref
	data-slot="tabs-trigger"
	class={cn(
		"min-h-(--tabs-height) min-w-0 gap-1.5 break-words rounded-md border border-transparent px-2.5 py-1 text-center text-sm font-medium leading-snug has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 group-data-[variant=default]/tabs-list:data-[state=active]:shadow-sm group-data-[variant=line]/tabs-list:data-[state=active]:shadow-none [&_svg:not([class*='size-'])]:size-4 focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:outline-ring text-foreground/65 hover:text-foreground dark:text-muted-foreground dark:hover:text-foreground relative inline-flex flex-1 items-center justify-center whitespace-normal transition-[background-color,border-color,color,box-shadow,transform] duration-200 ease-out motion-reduce:transition-none active:scale-[0.99] group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start focus-visible:ring-[3px] focus-visible:outline-1 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
		"group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-[state=active]:bg-transparent dark:group-data-[variant=line]/tabs-list:data-[state=active]:border-transparent dark:group-data-[variant=line]/tabs-list:data-[state=active]:bg-transparent",
		"group-data-[variant=default]/tabs-list:data-[state=active]:border-primary group-data-[variant=default]/tabs-list:data-[state=active]:bg-primary group-data-[variant=default]/tabs-list:data-[state=active]:text-primary-foreground dark:group-data-[variant=default]/tabs-list:data-[state=active]:bg-primary dark:group-data-[variant=default]/tabs-list:data-[state=active]:text-primary-foreground group-data-[variant=line]/tabs-list:data-[state=active]:text-foreground",
		"after:bg-foreground after:absolute after:opacity-0 after:transition-opacity group-data-[orientation=horizontal]/tabs:after:inset-x-0 group-data-[orientation=horizontal]/tabs:after:bottom-0 group-data-[orientation=horizontal]/tabs:after:h-0.5 group-data-[orientation=vertical]/tabs:after:inset-y-0 group-data-[orientation=vertical]/tabs:after:right-0 group-data-[orientation=vertical]/tabs:after:w-0.5 group-data-[variant=line]/tabs-list:data-[state=active]:after:opacity-100",
		className,
	)}
	{...restProps}
/>
