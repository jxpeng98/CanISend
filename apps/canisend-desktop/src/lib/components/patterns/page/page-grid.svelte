<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  import { cn, type WithElementRef } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
  bind:this={ref}
  data-slot="page-grid"
  class={cn("grid min-w-0 gap-[var(--density-section-gap)]", className)}
  {...restProps}
>
  {@render children?.()}
</div>
