<script lang="ts">
  import { CircleHelp } from "@lucide/svelte";
  import { mergeProps } from "bits-ui";
  import type { HTMLButtonAttributes } from "svelte/elements";

  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import { cn } from "$lib/utils.js";

  type Props = Omit<HTMLButtonAttributes, "children" | "title"> & {
    content: string;
    label?: string;
    side?: "top" | "right" | "bottom" | "left";
  };

  let { content, label = content, side = "top", class: className, ...restProps }: Props = $props();

  const buttonProps = $derived({
    type: "button" as const,
    "data-slot": "context-help",
    "data-context-help": "",
    class: cn(
      "inline-flex size-6 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors duration-150 ease-out hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none motion-reduce:transition-none",
      className,
    ),
    "aria-label": label,
    ...restProps,
  });
</script>

<Tooltip.Provider delayDuration={150}>
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        {@const mergedProps = mergeProps(buttonProps, props)}
        <button {...mergedProps}>
          <CircleHelp size={15} strokeWidth={1.8} aria-hidden="true" />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content {side} sideOffset={6} class="max-w-80 items-start text-pretty leading-5">
      {content}
    </Tooltip.Content>
  </Tooltip.Root>
</Tooltip.Provider>
