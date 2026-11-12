<script lang="ts">
  import { ICONS, type IconName } from "$lib/icons";

  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="currentColor"
  aria-hidden="true"
  focusable="false"
>
  <path d={ICONS[name]} />
</svg>

<style>
  .icon {
    display: block;
    flex: none;
  }
</style>
