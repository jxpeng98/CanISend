<script lang="ts">
  import * as Empty from "$lib/components/ui/empty/index.js";
  import { Spinner } from "$lib/components/ui/spinner/index.js";
  import { cn } from "$lib/utils.js";

  let {
    label,
    class: className,
  }: {
    label: string;
    class?: string;
  } = $props();
</script>

<Empty.Root class={cn("min-h-32", className)} role="status" aria-live="polite">
  <Empty.Header>
    <Empty.Media><Spinner class="size-5" /></Empty.Media>
    <Empty.Description>{label}</Empty.Description>
  </Empty.Header>
</Empty.Root>
