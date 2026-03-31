<script lang="ts">
  import {
    getQueue,
    removeFromQueue,
    clearCompleted,
    renderSingle,
    renderAll,
    reorderQueue,
    getPendingCount,
  } from "$lib/stores/renderQueue.svelte";
  import type { RenderEntry } from "$lib/types/game";
  import RenderPlaybackModal from "./RenderPlaybackModal.svelte";

  let queue = $derived(getQueue());
  let pendingCount = $derived(getPendingCount());
  let rendering = $state(false);
  let activeRender = $state<RenderEntry | null>(null);
  let dragIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  // Group items by game
  let grouped = $derived.by(() => {
    const groups: { gameName: string; gameDir: string; items: typeof queue }[] = [];
    for (const item of queue) {
      const existing = groups.find((g) => g.gameDir === item.gameDir);
      if (existing) {
        existing.items.push(item);
      } else {
        groups.push({
          gameName: item.gameName,
          gameDir: item.gameDir,
          items: [item],
        });
      }
    }
    return groups;
  });

  async function handleRenderAll() {
    rendering = true;
    await renderAll();
    rendering = false;
  }

  async function handleRenderSingle(id: string) {
    rendering = true;
    await renderSingle(id);
    rendering = false;
  }

  function statusColor(status: string): string {
    switch (status) {
      case "pending": return "text-text-muted";
      case "rendering": return "text-secondary";
      case "done": return "text-green-400";
      case "error": return "text-red-400";
      default: return "text-text-muted";
    }
  }

  function statusDot(status: string): string {
    switch (status) {
      case "pending": return "bg-text-muted";
      case "rendering": return "bg-secondary animate-pulse";
      case "done": return "bg-green-500";
      case "error": return "bg-red-500";
      default: return "bg-text-muted";
    }
  }

  function handleDragStart(index: number) { dragIndex = index; }
  function handleDragOver(index: number) {
    if (dragIndex === null || dragIndex === index) return;
    dragOverIndex = index;
  }
  function handleDragEnd() {
    if (dragIndex !== null && dragOverIndex !== null && dragIndex !== dragOverIndex) {
      reorderQueue(dragIndex, dragOverIndex);
    }
    dragIndex = null;
    dragOverIndex = null;
  }
</script>

<div class="space-y-5">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-bold">Render Queue</h2>
    <div class="flex gap-2">
      {#if queue.some((q) => q.status === "done" || q.status === "error")}
        <button
          class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
          onclick={clearCompleted}
        >
          Clear Completed
        </button>
      {/if}
      {#if pendingCount > 0}
        <button
          class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          disabled={rendering}
          onclick={handleRenderAll}
        >
          {rendering ? "Rendering..." : `Render All (${pendingCount})`}
        </button>
      {/if}
    </div>
  </div>

  {#if queue.length === 0}
    <div class="text-center py-16">
      <p class="text-text-muted text-sm">No items in queue.</p>
      <p class="text-text-muted text-xs mt-2">Add clips from the clip review panel using "Add to Queue".</p>
    </div>
  {:else}
    {#each grouped as group}
      <div>
        <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">
          {group.gameName}
        </h3>
        <div class="space-y-1">
          {#each group.items as item (item.id)}
            {@const globalIndex = queue.indexOf(item)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="flex items-center gap-3 px-3 py-2.5 bg-surface rounded-lg border transition-colors"
              class:border-secondary={dragOverIndex === globalIndex}
              class:border-border={dragOverIndex !== globalIndex}
              draggable={item.status === "pending" ? "true" : "false"}
              ondragstart={() => handleDragStart(globalIndex)}
              ondragover={(e) => { e.preventDefault(); handleDragOver(globalIndex); }}
              ondragend={handleDragEnd}
            >
              <!-- Status dot -->
              <span class="w-2 h-2 rounded-full shrink-0 {statusDot(item.status)}"></span>

              <!-- Clip info -->
              <div class="flex-1 min-w-0">
                <div class="text-sm font-medium truncate">{item.clipName}</div>
                <div class="flex items-center gap-2 mt-0.5">
                  {#each item.profiles as profile}
                    <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{profile.profile_name}</span>
                  {/each}
                  {#if item.concatOutput && item.profiles.length > 1}
                    <span class="text-[10px] text-text-muted">concat</span>
                  {/if}
                </div>
              </div>

              <!-- Status text -->
              <span class="text-xs {statusColor(item.status)} shrink-0">
                {item.status}
                {#if item.status === "error" && item.error}
                  <span class="block text-[10px] max-w-32 truncate" title={item.error}>{item.error}</span>
                {/if}
              </span>

              <!-- Actions -->
              <div class="flex items-center gap-1 shrink-0">
                {#if item.status === "pending"}
                  <button
                    class="px-2 py-1 text-xs text-text-muted hover:text-text bg-bg rounded transition-colors disabled:opacity-50"
                    disabled={rendering}
                    onclick={() => handleRenderSingle(item.id)}
                  >Render</button>
                {/if}
                {#if item.status === "done" && item.results && item.results.length > 0}
                  <button
                    class="px-2 py-1 text-xs text-secondary hover:text-text bg-bg rounded transition-colors"
                    onclick={() => activeRender = item.results![0]}
                  >View</button>
                {/if}
                <button
                  class="px-1.5 py-1 text-text-muted hover:text-accent text-xs transition-colors"
                  onclick={() => removeFromQueue(item.id)}
                  title="Remove"
                >&times;</button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>

{#if activeRender}
  <RenderPlaybackModal render={activeRender} onClose={() => activeRender = null} />
{/if}
