<script lang="ts" module>
	import { tv, type VariantProps } from "tailwind-variants";

	export const sidebarDesktopMenuButtonVariants = tv({
		base: "group/menu-button flex min-h-(--sidebar-item-height) w-full items-center gap-2 overflow-hidden rounded-lg p-2 text-left text-sm outline-none transition-[background-color,color,box-shadow,transform] duration-150 ease-out motion-reduce:transition-none hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:bg-sidebar-accent focus-visible:text-sidebar-accent-foreground focus-visible:underline focus-visible:underline-offset-4 active:translate-y-px active:bg-sidebar-accent disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground [&_svg]:size-4 [&_svg]:shrink-0 [&>span:last-child]:truncate",
		variants: {
			variant: {
				default: "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
				outline:
					"bg-background shadow-[0_0_0_1px_var(--sidebar-border)] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground hover:shadow-[0_0_0_1px_var(--sidebar-accent)]",
			},
			size: {
				default: "h-8 text-sm",
				sm: "h-7 text-xs",
				lg: "h-(--sidebar-item-height) text-sm",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	});

	export type SidebarDesktopMenuButtonVariant = VariantProps<
		typeof sidebarDesktopMenuButtonVariants
	>["variant"];
	export type SidebarDesktopMenuButtonSize = VariantProps<
		typeof sidebarDesktopMenuButtonVariants
	>["size"];
</script>

<script lang="ts">
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLButtonAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		class: className,
		children,
		variant = "default",
		size = "default",
		isActive = false,
		type = "button",
		...restProps
	}: WithElementRef<HTMLButtonAttributes, HTMLButtonElement> & {
		isActive?: boolean;
		variant?: SidebarDesktopMenuButtonVariant;
		size?: SidebarDesktopMenuButtonSize;
	} = $props();
</script>

<button
	bind:this={ref}
	{type}
	class={cn(sidebarDesktopMenuButtonVariants({ variant, size }), className)}
	data-slot="sidebar-menu-button"
	data-sidebar="menu-button"
	data-size={size}
	data-active={isActive ? "true" : undefined}
	{...restProps}
>
	{@render children?.()}
</button>
