<script lang="ts">
  import { cn, type WithElementRef } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    ref = $bindable(null),
    class: className,
    style,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
  data-slot="sidebar-wrapper"
  style={`--sidebar-width: 16rem; ${style ?? ""}`}
  class={cn("flex min-h-svh w-full bg-background", className)}
  bind:this={ref}
  {...restProps}
>
  {@render children?.()}
</div>
