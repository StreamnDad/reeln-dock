<script lang="ts">
  import { getDragState } from "$lib/stores/drag.svelte";

  let state = $derived(getDragState());
</script>

{#if state?.active}
  <div
    class="fixed z-[100] pointer-events-none px-4 py-2 bg-surface border border-secondary rounded-lg shadow-lg text-sm text-text opacity-90 whitespace-nowrap"
    style="left: {state.x + 12}px; top: {state.y - 16}px;"
  >
    {state.label}
  </div>
{/if}
