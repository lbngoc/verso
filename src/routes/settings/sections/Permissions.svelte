<script lang="ts">
  import PermissionGuide from "$lib/components/PermissionGuide.svelte";
  import type { PermissionStatus } from "$lib/ipc";

  // Settings never opens with the "after an update" reason: Setup takes over in that case (spec 010
  // contracts/permission-step.md). The parent keeps the status for the sidebar and its events.
  let {
    permission,
    ongranted,
  }: {
    permission: PermissionStatus;
    ongranted: (status: PermissionStatus) => void;
  } = $props();

  let status = $state<PermissionStatus>("denied");
  $effect(() => {
    status = permission;
  });
</script>

<PermissionGuide
  kind="accessibility"
  reason="missing"
  bind:status
  ongranted={() => ongranted("granted")}
/>
