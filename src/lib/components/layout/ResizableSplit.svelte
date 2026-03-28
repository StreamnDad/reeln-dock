<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    left: Snippet;
    right: Snippet;
  }

  let { left, right }: Props = $props();

  let sidebarWidth = $state(280);
  let dragging = $state(false);

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const newWidth = Math.min(400, Math.max(200, e.clientX));
    sidebarWidth = newWidth;
  }

  function onPointerUp() {
    dragging = false;
  }
</script>

<div class="flex flex-1 overflow-hidden">
  <div class="shrink-0 overflow-y-auto overflow-x-hidden" style="width: {sidebarWidth}px">
    {@render left()}
  </div>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="w-1 shrink-0 cursor-col-resize transition-colors hover:bg-secondary"
    class:bg-secondary={dragging}
    class:bg-border={!dragging}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
  ></div>

  <div class="flex-1 overflow-y-auto">
    {@render right()}
  </div>
</div>
