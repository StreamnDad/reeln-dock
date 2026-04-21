<script lang="ts">
  import type { GameSummary, GameEvent, RenderEntry } from "$lib/types/game";
  import type { EventTypeEntry } from "$lib/types/config";
  import { updateGameEvent, bulkUpdateEventType, getEventTypes, processSegment, mergeHighlights, finishGame, pruneRenders, executePluginHook } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";
  import { lookupTeam } from "$lib/stores/teams.svelte";
  import { getDockSettings } from "$lib/stores/config.svelte";
  import * as uiPrefs from "$lib/stores/uiPrefs.svelte";
  import RenderPlaybackModal from "./RenderPlaybackModal.svelte";
  import PrunePreviewModal from "./PrunePreviewModal.svelte";
  import DeleteGameModal from "./DeleteGameModal.svelte";

  // Action state
  let actionLoading = $state<string | null>(null);
  let actionError = $state("");
  let showPruneModal = $state(false);
  let showDeleteModal = $state(false);

  interface Props {
    game: GameSummary;
    activeEventId?: string | null;
    onBack?: () => void;
    onSelectEvent?: (id: string | null) => void;
    onUpdateGame?: (dirPath: string, updater: (g: GameSummary) => GameSummary) => void;
    onDeleteGame?: (dirPath: string) => void;
  }

  let { game, activeEventId = null, onBack, onSelectEvent, onUpdateGame, onDeleteGame }: Props = $props();
  let info = $derived(game.state.game_info);
  let gs = $derived(game.state);

  // Segment filter — persisted across navigation
  let selectedSegment = $derived(uiPrefs.getSelectedSegment());

  // Sort state
  type SortField = "event_type" | "player" | "segment_number" | "clip" | "created_at";
  let sortField = $state<SortField>("segment_number");
  let sortAsc = $state(true);

  // Inline editing
  let editingCell = $state<{ eventId: string; field: string } | null>(null);
  let editValue = $state("");
  let showSuggestions = $state(false);

  // Configured event types from config
  let configuredEventTypes = $state<EventTypeEntry[]>([]);

  $effect(() => {
    getEventTypes()
      .then((types) => { configuredEventTypes = types; })
      .catch(() => { configuredEventTypes = []; });
  });

  // Known event type names: merge configured + observed
  let knownEventTypes = $derived(
    [...new Set([...configuredEventTypes.map(e => e.name), ...gs.events.map((e) => e.event_type).filter(Boolean)])].sort(),
  );

  // Event type filter — persisted across navigation
  let selectedEventType = $derived(uiPrefs.getSelectedEventType());

  // Collapsible sections — persisted
  let eventsExpanded = $derived(uiPrefs.getEventsExpanded());
  let rendersExpanded = $derived(uiPrefs.getRendersExpanded());

  // Render playback
  let activeRender = $state<RenderEntry | null>(null);

  // Multi-select state
  let selectedEventIds = $state<Set<string>>(new Set());
  let lastClickedIndex = $state<number | null>(null);

  // Bulk tag
  let bulkTagType = $state("");
  let bulkTagLoading = $state(false);

  let filteredSuggestions = $derived(
    editingCell?.field === "event_type" && editValue
      ? knownEventTypes.filter((t) => t.toLowerCase().includes(editValue.toLowerCase()) && t !== editValue)
      : knownEventTypes,
  );

  // Filtered + sorted events
  let filteredEvents = $derived.by(() => {
    let events = gs.events;
    if (selectedSegment !== null) {
      events = events.filter((e) => e.segment_number === selectedSegment);
    }
    if (selectedEventType !== null) {
      events = events.filter((e) => e.event_type === selectedEventType);
    }
    const field = sortField;
    return [...events].sort((a, b) => {
      const aVal = String(a[field as keyof GameEvent] ?? "");
      const bVal = String(b[field as keyof GameEvent] ?? "");
      let cmp: number;
      if (field === "segment_number") {
        cmp = a.segment_number - b.segment_number;
      } else {
        cmp = aVal.localeCompare(bVal);
      }
      return sortAsc ? cmp : -cmp;
    });
  });

  // Deterministic color palette for event type badges
  const EVENT_TYPE_COLORS: [string, string][] = [
    ["#1E3A5F", "#7EB8E6"],  // deep blue / light blue
    ["#3B1F2B", "#D4849A"],  // plum / pink
    ["#1A3C34", "#6FCF97"],  // forest / mint
    ["#3D2C0A", "#E2B93B"],  // brown / gold
    ["#2E1A47", "#B48EE0"],  // indigo / lavender
    ["#3C1414", "#E07A7A"],  // maroon / salmon
    ["#0D3B3B", "#5CC9C9"],  // teal dark / teal light
    ["#3A2A0A", "#D4A05A"],  // dark amber / amber
  ];
  const CLIP_COLORS: [string, string] = ["#30363D", "#8B949E"]; // muted default for "clip"

  function eventTypeColorPair(type: string): [string, string] {
    if (!type || type === "clip") return CLIP_COLORS;
    let hash = 0;
    for (let i = 0; i < type.length; i++) {
      hash = ((hash << 5) - hash + type.charCodeAt(i)) | 0;
    }
    return EVENT_TYPE_COLORS[Math.abs(hash) % EVENT_TYPE_COLORS.length];
  }

  function eventTypeBg(type: string): string { return eventTypeColorPair(type)[0]; }
  function eventTypeFg(type: string): string { return eventTypeColorPair(type)[1]; }

  function toggleSort(field: SortField) {
    if (sortField === field) {
      sortAsc = !sortAsc;
    } else {
      sortField = field;
      sortAsc = true;
    }
  }

  function sortIndicator(field: SortField): string {
    if (sortField !== field) return "";
    return sortAsc ? " \u25B2" : " \u25BC";
  }

  function startEdit(event: GameEvent, field: string) {
    editingCell = { eventId: event.id, field };
    editValue = field === "clip" ? event.clip : field === "player" ? event.player : event.event_type;
    showSuggestions = field === "event_type";
  }

  function selectSuggestion(value: string) {
    editValue = value;
    showSuggestions = false;
    commitEdit();
  }

  async function commitEdit() {
    if (!editingCell) return;
    const { eventId, field } = editingCell;
    const value = editValue.trim();
    editingCell = null;

    try {
      log.info("GameView", `updating event ${eventId} field "${field}" to "${value}"`);
      const newState = await updateGameEvent(game.dir_path, eventId, field, value);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("GameView", `Failed to update event: ${err}`);
    }
  }

  function cancelEdit() {
    editingCell = null;
    showSuggestions = false;
  }

  function selectEvent(event: GameEvent) {
    onSelectEvent?.(activeEventId === event.id ? null : event.id);
  }

  function toggleCheckbox(event: GameEvent, index: number, shiftKey: boolean) {
    const newSet = new Set(selectedEventIds);
    if (shiftKey && lastClickedIndex !== null) {
      const start = Math.min(lastClickedIndex, index);
      const end = Math.max(lastClickedIndex, index);
      for (let i = start; i <= end; i++) {
        newSet.add(filteredEvents[i].id);
      }
    } else {
      if (newSet.has(event.id)) {
        newSet.delete(event.id);
      } else {
        newSet.add(event.id);
      }
    }
    selectedEventIds = newSet;
    lastClickedIndex = index;
  }

  function clearSelection() {
    selectedEventIds = new Set();
    lastClickedIndex = null;
  }

  async function handleBulkTag() {
    if (!bulkTagType.trim() || selectedEventIds.size === 0) return;
    bulkTagLoading = true;
    try {
      const newState = await bulkUpdateEventType(
        game.dir_path,
        [...selectedEventIds],
        bulkTagType.trim(),
      );
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("GameView", `Bulk tagged ${selectedEventIds.size} events as "${bulkTagType.trim()}"`);
      clearSelection();
      bulkTagType = "";
    } catch (err) {
      log.error("GameView", `Bulk tag failed: ${err}`);
    }
    bulkTagLoading = false;
  }

  async function handleProcessSegment(segNum: number) {
    actionLoading = `process-${segNum}`;
    actionError = "";
    try {
      const newState = await processSegment(game.dir_path, segNum);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("GameView", `Segment ${segNum} processed`);
    } catch (err) {
      actionError = String(err);
      log.error("GameView", `Failed to process segment ${segNum}: ${err}`);
    } finally {
      actionLoading = null;
    }
  }

  async function handleMergeHighlights() {
    actionLoading = "merge";
    actionError = "";
    try {
      const newState = await mergeHighlights(game.dir_path);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("GameView", "Highlights merged");
    } catch (err) {
      actionError = String(err);
      log.error("GameView", `Failed to merge highlights: ${err}`);
    } finally {
      actionLoading = null;
    }
  }

  async function handleFinishGame() {
    actionLoading = "finish";
    actionError = "";
    try {
      const newState = await finishGame(game.dir_path);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("GameView", "Game finished");
    } catch (err) {
      actionError = String(err);
      log.error("GameView", `Failed to finish game: ${err}`);
    } finally {
      actionLoading = null;
    }
  }

  async function handlePruneRenders() {
    actionLoading = "prune";
    actionError = "";
    try {
      const result = await pruneRenders(game.dir_path);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: result.state }));
      log.info("GameView", `Pruned ${result.cleared_entries} render entries, removed ${result.removed_files} files`);
    } catch (err) {
      actionError = String(err);
      log.error("GameView", `Failed to prune renders: ${err}`);
    } finally {
      actionLoading = null;
    }
  }

  async function handleRegenerateImage() {
    actionLoading = "regen-image";
    actionError = "";
    try {
      const homeProfile = lookupTeam(info.home_team);
      const awayProfile = lookupTeam(info.away_team);

      const contextData: Record<string, unknown> = {
        game_dir: game.dir_path,
        game_info: {
          sport: info.sport,
          home_team: info.home_team,
          away_team: info.away_team,
          date: info.date,
          venue: info.venue,
          game_time: info.game_time,
          level: info.level,
          tournament: info.tournament,
          description: info.description,
        },
        regenerate_image_only: true,
        ...(homeProfile ? { home_profile: homeProfile } : {}),
        ...(awayProfile ? { away_profile: awayProfile } : {}),
      };

      const configPath = getDockSettings().reeln_config_path ?? undefined;
      const result = await executePluginHook("on_game_init", contextData, {}, configPath);

      if (result.success) {
        log.info("GameView", "Game image regenerated");
      } else {
        actionError = result.errors.join("\n") || "Hook execution failed";
        log.error("GameView", `Regenerate image failed: ${actionError}`);
      }
    } catch (err) {
      actionError = String(err);
      log.error("GameView", `Failed to regenerate image: ${err}`);
    } finally {
      actionLoading = null;
    }
  }
