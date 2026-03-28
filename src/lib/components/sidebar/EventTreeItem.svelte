<script lang="ts">
  import type { GameEvent } from "$lib/types/game";

  interface Props {
    event: GameEvent;
  }

  let { event }: Props = $props();
</script>

<div class="flex items-center gap-2 px-2 py-1 text-xs text-text-muted">
  <span class="font-medium text-text">{event.event_type || "clip"}</span>
  {#if event.player}
    <span>- {event.player}</span>
  {/if}
</div>
