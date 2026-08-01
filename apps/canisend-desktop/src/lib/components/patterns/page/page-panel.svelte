<script lang="ts" module>
  import { tv, type VariantProps } from "tailwind-variants";

  export const pagePanelVariants = tv({
    base: "min-w-0 rounded-lg border p-[var(--density-panel-padding)] transition-colors duration-150 ease-out motion-reduce:transition-none",
    variants: {
      tone: {
        default: "bg-background",
        muted: "bg-muted/20",
        accent: "bg-accent/25",
        primary: "border-primary/35 bg-primary/5",
      },
    },
    defaultVariants: {
      tone: "default",
    },
  });

  export type PagePanelTone = VariantProps<typeof pagePanelVariants>["tone"];
</script>

<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";

  import { cn, type WithElementRef } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    children,
    tone = "default",
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & { tone?: PagePanelTone } = $props();
</script>

<div
  bind:this={ref}
  data-slot="page-panel"
  data-tone={tone}
  class={cn(pagePanelVariants({ tone }), className)}
  {...restProps}
>
  {@render children?.()}
</div>