</script>

<div class="flex flex-col h-full">
  <!-- Fixed header: back, game info, actions, filters -->
  <div class="shrink-0 space-y-4 pb-3">
    <!-- Back button -->
    <button
      class="text-sm text-text-muted hover:text-text transition-colors"
      onclick={() => onBack?.()}
    >
      &larr; All Games
    </button>

    <!-- Game Header -->
    <div class="flex items-start justify-between">
      <div>
        <h2 class="text-xl font-bold">{info.home_team} vs {info.away_team}</h2>
        <div class="flex items-center gap-3 mt-1 text-sm text-text-muted">
          <span>{info.date}</span>
          <span>{info.sport}</span>
          {#if info.venue}
            <span>{info.venue}</span>
          {/if}
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="px-3 py-1.5 text-xs font-medium text-text-muted hover:text-secondary border border-border hover:border-secondary/30 rounded-lg transition-colors disabled:opacity-50"
          disabled={actionLoading === "regen-image"}
          onclick={handleRegenerateImage}
        >
          {actionLoading === "regen-image" ? "Generating..." : "Regenerate Image"}
        </button>
        {#if gs.finished}
          <span class="px-3 py-1 rounded-full bg-green-900 text-green-300 text-sm">Finished</span>
        {:else}
          <span class="px-3 py-1 rounded-full bg-blue-900 text-secondary text-sm">In Progress</span>
        {/if}
      </div>
    </div>

    <!-- Game Actions -->
    {#if !gs.finished}
      <div>
        <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">Actions</h3>
        {#if actionError}
          <div class="mb-2 px-3 py-2 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-300">
            {actionError}
          </div>
        {/if}
        <div class="flex gap-2 flex-wrap">
          {#if !gs.finished}
            {@const maxProcessed = gs.segments_processed.length > 0 ? Math.max(...gs.segments_processed) : 0}
            {@const nextSeg = maxProcessed + 1}
            <button
              class="px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors disabled:opacity-50"
              disabled={actionLoading !== null}
              onclick={() => handleProcessSegment(nextSeg)}
            >
              {actionLoading === `process-${nextSeg}` ? "Processing..." : `Process Segment ${nextSeg}`}
            </button>
          {/if}

          {#if gs.segments_processed.length > 0 && !gs.highlighted}
            <button
              class="px-3 py-1.5 bg-secondary/20 border border-secondary rounded-lg text-sm text-secondary hover:bg-secondary/30 transition-colors disabled:opacity-50"
              disabled={actionLoading !== null}
              onclick={handleMergeHighlights}
            >
              {actionLoading === "merge" ? "Merging..." : "Merge Highlights"}
            </button>
          {/if}

          <button
            class="px-3 py-1.5 bg-green-900/30 border border-green-800 rounded-lg text-sm text-green-300 hover:bg-green-900/50 transition-colors disabled:opacity-50"
            disabled={actionLoading !== null}
            onclick={handleFinishGame}
          >
            {actionLoading === "finish" ? "Finishing..." : "Finish Game"}
          </button>
        </div>
      </div>
    {/if}

    <!-- Segment Filter Pills -->
    <div>
      <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">Segments</h3>
      {#if gs.segments_processed.length === 0}
        <p class="text-text-muted text-sm">No segments processed yet.</p>
      {:else}
        <div class="flex gap-2 flex-wrap">
          <button
            class="px-3 py-1 rounded text-sm font-medium transition-colors"
            class:bg-secondary={selectedSegment === null}
            class:text-bg={selectedSegment === null}
            class:bg-surface={selectedSegment !== null}
            class:text-text-muted={selectedSegment !== null}
            class:hover:bg-surface-hover={selectedSegment !== null}
            onclick={() => uiPrefs.setSelectedSegment(null)}
          >
            All
          </button>
          {#each gs.segments_processed as seg}
            <button
              class="px-3 py-1 rounded text-sm font-medium transition-colors"
              class:bg-secondary={selectedSegment === seg}
              class:text-bg={selectedSegment === seg}
              class:bg-surface={selectedSegment !== seg}
              class:text-text-muted={selectedSegment !== seg}
              class:hover:bg-surface-hover={selectedSegment !== seg}
              onclick={() => uiPrefs.setSelectedSegment(selectedSegment === seg ? null : seg)}
            >
              Segment {seg}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Event Type Filter Pills -->
    {#if knownEventTypes.length > 0}
      <div>
        <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">Event Types</h3>
        <div class="flex gap-2 flex-wrap">
          <button
            class="px-3 py-1 rounded text-sm font-medium transition-colors"
            class:bg-secondary={selectedEventType === null}
            class:text-bg={selectedEventType === null}
            class:bg-surface={selectedEventType !== null}
            class:text-text-muted={selectedEventType !== null}
            class:hover:bg-surface-hover={selectedEventType !== null}
            onclick={() => uiPrefs.setSelectedEventType(null)}
          >
            All
          </button>
          {#each knownEventTypes as et}
            <button
              class="px-3 py-1 rounded text-sm font-medium transition-colors"
              class:bg-secondary={selectedEventType === et}
              class:text-bg={selectedEventType === et}
              class:bg-surface={selectedEventType !== et}
              class:text-text-muted={selectedEventType !== et}
              class:hover:bg-surface-hover={selectedEventType !== et}
              onclick={() => uiPrefs.setSelectedEventType(selectedEventType === et ? null : et)}
            >
              {et}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Scrollable: events + renders as collapsible sections -->
  <div class="flex-1 overflow-y-auto min-h-0 space-y-4">
    <!-- Events (collapsible) -->
    <div>
      <button
        class="w-full text-left text-sm font-semibold uppercase tracking-wider text-text-muted hover:text-text transition-colors flex items-center gap-1.5 mb-2"
        onclick={() => uiPrefs.setEventsExpanded(!eventsExpanded)}
      >
        <span class="transition-transform text-xs" class:rotate-90={eventsExpanded}>&#9654;</span>
        Events ({filteredEvents.length}{selectedSegment !== null || selectedEventType !== null ? ` of ${gs.events.length}` : ""})
      </button>

      {#if eventsExpanded}
        <!-- Bulk tag bar -->
        {#if selectedEventIds.size > 0}
          <div class="flex items-center gap-3 mb-2 px-3 py-2 bg-primary/20 border border-primary rounded-lg">
            <span class="text-sm text-text">{selectedEventIds.size} selected</span>
            <select
              bind:value={bulkTagType}
              class="px-2 py-1 bg-bg border border-border rounded text-sm text-text"
            >
              <option value="">Select type...</option>
              {#each knownEventTypes as et}
                <option value={et}>{et}</option>
              {/each}
            </select>
            <button
              class="px-3 py-1 bg-secondary text-bg rounded text-sm font-medium transition-colors hover:bg-secondary/80 disabled:opacity-50"
              onclick={handleBulkTag}
              disabled={bulkTagLoading || !bulkTagType.trim()}
            >
              {bulkTagLoading ? "Tagging..." : "Tag Selected"}
            </button>
            <button
              class="px-3 py-1 text-text-muted hover:text-text text-sm transition-colors"
              onclick={clearSelection}
            >
              Clear
            </button>
          </div>
        {/if}

        {#if gs.events.length === 0}
          <p class="text-text-muted text-sm">No events recorded.</p>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="bg-surface rounded-lg border border-border overflow-hidden">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-border text-text-muted text-left">
                  <th class="w-8 px-2 py-2">
                    <input
                      type="checkbox"
                      checked={filteredEvents.length > 0 && filteredEvents.every((e) => selectedEventIds.has(e.id))}
                      onchange={() => {
                        if (filteredEvents.every((e) => selectedEventIds.has(e.id))) {
                          const newSet = new Set(selectedEventIds);
                          for (const e of filteredEvents) newSet.delete(e.id);
                          selectedEventIds = newSet;
                        } else {
                          const newSet = new Set(selectedEventIds);
                          for (const e of filteredEvents) newSet.add(e.id);
                          selectedEventIds = newSet;
                        }
                      }}
                      class="rounded"
                    />
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-text select-none" onclick={() => toggleSort("event_type")}>
                    Type{sortIndicator("event_type")}
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-text select-none" onclick={() => toggleSort("player")}>
                    Player{sortIndicator("player")}
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-text select-none" onclick={() => toggleSort("segment_number")}>
                    Segment{sortIndicator("segment_number")}
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-text select-none" onclick={() => toggleSort("clip")}>
                    Clip{sortIndicator("clip")}
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-text select-none" onclick={() => toggleSort("created_at")}>
                    Created{sortIndicator("created_at")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {#each filteredEvents as event, index (event.id)}
                  <tr
                    class="border-b border-border last:border-0 transition-colors cursor-pointer {selectedEventIds.has(event.id) && activeEventId !== event.id ? 'bg-secondary/10' : ''}"
                    class:bg-primary={activeEventId === event.id}
                    class:hover:bg-surface-hover={activeEventId !== event.id}
                    onclick={() => selectEvent(event)}
                  >
                    <!-- Checkbox -->
                    <td class="w-8 px-2 py-2" onclick={(e) => e.stopPropagation()}>
                      <input
                        type="checkbox"
                        checked={selectedEventIds.has(event.id)}
                        onclick={(e) => toggleCheckbox(event, index, e.shiftKey)}
                        class="rounded"
                      />
                    </td>
                    <!-- Type (combobox) -->
                    <td class="px-3 py-2 font-medium">
                      {#if editingCell?.eventId === event.id && editingCell.field === "event_type"}
                        <div class="relative" onclick={(e) => e.stopPropagation()}>
                          <!-- svelte-ignore a11y_autofocus -->
                          <input
                            type="text"
                            bind:value={editValue}
                            class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
                            onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
                            onfocus={() => (showSuggestions = true)}
                            onblur={() => { setTimeout(() => { showSuggestions = false; commitEdit(); }, 150); }}
                            autofocus
                          />
                          {#if showSuggestions && filteredSuggestions.length > 0}
                            <div class="absolute z-10 top-full left-0 right-0 mt-1 bg-surface border border-border rounded-lg shadow-lg py-1 max-h-32 overflow-y-auto">
                              {#each filteredSuggestions as suggestion}
                                <button
                                  class="w-full text-left px-2 py-1 text-sm hover:bg-surface-hover transition-colors"
                                  onmousedown={() => selectSuggestion(suggestion)}
                                >
                                  {suggestion}
                                </button>
                              {/each}
                            </div>
                          {/if}
                        </div>
                      {:else}
                        <span
                          class="inline-block px-2 py-0.5 rounded text-xs font-semibold cursor-text hover:opacity-80"
                          style="background-color: {eventTypeBg(event.event_type)}; color: {eventTypeFg(event.event_type)}"
                          ondblclick={(e) => { e.stopPropagation(); startEdit(event, "event_type"); }}
                        >
                          {event.event_type || "clip"}
                        </span>
                      {/if}
                    </td>

                    <!-- Player -->
                    <td class="px-3 py-2">
                      {#if editingCell?.eventId === event.id && editingCell.field === "player"}
                        <!-- svelte-ignore a11y_autofocus -->
                        <input
                          type="text"
                          bind:value={editValue}
                          class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
                          onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
                          onblur={commitEdit}
                          onclick={(e) => e.stopPropagation()}
                          autofocus
                        />
                      {:else}
                        <span
                          class="hover:underline hover:decoration-dotted cursor-text"
                          ondblclick={(e) => { e.stopPropagation(); startEdit(event, "player"); }}
                        >
                          {event.player || "-"}
                        </span>
                      {/if}
                    </td>

                    <!-- Segment -->
                    <td class="px-3 py-2">{event.segment_number}</td>

                    <!-- Clip (editable) -->
                    <td class="px-3 py-2 max-w-64">
                      {#if editingCell?.eventId === event.id && editingCell.field === "clip"}
                        <!-- svelte-ignore a11y_autofocus -->
                        <input
                          type="text"
                          bind:value={editValue}
                          class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
                          onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
                          onblur={commitEdit}
                          onclick={(e) => e.stopPropagation()}
                          autofocus
                        />
                      {:else}
                        <span
                          class="text-text-muted truncate block hover:underline hover:decoration-dotted cursor-text"
                          title={event.clip}
                          ondblclick={(e) => { e.stopPropagation(); startEdit(event, "clip"); }}
                        >
                          {event.clip.split("/").pop() || event.clip}
                        </span>
                      {/if}
                    </td>

                    <!-- Created -->
                    <td class="px-3 py-2 text-text-muted">{event.created_at || "-"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Renders (collapsible) -->
    <div>
      <div class="flex items-center gap-2 mb-2">
        <button
          class="text-left text-sm font-semibold uppercase tracking-wider text-text-muted hover:text-text transition-colors flex items-center gap-1.5"
          onclick={() => uiPrefs.setRendersExpanded(!rendersExpanded)}
        >
          <span class="transition-transform text-xs" class:rotate-90={rendersExpanded}>&#9654;</span>
          Renders ({gs.renders.length})
        </button>
        {#if gs.renders.length > 0}
          <button
            class="ml-auto px-2 py-0.5 text-[11px] text-text-muted hover:text-accent transition-colors disabled:opacity-50"
            disabled={actionLoading !== null}
            onclick={handlePruneRenders}
            title="Remove render files and clear history"
          >
            {actionLoading === "prune" ? "Pruning..." : "Prune"}
          </button>
        {/if}
      </div>

      {#if rendersExpanded}
        {#if gs.renders.length === 0}
          <p class="text-text-muted text-sm">No renders yet.</p>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div class="bg-surface rounded-lg border border-border overflow-hidden">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-border text-text-muted text-left">
                  <th class="px-3 py-2">Format</th>
                  <th class="px-3 py-2">Segment</th>
                  <th class="px-3 py-2">Crop</th>
                  <th class="px-3 py-2">Output</th>
                  <th class="px-3 py-2">Date</th>
                  <th class="px-3 py-2 w-16"></th>
                </tr>
              </thead>
              <tbody>
                {#each gs.renders as render}
                  <tr class="border-b border-border last:border-0 hover:bg-surface-hover cursor-pointer" onclick={() => activeRender = render}>
                    <td class="px-3 py-2 font-medium">{render.format}</td>
                    <td class="px-3 py-2">{render.segment_number}</td>
                    <td class="px-3 py-2">{render.crop_mode}</td>
                    <td class="px-3 py-2 text-text-muted truncate max-w-48">{render.output}</td>
                    <td class="px-3 py-2 text-text-muted">{render.rendered_at}</td>
                    <td class="px-3 py-2">
                      <span class="text-xs text-secondary">View</span>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Prune / Delete Game -->
    <div class="pt-2 border-t border-border flex flex-wrap items-start gap-3">
      {#if gs.finished}
        <div>
          <button
            class="px-3 py-1.5 text-xs font-medium text-text-muted hover:text-accent border border-border hover:border-accent/30 rounded-lg transition-colors"
            onclick={() => showPruneModal = true}
          >
            Prune Game Files
          </button>
          <p class="text-[11px] text-text-muted mt-1">Remove generated artifacts to free disk space.</p>
        </div>
      {/if}
      <div>
        <button
          class="px-3 py-1.5 text-xs font-medium text-text-muted hover:text-accent border border-border hover:border-accent/30 rounded-lg transition-colors"
          onclick={() => showDeleteModal = true}
        >
          Delete Game
        </button>
        <p class="text-[11px] text-text-muted mt-1">Permanently remove the entire game directory.</p>
      </div>
    </div>
  </div>
</div>

{#if activeRender}
  <RenderPlaybackModal render={activeRender} onClose={() => activeRender = null} />
{/if}

{#if showPruneModal}
  <PrunePreviewModal
    gameDir={game.dir_path}
    onClose={() => showPruneModal = false}
    onPruned={() => {
      // Reload game state after prune
      import("$lib/ipc/games").then(({ getGameState }) => {
        getGameState(game.dir_path).then((state) => {
          onUpdateGame?.(game.dir_path, () => ({ dir_path: game.dir_path, state }));
        });
      });
    }}
  />
{/if}

{#if showDeleteModal}
  <DeleteGameModal
    gameDir={game.dir_path}
    onClose={() => showDeleteModal = false}
    onDeleted={() => {
      onDeleteGame?.(game.dir_path);
    }}
  />
{/if}
