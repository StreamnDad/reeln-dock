<script lang="ts">
  import type { GameSummary, GameEvent } from "$lib/types/game";
  import { updateGameEvent, processSegment, mergeHighlights, finishGame } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";

  // Action state
  let actionLoading = $state<string | null>(null);
  let actionError = $state("");

  interface Props {
    game: GameSummary;
    activeEventId?: string | null;
    onBack?: () => void;
    onSelectEvent?: (id: string | null) => void;
    onUpdateGame?: (dirPath: string, updater: (g: GameSummary) => GameSummary) => void;
  }

  let { game, activeEventId = null, onBack, onSelectEvent, onUpdateGame }: Props = $props();
  let info = $derived(game.state.game_info);
  let gs = $derived(game.state);

  // Segment filter — null means "all"
  let selectedSegment = $state<number | null>(null);

  // Sort state
  type SortField = "event_type" | "player" | "segment_number" | "clip" | "created_at";
  let sortField = $state<SortField>("segment_number");
  let sortAsc = $state(true);

  // Inline editing
  let editingCell = $state<{ eventId: string; field: string } | null>(null);
  let editValue = $state("");
  let showSuggestions = $state(false);

  // Known event types for suggestions
  let knownEventTypes = $derived(
    [...new Set(gs.events.map((e) => e.event_type).filter(Boolean))].sort(),
  );

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
</script>

<div class="space-y-5 h-full overflow-y-auto">
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
        <!-- Process next segment button: only the next unprocessed one -->
        {@const maxProcessed = gs.segments_processed.length > 0 ? Math.max(...gs.segments_processed) : 0}
        {@const nextSeg = maxProcessed + 1}
        {#if !gs.finished}
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
          onclick={() => (selectedSegment = null)}
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
            onclick={() => (selectedSegment = selectedSegment === seg ? null : seg)}
          >
            Segment {seg}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Events Table -->
  <div>
    <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">
      Events ({filteredEvents.length}{selectedSegment !== null ? ` of ${gs.events.length}` : ""})
    </h3>
    {#if gs.events.length === 0}
      <p class="text-text-muted text-sm">No events recorded.</p>
    {:else}
      <div class="bg-surface rounded-lg border border-border overflow-hidden">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-border text-text-muted text-left">
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
            {#each filteredEvents as event (event.id)}
              <tr
                class="border-b border-border last:border-0 transition-colors cursor-pointer"
                class:bg-primary={activeEventId === event.id}
                class:hover:bg-surface-hover={activeEventId !== event.id}
                onclick={() => selectEvent(event)}
              >
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
                      class="hover:underline hover:decoration-dotted cursor-text"
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
  </div>

  <!-- Renders -->
  <div>
    <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-2">
      Renders ({gs.renders.length})
    </h3>
    {#if gs.renders.length === 0}
      <p class="text-text-muted text-sm">No renders yet.</p>
    {:else}
      <div class="bg-surface rounded-lg border border-border overflow-hidden">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-border text-text-muted text-left">
              <th class="px-3 py-2">Format</th>
              <th class="px-3 py-2">Segment</th>
              <th class="px-3 py-2">Crop</th>
              <th class="px-3 py-2">Output</th>
              <th class="px-3 py-2">Date</th>
            </tr>
          </thead>
          <tbody>
            {#each gs.renders as render}
              <tr class="border-b border-border last:border-0 hover:bg-surface-hover">
                <td class="px-3 py-2 font-medium">{render.format}</td>
                <td class="px-3 py-2">{render.segment_number}</td>
                <td class="px-3 py-2">{render.crop_mode}</td>
                <td class="px-3 py-2 text-text-muted truncate max-w-48">{render.output}</td>
                <td class="px-3 py-2 text-text-muted">{render.rendered_at}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>
