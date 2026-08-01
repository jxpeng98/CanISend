<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { cn, type WithElementRef } from "$lib/utils.js";

  type Props = WithElementRef<Omit<HTMLAttributes<HTMLElement>, "title">> & {
    eyebrow: string;
    title: string;
    description?: string;
    actions?: Snippet;
  };

  let {
    ref = $bindable(null),
    class: className,
    eyebrow,
    title,
    description,
    actions,
    ...restProps
  }: Props = $props();
</script>

<header
  bind:this={ref}
  data-slot="page-header"
  class={cn("page-header pb-1", className)}
  {...restProps}
>
  <div data-slot="page-intro" class="min-w-0 max-w-4xl">
    <Badge variant="secondary" class="mb-2 px-2.5 py-0.5">{eyebrow}</Badge>
    <h1 class="page-title text-balance">{title}</h1>
    {#if description}
      <p class="mt-2 max-w-3xl text-pretty text-sm leading-5 text-muted-foreground">
        {description}
      </p>
    {/if}
  </div>

  {#if actions}
    <div
      data-slot="page-actions"
      class="flex min-w-0 flex-wrap items-center gap-2 lg:justify-end"
    >
      {@render actions()}
    </div>
  {/if}
</header>
