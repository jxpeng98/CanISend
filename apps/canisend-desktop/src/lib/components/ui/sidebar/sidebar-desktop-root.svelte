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
  data-slot="sidebar"
  data-desktop-only="true"
  class={cn(
    "flex h-full w-(--sidebar-width) flex-col bg-sidebar text-sidebar-foreground",
    className,
  )}
  bind:this={ref}
  {...restProps}
>
  {@render children?.()}
</div>
