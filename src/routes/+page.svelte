<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Settings from "$lib/Settings.svelte";
  import Overlay from "$lib/Overlay.svelte";

  const label = getCurrentWindow().label;
</script>

{#if label === "settings"}
  <Settings />
{:else if label === "overlay"}
  <Overlay />
{:else}
  <p>Unknown window: {label}</p>
{/if}
