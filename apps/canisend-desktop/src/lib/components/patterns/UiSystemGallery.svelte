<script lang="ts">
  import {
    CircleCheck,
    Info,
    LoaderCircle,
    Search,
    TriangleAlert,
  } from "@lucide/svelte";

  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import * as Alert from "$lib/components/ui/alert/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import { Checkbox } from "$lib/components/ui/checkbox/index.js";
  import * as Empty from "$lib/components/ui/empty/index.js";
  import * as Field from "$lib/components/ui/field/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import * as Item from "$lib/components/ui/item/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";
  import { Spinner } from "$lib/components/ui/spinner/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";

  let dark = $state(false);
  let compact = $state(false);
  let consent = $state(false);
  let reducedMotion = $state(false);
  let status = $state("ready");
</script>

<svelte:head>
  <title>CanISend UI system gallery</title>
</svelte:head>

<main
  class:dark
  class:reduce-motion={reducedMotion}
  class="desktop-shell min-h-screen bg-background p-5 text-foreground lg:p-8"
  data-density={compact ? "compact" : "comfortable"}
  data-testid="ui-system-gallery"
>
  <div class="mx-auto max-w-6xl space-y-[var(--density-section-gap)]">
    <header class="flex flex-col justify-between gap-[var(--density-section-gap)] border-b pb-5 lg:flex-row lg:items-end">
      <div>
        <Badge variant="secondary">Development only</Badge>
        <h1 class="mt-3 text-page-title font-semibold tracking-tight">
          CanISend UI system
        </h1>
        <p class="mt-2 max-w-2xl text-sm text-muted-foreground">
          Registry primitives and shared states · 组件与共享状态画廊
        </p>
      </div>
      <div class="flex flex-wrap gap-2" aria-label="Gallery appearance">
        <Button size="desktop" variant="outline" onclick={() => (dark = !dark)}>
          {dark ? "Light" : "Dark"}
        </Button>
        <Button size="desktop" variant="outline" onclick={() => (compact = !compact)}>
          {compact ? "Comfortable" : "Compact"}
        </Button>
        <Button
          size="desktop"
          variant="outline"
          onclick={() => (reducedMotion = !reducedMotion)}
        >
          {reducedMotion ? "Enable motion" : "Reduce motion"}
        </Button>
      </div>
    </header>

    <section class="grid gap-[var(--shell-block-gap)] lg:grid-cols-2" aria-label="Controls">
      <Card.Root>
        <Card.Header>
          <Card.Title>Controls</Card.Title>
          <Card.Description>Shared desktop sizes and semantic variants.</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-[var(--density-section-gap)]">
          <div class="flex flex-wrap gap-2">
            <Button size="desktop">Primary</Button>
            <Button size="desktop" variant="secondary">Secondary</Button>
            <Button size="desktop" variant="outline">Outline</Button>
            <Button size="desktop" variant="destructive">Destructive</Button>
          </div>
          <div class="flex flex-wrap gap-2">
            <Badge>Primary</Badge>
            <Badge variant="success">Success</Badge>
            <Badge variant="warning">Warning</Badge>
            <Badge variant="info">Information</Badge>
            <Badge variant="destructive">Error</Badge>
          </div>
          <Field.Group>
            <Field.Field>
              <Field.Label for="gallery-search">Search</Field.Label>
              <div class="relative">
                <Search
                  class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
                  aria-hidden="true"
                />
                <Input id="gallery-search" class="pl-9" placeholder="Search applications" />
              </div>
              <Field.Description>Labels, help text, and controls stay associated.</Field.Description>
            </Field.Field>
            <Field.Field>
              <Field.Label for="gallery-status">Status</Field.Label>
              <NativeSelect.Root id="gallery-status" size="desktop" bind:value={status}>
                <NativeSelect.Option value="ready">Ready</NativeSelect.Option>
                <NativeSelect.Option value="blocked">Blocked</NativeSelect.Option>
                <NativeSelect.Option value="complete">Complete</NativeSelect.Option>
              </NativeSelect.Root>
            </Field.Field>
            <Field.Field>
              <Field.Label for="gallery-notes">Notes</Field.Label>
              <Textarea id="gallery-notes" placeholder="Add review notes" />
            </Field.Field>
          </Field.Group>
          <div class="flex flex-wrap items-center gap-[var(--density-section-gap)]">
            <Field.Field orientation="horizontal">
              <Checkbox id="gallery-consent" bind:checked={consent} />
              <Field.Label for="gallery-consent">Private-read consent</Field.Label>
            </Field.Field>
            <Field.Field orientation="horizontal">
              <Switch id="gallery-switch" bind:checked={reducedMotion} />
              <Field.Label for="gallery-switch">Reduce motion</Field.Label>
            </Field.Field>
          </div>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title>Feedback</Card.Title>
          <Card.Description>Persistent, announced states with semantic color.</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-3">
          <Alert.Root variant="success">
            <CircleCheck aria-hidden="true" />
            <Alert.Title>Application package is ready</Alert.Title>
            <Alert.Description>All required revisions are current.</Alert.Description>
          </Alert.Root>
          <Alert.Root variant="warning">
            <TriangleAlert aria-hidden="true" />
            <Alert.Title>Deadline needs confirmation</Alert.Title>
            <Alert.Description>Review the source before exporting.</Alert.Description>
          </Alert.Root>
          <Alert.Root variant="info">
            <Info aria-hidden="true" />
            <Alert.Title>Local-first boundary</Alert.Title>
            <Alert.Description>No application is submitted by CanISend.</Alert.Description>
          </Alert.Root>
          <Alert.Root variant="destructive">
            <TriangleAlert aria-hidden="true" />
            <Alert.Title>Workspace check failed</Alert.Title>
            <Alert.Description>Restore from a verified backup before writing.</Alert.Description>
          </Alert.Root>
          <div class="space-y-2 rounded-lg border p-3" role="status" aria-live="polite">
            <div class="flex items-center justify-between text-sm">
              <span class="inline-flex items-center gap-2">
                <Spinner /> Loading current workflow
              </span>
              <span class="font-medium">64%</span>
            </div>
            <Progress value={64} aria-label="Gallery progress" />
          </div>
        </Card.Content>
      </Card.Root>
    </section>

    <section class="grid gap-[var(--shell-block-gap)] lg:grid-cols-2" aria-label="Content states">
      <Card.Root>
        <Card.Header>
          <Card.Title>Items</Card.Title>
          <Card.Description>Consistent list rows with content and actions.</Card.Description>
        </Card.Header>
        <Card.Content>
          <Item.Group>
            <Item.Root variant="outline">
              <Item.Media variant="icon"><CircleCheck aria-hidden="true" /></Item.Media>
              <Item.Content>
                <Item.Title>Lecturer in Economics</Item.Title>
                <Item.Description>University of Example · 14 days remaining</Item.Description>
              </Item.Content>
              <Item.Actions><Badge variant="success">On track</Badge></Item.Actions>
            </Item.Root>
            <Item.Root variant="outline">
              <Item.Media variant="icon"><TriangleAlert aria-hidden="true" /></Item.Media>
              <Item.Content>
                <Item.Title>Research Fellow</Item.Title>
                <Item.Description>Evidence confirmation is required.</Item.Description>
              </Item.Content>
              <Item.Actions><Button size="desktop" variant="outline">Review</Button></Item.Actions>
            </Item.Root>
          </Item.Group>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header>
          <Card.Title>Empty and disclosure</Card.Title>
          <Card.Description>Reusable no-data and progressive-disclosure states.</Card.Description>
        </Card.Header>
        <Card.Content class="space-y-[var(--density-section-gap)]">
          <Empty.Root class="border border-dashed">
            <Empty.Header>
              <Empty.Media variant="icon"><LoaderCircle aria-hidden="true" /></Empty.Media>
              <Empty.Title>No review findings</Empty.Title>
              <Empty.Description>Run review after accepting the current document set.</Empty.Description>
            </Empty.Header>
            <Empty.Content><Button size="desktop">Run review</Button></Empty.Content>
          </Empty.Root>
          <Accordion.Root type="single" value="details">
            <Accordion.Item value="details">
              <Accordion.Trigger level={2}>Artifact details</Accordion.Trigger>
              <Accordion.Content>
                Revision identities and provenance remain visible without exposing document bodies.
              </Accordion.Content>
            </Accordion.Item>
          </Accordion.Root>
        </Card.Content>
      </Card.Root>
    </section>
  </div>
</main>
