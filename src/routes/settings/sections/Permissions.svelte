<script lang="ts">
  import Card from "$lib/components/Card.svelte";
  import PermissionRow from "$lib/components/PermissionRow.svelte";
  import type { SettingsProfile } from "$lib/ipc";

  // Settings › Permissions (macOS): one row per permission instead of one tall guide. Screen
  // Recording is listed only while the screen translate module is on (spec 004 FR-014).
  let { settings }: { settings: SettingsProfile } = $props();
</script>

<Card>
  <PermissionRow kind="accessibility" />
  {#if settings.ocr.enabled}
    <PermissionRow kind="screen_recording" />
  {/if}
</Card>
