<script lang="ts">
  import { MoreHorizontal } from "@lucide/svelte";
  import { mergeProps } from "bits-ui";
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  import { buttonVariants } from "$lib/components/ui/button/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { cn } from "$lib/utils.js";

  type Props = Omit<HTMLButtonAttributes, "children"> & {
    label: string;
    children?: Snippet;
    showLabel?: boolean;
    contentClass?: string;
  };

  let {
    label,
    children,
    showLabel = false,
    contentClass,
    class: className,
    disabled,
    ...restProps
  }: Props = $props();

  const triggerProps = $derived({
    type: "button" as const,
    class: cn(
      buttonVariants({
        variant: "outline",
        size: showLabel ? "desktop" : "icon-desktop",
      }),
      showLabel ? "page-action" : "",
      className,
    ),
    "aria-label": label,
    title: showLabel ? undefined : label,
    disabled,
    ...restProps,
  });
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      {@const mergedProps = mergeProps(triggerProps, props)}
      <button {...mergedProps}>
        <MoreHorizontal size={16} strokeWidth={1.8} aria-hidden="true" />
        {#if showLabel}
          <span>{label}</span>
        {/if}
      </button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end" class={cn("w-56", contentClass)}>
    {@render children?.()}
  </DropdownMenu.Content>
</DropdownMenu.Root>
