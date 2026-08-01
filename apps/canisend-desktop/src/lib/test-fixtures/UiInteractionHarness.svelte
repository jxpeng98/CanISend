<script lang="ts">
  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import * as NativeSelect from "$lib/components/ui/native-select/index.js";
  import * as Tabs from "$lib/components/ui/tabs/index.js";

  let selected = $state("ready");
  let confirmed = $state(0);
  let confirmationOpen = $state(false);
</script>

<label for="harness-status">Status</label>
<NativeSelect.Root id="harness-status" size="desktop" bind:value={selected}>
  <NativeSelect.Option value="ready">Ready</NativeSelect.Option>
  <NativeSelect.Option value="blocked">Blocked</NativeSelect.Option>
</NativeSelect.Root>
<output aria-label="Selected status">{selected}</output>

<Tabs.Root value="overview">
  <Tabs.List aria-label="Harness sections">
    <Tabs.Trigger value="overview">Overview</Tabs.Trigger>
    <Tabs.Trigger value="review">Review</Tabs.Trigger>
  </Tabs.List>
  <Tabs.Content value="overview">Overview panel</Tabs.Content>
  <Tabs.Content value="review">Review panel</Tabs.Content>
</Tabs.Root>

<Accordion.Root type="single">
  <Accordion.Item value="provenance">
    <Accordion.Trigger>Revision provenance</Accordion.Trigger>
    <Accordion.Content>Artifact r7 was verified locally.</Accordion.Content>
  </Accordion.Item>
</Accordion.Root>

<AlertDialog.Root bind:open={confirmationOpen}>
  <AlertDialog.Trigger>Remove managed files</AlertDialog.Trigger>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Remove managed files?</AlertDialog.Title>
      <AlertDialog.Description>This leaves user-modified files untouched.</AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
      <AlertDialog.Action
        variant="destructive"
        onclick={() => {
          confirmed += 1;
          confirmationOpen = false;
        }}
      >
        Confirm removal
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
<output aria-label="Confirm count">{confirmed}</output>
