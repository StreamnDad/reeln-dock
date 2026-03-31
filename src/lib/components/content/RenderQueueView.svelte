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
  import type { QueueItem } from "$lib/types/queue";
  import { editingQueueItem } from "$lib/stores/navigation";
  import RenderPlaybackModal from "./RenderPlaybackModal.svelte";

  let queue = $derived(getQueue());
  let pendingCount = $derived(getPendingCount());
  let rendering = $state(false);
  let activeRender = $state<RenderEntry | null>(null);
  let expandedId = $state<string | null>(null);
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

  // Detect duplicate clip paths among pending items
  let duplicateClips = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const item of queue) {
      if (item.status === "pending") {
        counts.set(item.clipPath, (counts.get(item.clipPath) ?? 0) + 1);
      }
    }
    return new Set([...counts.entries()].filter(([, c]) => c > 1).map(([p]) => p));
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

  function handleEdit(item: QueueItem) {
    editingQueueItem.set({
      gameDir: item.gameDir,
      eventId: item.eventId,
      mode: item.mode,
      profiles: item.profiles,
      concatOutput: item.concatOutput,
      overrides: item.overrides,
      pluginProfile: item.pluginProfile,
      scorer: item.scorer,
      assist1: item.assist1,
      assist2: item.assist2,
    });
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

  function statusColor(status: string): string {
    switch (status) {
      case "pending": return "text-text-muted";
      case "rendering": return "text-secondary";
      case "done": return "text-green-400";
      case "error": return "text-red-400";
      default: return "text-text-muted";
    }
  }

  function hasOverrides(item: QueueItem): boolean {
    if (!item.overrides) return false;
    const o = item.overrides;
    return !!(o.crop_mode || (o.scale != null && o.scale !== 1) || (o.speed != null && o.speed !== 1) || o.smart || o.pad_color || o.zoom_frames);
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
        >Clear Completed</button>
      {/if}
      {#if pendingCount > 0}
        <button
          class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          disabled={rendering}
          onclick={handleRenderAll}
        >{rendering ? "Rendering..." : `Render All (${pendingCount})`}</button>
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
        <div class="space-y-1.5">
          {#each group.items as item (item.id)}
            {@const globalIndex = queue.indexOf(item)}
            {@const isExpanded = expandedId === item.id}
            {@const isDuplicate = duplicateClips.has(item.clipPath) && item.status === "pending"}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="bg-surface rounded-lg border transition-colors"
              class:border-secondary={dragOverIndex === globalIndex}
              class:border-yellow-600={isDuplicate && dragOverIndex !== globalIndex}
              class:border-border={!isDuplicate && dragOverIndex !== globalIndex}
              draggable={item.status === "pending" ? "true" : "false"}
              ondragstart={() => handleDragStart(globalIndex)}
              ondragover={(e) => { e.preventDefault(); handleDragOver(globalIndex); }}
              ondragend={handleDragEnd}
            >
              <!-- Summary row -->
              <div
                class="flex items-center gap-3 px-3 py-2.5 cursor-pointer"
                onclick={() => expandedId = isExpanded ? null : item.id}
              >
                <span class="transition-transform text-[10px] text-text-muted" class:rotate-90={isExpanded}>&#9654;</span>
                <span class="w-2 h-2 rounded-full shrink-0 {statusDot(item.status)}"></span>

                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-medium truncate">{item.clipName}</span>
                    {#if isDuplicate}
                      <span class="px-1 py-0.5 bg-yellow-900/30 text-yellow-400 text-[9px] rounded" title="This clip appears multiple times in the queue">DUP</span>
                    {/if}
                  </div>
                  <div class="flex items-center gap-2 mt-0.5">
                    {#each item.profiles as profile}
                      <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{profile.profile_name}</span>
                    {/each}
                    {#if item.mode}
                      <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.mode}</span>
                    {/if}
                  </div>
                </div>

                <span class="text-xs {statusColor(item.status)} shrink-0">
                  {item.status}
                  {#if item.status === "error" && item.error}
                    <span class="block text-[10px] max-w-32 truncate" title={item.error}>{item.error}</span>
                  {/if}
                </span>

                <!-- Actions -->
                <div class="flex items-center gap-1 shrink-0" onclick={(e) => e.stopPropagation()}>
                  {#if item.status === "pending"}
                    <button
                      class="px-2 py-1 text-xs text-text-muted hover:text-text bg-bg rounded transition-colors"
                      onclick={() => handleEdit(item)}
                    >Edit</button>
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

              <!-- Expandable details -->
              {#if isExpanded}
                <div class="px-3 pb-3 pt-0 border-t border-border">
                  <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs pt-2.5">
                    <span class="text-text-muted">Mode</span>
                    <span>{item.mode === "apply" ? "Apply (full-frame)" : "Short (crop/scale)"}</span>

                    <span class="text-text-muted">Profiles</span>
                    <span>{item.profiles.map((p) => p.profile_name).join(", ")}</span>

                    {#if item.profiles.length > 1}
                      <span class="text-text-muted">Concatenate</span>
                      <span>{item.concatOutput ? "Yes" : "No"}</span>
                    {/if}

                    {#if hasOverrides(item)}
                      <span class="text-text-muted">Overrides</span>
                      <div class="flex flex-wrap gap-1">
                        {#if item.overrides?.crop_mode}
                          <span class="px-1.5 py-0.5 bg-bg rounded">crop: {item.overrides.crop_mode}</span>
                        {/if}
                        {#if item.overrides?.scale != null && item.overrides.scale !== 1}
                          <span class="px-1.5 py-0.5 bg-bg rounded">scale: {item.overrides.scale}</span>
                        {/if}
                        {#if item.overrides?.speed != null && item.overrides.speed !== 1}
                          <span class="px-1.5 py-0.5 bg-bg rounded">speed: {item.overrides.speed}x</span>
                        {/if}
                        {#if item.overrides?.smart}
                          <span class="px-1.5 py-0.5 bg-bg rounded">smart zoom</span>
                        {/if}
                        {#if item.overrides?.zoom_frames}
                          <span class="px-1.5 py-0.5 bg-bg rounded">zoom: {item.overrides.zoom_frames}f</span>
                        {/if}
                        {#if item.overrides?.pad_color}
                          <span class="px-1.5 py-0.5 bg-bg rounded">pad: {item.overrides.pad_color}</span>
                        {/if}
                      </div>
                    {/if}

                    {#if item.scorer}
                      <span class="text-text-muted">Scorer</span>
                      <span>{item.scorer}</span>
                    {/if}
                    {#if item.assist1}
                      <span class="text-text-muted">Assist 1</span>
                      <span>{item.assist1}</span>
                    {/if}
                    {#if item.assist2}
                      <span class="text-text-muted">Assist 2</span>
                      <span>{item.assist2}</span>
                    {/if}

                    {#if item.pluginProfile}
                      <span class="text-text-muted">Plugin Profile</span>
                      <span>{item.pluginProfile}</span>
                    {/if}

                    <span class="text-text-muted">Clip</span>
                    <span class="truncate" title={item.clipPath}>{item.clipPath}</span>
                  </div>
                </div>
              {/if}
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
