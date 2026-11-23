<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import PermissionRow from "$lib/components/PermissionRow.svelte";
  import type { SettingsProfile } from "$lib/ipc";

  // Settings › Permissions (macOS): one row per permission instead of one tall guide. Screen
  // Recording is listed only while the screen translate module is on (spec 004 FR-014).
  let { settings }: { settings: SettingsProfile } = $props();
</script>

<div class="rows">
  <Card>
    <PermissionRow kind="accessibility" />
  </Card>
  {#if settings.ocr.enabled}
    <Card>
      <PermissionRow kind="screen_recording" />
    </Card>
  {/if}
</div>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
</style>
