<script lang="ts">
  import { CheckCircle2, FileCheck2, GitCompareArrows, ShieldCheck, Target } from "@lucide/svelte";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import type { IntakeReviewReadModel } from "$lib/bridge";
  import type { Messages } from "$lib/i18n";

  type Props = {
    copy: Messages;
    review: IntakeReviewReadModel;
  };

  let { copy, review }: Props = $props();

  function formatBytes(value: number): string {
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
    return `${(value / (1024 * 1024)).toFixed(1)} MB`;
  }
</script>

<div class="space-y-[var(--density-section-gap)]">
  <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
    <div class="min-w-0 rounded-lg border bg-background p-3">
      <p
        class="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
      >
        <FileCheck2 size={14} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeSourceIdentity}
      </p>
      <p class="mt-2 text-sm font-semibold">
        {copy.intakeSourceKindLabel[review.source.kind]}
      </p>
      <p class="mt-1 truncate text-[11px] text-muted-foreground" title={review.source.locator}>
        {review.source.locator}
      </p>
    </div>

    <div class="rounded-lg border bg-background p-3">
      <p
        class="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
      >
        <CheckCircle2 size={14} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeDetectedType}
      </p>
      <p class="mt-2 break-words text-sm font-semibold">{review.source.detected_type}</p>
      {#if review.source.sha256}
        <p
          class="mt-1 truncate font-mono text-[10px] text-muted-foreground"
          title={review.source.sha256}
        >
          SHA-256 · {review.source.sha256}
        </p>
      {/if}
    </div>

    <div class="rounded-lg border bg-background p-3">
      <p
        class="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
      >
        <Target size={14} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeTarget}
      </p>
      <p class="mt-2 text-sm font-semibold">
        {copy.intakeTargetKindLabel[review.target.kind]}
      </p>
      <p class="mt-1 truncate text-[11px] text-muted-foreground" title={review.target.label}>
        {review.target.label}
      </p>
    </div>

    <div class="rounded-lg border bg-background p-3">
      <p
        class="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
      >
        <GitCompareArrows size={14} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeDuplicateSignal}
      </p>
      <p class="mt-2 text-sm font-semibold">
        {copy.intakeDuplicateStateLabel[review.duplicate_signal.state]}
      </p>
      {#if review.duplicate_signal.count}
        <p class="mt-1 text-[11px] text-muted-foreground">
          {review.duplicate_signal.count}
          {copy.items}
        </p>
      {/if}
    </div>
  </div>

  <div class="flex flex-wrap gap-2">
    {#if review.extraction.original_bytes !== null}
      <Badge variant="outline">
        {copy.sourceSize} · {formatBytes(review.extraction.original_bytes)}
      </Badge>
    {/if}
    {#if review.extraction.normalized_text_bytes !== null}
      <Badge variant="outline">
        {copy.intakeNormalizedText} · {formatBytes(review.extraction.normalized_text_bytes)}
      </Badge>
    {/if}
    {#if review.extraction.normalized_lines !== null}
      <Badge variant="outline">
        {copy.normalizedLines} · {review.extraction.normalized_lines}
      </Badge>
    {/if}
    {#if review.extraction.pdf_pages !== null}
      <Badge variant="outline">
        {copy.pdfPages} · {review.extraction.pdf_pages}
      </Badge>
    {/if}
    <Badge variant="outline">
      {copy.acceptedRows} · {review.extraction.accepted_items}
    </Badge>
    {#if review.extraction.rejected_items}
      <Badge variant="outline">
        {copy.rejectedRows} · {review.extraction.rejected_items}
      </Badge>
    {/if}
  </div>

  <div class="grid gap-3 lg:grid-cols-2">
    <div class="rounded-lg border bg-muted/20 p-3">
      <p class="flex items-center gap-1.5 text-xs font-semibold">
        <ShieldCheck size={15} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeConsentBoundary}
      </p>
      <p class="mt-1 text-xs leading-5 text-muted-foreground">
        {copy.intakeConsentScopeLabel[review.required_consent]}
      </p>
    </div>
    <div class="rounded-lg border bg-muted/20 p-3">
      <p class="flex items-center gap-1.5 text-xs font-semibold">
        <FileCheck2 size={15} strokeWidth={1.8} aria-hidden="true" />
        {copy.intakeCommitBoundary}
      </p>
      <p class="mt-1 text-xs leading-5 text-muted-foreground">
        {copy.intakeCommitBoundaryLabel[review.commit_boundary]}
      </p>
    </div>
  </div>

  <div>
    <p class="text-xs font-medium text-muted-foreground">{copy.intendedChanges}</p>
    <div class="mt-2 grid gap-2 lg:grid-cols-2">
      {#each review.intended_mutations as mutation (mutation.subject + mutation.action)}
        <div class="rounded-lg border p-3">
          <p class="text-xs font-semibold">{mutation.action}</p>
          <p class="mt-1 text-xs leading-5 text-muted-foreground">
            {mutation.description}
          </p>
        </div>
      {/each}
    </div>
  </div>
</div>
