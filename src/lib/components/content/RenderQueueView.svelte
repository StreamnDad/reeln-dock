<script lang="ts">
  import { openFile } from "$lib/ipc/media";
  import {
    getStageItems,
    removeFromStage,
    clearStageErrors,
    renderSingle,
    renderAll,
    reorderStage,
    getPendingStageCount,
    getCliQueueItems,
    isCliQueueLoading,
    refreshAllCliQueues,
    removeCliItem,
    publishAllCliItems,
  } from "$lib/stores/renderQueue.svelte";
  import type { RenderStageItem, CliQueueItem } from "$lib/types/queue";
  import { editingQueueItem } from "$lib/stores/navigation";
  import QueuePublishPanel from "./QueuePublishPanel.svelte";

  type Tab = "staging" | "publish";
  let activeTab = $state<Tab>("staging");

  let stageItems = $derived(getStageItems());
  let pendingCount = $derived(getPendingStageCount());
  let cliItems = $derived(getCliQueueItems());
  let cliLoading = $derived(isCliQueueLoading());
  let rendering = $state(false);
  let expandedStageId = $state<string | null>(null);
  let expandedCliId = $state<string | null>(null);
  let dragIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  // Count unpublished CLI items
  let unpublishedCliCount = $derived(
    cliItems.filter((i) => i.status === "rendered").length,
  );

  // Refresh CLI queue when switching to publish tab.
  // Only tracks activeTab — does NOT read cliItems to avoid infinite re-trigger.
  $effect(() => {
    if (activeTab === "publish") {
      refreshAllCliQueues();
    }
  });

  // Group stage items by game
  let stageGrouped = $derived.by(() => {
    const groups: { gameName: string; gameDir: string; items: RenderStageItem[] }[] = [];
    for (const item of stageItems) {
      const existing = groups.find((g) => g.gameDir === item.gameDir);
      if (existing) {
        existing.items.push(item);
      } else {
        groups.push({ gameName: item.gameName, gameDir: item.gameDir, items: [item] });
      }
    }
    return groups;
  });

  // Group CLI items by game
  let cliGrouped = $derived.by(() => {
    const groups: { gameName: string; gameDir: string; items: CliQueueItem[] }[] = [];
    for (const item of cliItems) {
      const gameName = `${item.home_team} vs ${item.away_team}` + (item.date ? ` (${item.date})` : "");
      const existing = groups.find((g) => g.gameDir === item.game_dir);
      if (existing) {
        existing.items.push(item);
      } else {
        groups.push({ gameName, gameDir: item.game_dir, items: [item] });
      }
    }
    return groups;
  });

  // Detect duplicate clip paths among pending stage items
  let duplicateClips = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const item of stageItems) {
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

  function handleEditStage(item: RenderStageItem) {
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

  function stageStatusDot(status: string): string {
    switch (status) {
      case "pending": return "bg-text-muted";
      case "rendering": return "bg-secondary animate-pulse";
      case "error": return "bg-red-500";
      default: return "bg-text-muted";
    }
  }

  function stageStatusColor(status: string): string {
    switch (status) {
      case "pending": return "text-text-muted";
      case "rendering": return "text-secondary";
      case "error": return "text-red-400";
      default: return "text-text-muted";
    }
  }

  function cliStatusDot(status: string): string {
    switch (status) {
      case "rendered": return "bg-blue-400";
      case "publishing": return "bg-yellow-400 animate-pulse";
      case "published": return "bg-green-500";
      case "partial": return "bg-orange-400";
      case "failed": return "bg-red-500";
      default: return "bg-text-muted";
    }
  }

  function cliStatusColor(status: string): string {
    switch (status) {
      case "rendered": return "text-blue-400";
      case "publishing": return "text-yellow-400";
      case "published": return "text-green-400";
      case "partial": return "text-orange-400";
      case "failed": return "text-red-400";
      default: return "text-text-muted";
    }
  }

  function hasOverrides(item: RenderStageItem): boolean {
    if (!item.overrides) return false;
    const o = item.overrides;
    return !!(o.crop_mode || (o.scale != null && o.scale !== 1) || (o.speed != null && o.speed !== 1) || o.smart || o.pad_color || o.zoom_frames);
  }

  function openDebugIndex(gameDir: string) {
    const debugPath = `${gameDir}/debug/index.html`;
    openFile(debugPath).catch(() => {});
  }

  function handleDragStart(index: number) { dragIndex = index; }
  function handleDragOver(index: number) {
    if (dragIndex === null || dragIndex === index) return;
    dragOverIndex = index;
  }
  function handleDragEnd() {
    if (dragIndex !== null && dragOverIndex !== null && dragIndex !== dragOverIndex) {
      reorderStage(dragIndex, dragOverIndex);
    }
    dragIndex = null;
    dragOverIndex = null;
  }
</script>

<div class="space-y-4">
  <!-- Tabs -->
  <div class="flex items-center gap-1 border-b border-border">
    <button
      class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px flex items-center gap-1.5"
      class:border-primary={activeTab === "staging"}
      class:text-text={activeTab === "staging"}
      class:border-transparent={activeTab !== "staging"}
      class:text-text-muted={activeTab !== "staging"}
      onclick={() => activeTab = "staging"}
    >
      Pending Renders
      {#if pendingCount > 0}
        <span class="px-1.5 py-0.5 text-[10px] font-bold rounded-full bg-primary/30 text-text leading-none">{pendingCount}</span>
      {/if}
    </button>
    <button
      class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px flex items-center gap-1.5"
      class:border-primary={activeTab === "publish"}
      class:text-text={activeTab === "publish"}
      class:border-transparent={activeTab !== "publish"}
      class:text-text-muted={activeTab !== "publish"}
      onclick={() => activeTab = "publish"}
    >
      Review & Publish
      {#if unpublishedCliCount > 0}
        <span class="px-1.5 py-0.5 text-[10px] font-bold rounded-full bg-blue-500/30 text-blue-300 leading-none">{unpublishedCliCount}</span>
      {/if}
    </button>
  </div>

  <!-- ================================================================== -->
  <!-- Tab: Pending Renders (staging items)                                -->
  <!-- ================================================================== -->
  {#if activeTab === "staging"}
    <div class="space-y-5">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-bold">Pending Renders</h2>
        <div class="flex gap-2">
          {#if stageItems.some((q) => q.status === "error")}
            <button
              class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
              onclick={clearStageErrors}
            >Clear Errors</button>
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

      {#if stageItems.length === 0}
        <div class="text-center py-16">
          <p class="text-text-muted text-sm">No pending renders.</p>
          <p class="text-text-muted text-xs mt-2">Add clips from the clip review panel using "Add to Queue".</p>
        </div>
      {:else}
        {#each stageGrouped as group}
          <div>
            <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">
              {group.gameName}
            </h3>
            <div class="space-y-1.5">
              {#each group.items as item (item.id)}
                {@const globalIndex = stageItems.indexOf(item)}
                {@const isExpanded = expandedStageId === item.id}
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
                    onclick={() => expandedStageId = isExpanded ? null : item.id}
                  >
                    <span class="transition-transform text-[10px] text-text-muted" class:rotate-90={isExpanded}>&#9654;</span>
                    <span class="w-2 h-2 rounded-full shrink-0 {stageStatusDot(item.status)}"></span>

                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-medium truncate">{item.clipName}</span>
                        {#if isDuplicate}
                          <span class="px-1 py-0.5 bg-yellow-900/30 text-yellow-400 text-[9px] rounded" title="This clip appears multiple times in the queue">DUP</span>
                        {/if}
                      </div>
                      <div class="flex items-center gap-2 mt-0.5">
                        {#if item.pluginProfile}
                          <span class="px-1.5 py-0.5 bg-primary/30 rounded text-[10px] font-medium text-text">{item.pluginProfile}</span>
                        {/if}
                        {#each item.profiles as profile}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{profile.profile_name}</span>
                        {/each}
                        {#if item.mode}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.mode}</span>
                        {/if}
                      </div>
                    </div>

                    <span class="text-xs {stageStatusColor(item.status)} shrink-0">
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
                          onclick={() => handleEditStage(item)}
                        >Edit</button>
                        <button
                          class="px-2 py-1 text-xs text-text-muted hover:text-text bg-bg rounded transition-colors disabled:opacity-50"
                          disabled={rendering}
                          onclick={() => handleRenderSingle(item.id)}
                        >Render</button>
                      {/if}
                      {#if item.status === "error" && item.debug}
                        <button
                          class="px-2 py-1 text-xs text-yellow-400 hover:text-text bg-bg rounded transition-colors"
                          onclick={() => openDebugIndex(item.gameDir)}
                          title="Open debug index.html"
                        >Debug</button>
                      {/if}
                      <button
                        class="px-1.5 py-1 text-text-muted hover:text-accent text-xs transition-colors"
                        onclick={() => removeFromStage(item.id)}
                        title="Remove"
                      >&times;</button>
                    </div>
                  </div>

                  <!-- Expandable details -->
                  {#if isExpanded}
                    <div class="px-3 pb-3 pt-0 border-t border-border">
                      {#if item.pluginProfile}
                        <div class="pt-2.5 pb-1.5 mb-1.5 border-b border-border/50">
                          <span class="text-xs text-text-muted">Plugin Profile</span>
                          <span class="ml-2 text-sm font-medium">{item.pluginProfile}</span>
                        </div>
                      {/if}

                      <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs {item.pluginProfile ? 'pt-0' : 'pt-2.5'}">
                        <span class="text-text-muted">Mode</span>
                        <span>{item.mode === "apply" ? "Apply (full-frame)" : "Short (crop/scale)"}</span>

                        <span class="text-text-muted">Profiles</span>
                        <span>{item.profiles.map((p: { profile_name: string }) => p.profile_name).join(", ")}</span>

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

                        <span class="text-text-muted">Clip</span>
                        <span class="truncate" title={item.clipPath}>{item.clipPath}</span>

                        {#if item.status === "error" && item.error}
                          <span class="text-text-muted">Error</span>
                          <span class="text-red-400 text-[10px] break-all whitespace-pre-wrap">{item.error}</span>
                        {/if}
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

  <!-- ================================================================== -->
  <!-- Tab: Review & Publish (CLI queue items)                             -->
  <!-- ================================================================== -->
  {:else}
    <div class="space-y-5">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-bold">Review & Publish</h2>
        <div class="flex gap-2">
          <button
            class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
            onclick={() => refreshAllCliQueues()}
          >{cliLoading ? "Loading..." : "Refresh"}</button>
        </div>
      </div>

      {#if cliItems.length === 0}
        <div class="text-center py-16">
          <p class="text-text-muted text-sm">{cliLoading ? "Loading queue..." : "No rendered items to review."}</p>
          <p class="text-text-muted text-xs mt-2">Rendered clips will appear here for review before publishing.</p>
        </div>
      {:else}
        {#each cliGrouped as group}
          <div>
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted">
                {group.gameName}
              </h3>
              {#if group.items.some((i) => i.status === "rendered")}
                <button
                  class="px-3 py-1 text-xs text-secondary hover:text-text bg-bg rounded transition-colors"
                  onclick={() => publishAllCliItems(group.gameDir)}
                >Publish All</button>
              {/if}
            </div>
            <div class="space-y-1.5">
              {#each group.items as item (item.id)}
                {@const isExpanded = expandedCliId === item.id}
                <div class="bg-surface rounded-lg border border-border">
                  <!-- Summary row -->
                  <div
                    class="flex items-center gap-3 px-3 py-2.5 cursor-pointer"
                    onclick={() => expandedCliId = isExpanded ? null : item.id}
                  >
                    <span class="transition-transform text-[10px] text-text-muted" class:rotate-90={isExpanded}>&#9654;</span>
                    <span class="w-2 h-2 rounded-full shrink-0 {cliStatusDot(item.status)}"></span>

                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium truncate">{item.title || "Untitled"}</div>
                      <div class="flex items-center gap-2 mt-0.5">
                        {#if item.render_profile}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.render_profile}</span>
                        {/if}
                        {#if item.player}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.player}</span>
                        {/if}
                        {#if item.event_type}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.event_type}</span>
                        {/if}
                        {#if item.format}
                          <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{item.format}</span>
                        {/if}
                        <!-- Publish target status icons -->
                        {#each item.publish_targets as pt}
                          {@const ptClass = pt.status === "published" ? "bg-green-900/30 text-green-400"
                            : pt.status === "failed" ? "bg-red-900/30 text-red-400"
                            : pt.status === "skipped" ? "bg-yellow-900/30 text-yellow-400"
                            : "bg-bg text-text-muted"}
                          <span
                            class="px-1.5 py-0.5 rounded text-[10px] {ptClass}"
                            title="{pt.target}: {pt.status}{pt.url ? ` - ${pt.url}` : ''}{pt.error ? ` - ${pt.error}` : ''}"
                          >{pt.target}</span>
                        {/each}
                      </div>
                    </div>

                    <span class="text-xs {cliStatusColor(item.status)} shrink-0">{item.status}</span>

                    <!-- Quick actions -->
                    <div class="flex items-center gap-1 shrink-0" onclick={(e) => e.stopPropagation()}>
                      <button
                        class="px-1.5 py-1 text-text-muted hover:text-accent text-xs transition-colors"
                        onclick={() => removeCliItem(item.game_dir, item.id)}
                        title="Remove"
                      >&times;</button>
                    </div>
                  </div>

                  <!-- Expandable publish panel -->
                  {#if isExpanded}
                    <QueuePublishPanel {item} />
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>
