<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAttributes } from "svelte/elements";

  import ContextHelp from "$lib/components/patterns/ContextHelp.svelte";
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
    <div class="flex min-w-0 items-center gap-1.5">
      <h1 class="page-title min-w-0 text-balance">{title}</h1>
      {#if description}
        <ContextHelp content={description} side="bottom" />
      {/if}
    </div>
  </div>

  {#if actions}
    <div data-slot="page-actions" class="flex min-w-0 flex-wrap items-center gap-2 lg:justify-end">
      {@render actions()}
    </div>
  {/if}
</header>
