<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { GameSummary, GameEvent, GameInfo, RenderEntry } from "$lib/types/game";
  import type { EventTypeEntry } from "$lib/types/config";
  import { updateGameEvent, updateGameInfo, setGameLivestream, removeGameLivestream, bulkUpdateEventType, getEventTypes, processSegment, mergeHighlights, finishGame, pruneRenders, executePluginHook, discoverGameImage } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";
  import { lookupTeam } from "$lib/stores/teams.svelte";
  import { getDockSettings } from "$lib/stores/config.svelte";
  import * as uiPrefs from "$lib/stores/uiPrefs.svelte";
  import TeamLogo from "$lib/components/TeamLogo.svelte";
  import { help } from "$lib/help";
  import HelpLink from "$lib/components/HelpLink.svelte";
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
        // Extract image path from shared context and persist to game state
        const gameImage = result.shared?.game_image as { image_path?: string } | undefined;
        if (gameImage?.image_path) {
          const newState = await updateGameInfo(game.dir_path, "thumbnail", gameImage.image_path);
          onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
          log.info("GameView", `Game image regenerated: ${gameImage.image_path}`);
        } else {
          log.info("GameView", "Game image regenerated (no path returned)");
        }
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

  // ── Game info inline editing ──────────────────────────────────────

  let editingInfoField = $state<string | null>(null);
  let editInfoValue = $state("");

  const INFO_FIELDS: { key: keyof GameInfo; label: string; display?: (v: string | number) => string }[] = [
    { key: "sport", label: "Sport" },
    { key: "date", label: "Date" },
    { key: "game_time", label: "Game Time" },
    { key: "level", label: "Level" },
    { key: "tournament", label: "Tournament" },
    { key: "game_number", label: "Game Number", display: (v) => String(v) },
    { key: "period_length", label: "Period Length", display: (v) => v ? `${v} min` : "-" },
    { key: "venue", label: "Venue" },
    { key: "description", label: "Description" },
    { key: "thumbnail", label: "Thumbnail" },
    { key: "home_slug", label: "Home Slug" },
    { key: "away_slug", label: "Away Slug" },
  ];

  function startInfoEdit(field: string, currentValue: string | number) {
    editingInfoField = field;
    editInfoValue = String(currentValue);
  }

  async function commitInfoEdit() {
    if (!editingInfoField) return;
    const field = editingInfoField;
    const value = editInfoValue.trim();
    editingInfoField = null;
    try {
      const newState = await updateGameInfo(game.dir_path, field, value);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("GameView", `Updated ${field} to "${value}"`);
    } catch (err) {
      log.error("GameView", `Failed to update ${field}: ${err}`);
    }
  }

  function cancelInfoEdit() {
    editingInfoField = null;
  }

  // ── Livestream editing ────────────────────────────────────────────

  let editingLivestream = $state<string | null>(null);
  let editLivestreamValue = $state("");
  let addingLivestream = $state(false);
  let newPlatform = $state("");
  let newUrl = $state("");

  function startLivestreamEdit(platform: string, url: string) {
    editingLivestream = platform;
    editLivestreamValue = url;
  }

  async function commitLivestreamEdit() {
    if (!editingLivestream) return;
    const platform = editingLivestream;
    const url = editLivestreamValue.trim();
    editingLivestream = null;
    if (!url) return;
    try {
      const newState = await setGameLivestream(game.dir_path, platform, url);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("GameView", `Failed to update livestream: ${err}`);
    }
  }

  async function handleAddLivestream() {
    const platform = newPlatform.trim();
    const url = newUrl.trim();
    if (!platform || !url) return;
    try {
      const newState = await setGameLivestream(game.dir_path, platform, url);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      newPlatform = "";
      newUrl = "";
      addingLivestream = false;
    } catch (err) {
      log.error("GameView", `Failed to add livestream: ${err}`);
    }
  }

  async function handleRemoveLivestream(platform: string) {
    try {
      const newState = await removeGameLivestream(game.dir_path, platform);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("GameView", `Failed to remove livestream: ${err}`);
    }
  }

  // Thumbnail / discovered image
  let thumbnailError = $state(false);
  let discoveredImage = $state<string | null>(null);
  let prevGameDir = "";

  // The effective image: explicit thumbnail > discovered image
  let gameImagePath = $derived(
    (info.thumbnail && !thumbnailError) ? info.thumbnail : discoveredImage
  );
  let gameImageSrc = $derived(
    gameImagePath ? convertFileSrc(gameImagePath) : ""
  );

  // Discover image on game change
  $effect(() => {
    const dir = game.dir_path;
    if (dir !== prevGameDir) {
      prevGameDir = dir;
      thumbnailError = false;
      discoveredImage = null;
      if (!info.thumbnail) {
        discoverGameImage(dir).then((path) => {
          if (path) discoveredImage = path;
        }).catch(() => {});
      }
    }
  });

  // Right panel toggle
  let showInfoPanel = $state(true);

  let livestreamEntries = $derived(Object.entries(gs.livestreams).sort((a, b) => a[0].localeCompare(b[0])));
</script>

<div class="flex flex-col h-full">
  <!-- Hero header with background image -->
  <div class="shrink-0 relative overflow-hidden rounded-lg mb-3">
    {#if gameImageSrc}
      <img
        src={gameImageSrc}
        alt=""
        class="absolute inset-0 w-full h-full object-cover opacity-25 blur-sm"
        onerror={() => { thumbnailError = true; }}
      />
    {/if}
    <div class="relative z-10 px-4 py-4 space-y-3 {gameImageSrc ? 'bg-bg/60' : 'bg-surface/80'}">
      <!-- Back + info toggle -->
      <div class="flex items-center justify-between">
        <button
          class="text-sm text-text-muted hover:text-text transition-colors"
          onclick={() => onBack?.()}
        >&larr; All Games</button>
        <button
          class="text-xs text-text-muted hover:text-text transition-colors px-2 py-1 rounded border border-border/50"
          onclick={() => showInfoPanel = !showInfoPanel}
        >{showInfoPanel ? "Hide Details" : "Show Details"}</button>
      </div>

      <!-- Team header -->
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-3 flex-1 min-w-0">
          <TeamLogo teamName={info.home_team} size="lg" />
          <div class="min-w-0">
            <h2 class="text-xl font-bold truncate">{info.home_team} vs {info.away_team}</h2>
            <div class="flex items-center gap-2 mt-0.5 text-sm text-text-muted flex-wrap">
              <span>{info.date}</span>
              <span>{info.sport}</span>
              {#if info.venue}<span>{info.venue}</span>{/if}
              {#if info.game_time}<span>{info.game_time}</span>{/if}
              {#if info.level}<span class="px-1.5 py-0.5 rounded bg-bg/50 border border-border/50 text-xs">{info.level}</span>{/if}
              {#if info.tournament}<span class="text-secondary">{info.tournament}</span>{/if}
            </div>
          </div>
          <TeamLogo teamName={info.away_team} size="lg" />
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <button
            class="px-3 py-1.5 text-xs font-medium text-text-muted hover:text-secondary border border-border hover:border-secondary/30 rounded-lg transition-colors disabled:opacity-50"
            disabled={actionLoading === "regen-image"}
            onclick={handleRegenerateImage}
          >
            {actionLoading === "regen-image" ? "Generating..." : "Regenerate Image"}
          </button>
          {#if gs.finished}
            <span class="px-3 py-1 rounded-full bg-green-900/80 text-green-300 text-sm">Finished</span>
          {:else}
            <span class="px-3 py-1 rounded-full bg-blue-900/80 text-secondary text-sm">In Progress</span>
          {/if}
        </div>
      </div>

      {#if info.description}
        <p class="text-sm text-text-muted italic">{info.description}</p>
      {/if}
    </div>
  </div>

  <!-- Fixed controls: actions + filters -->
  <div class="shrink-0 space-y-3 pb-3">

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
          <HelpLink text={help["game.finish"].text} url={help["game.finish"].url} />
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

  <!-- Main content: split layout -->
  <div class="flex-1 flex gap-4 min-h-0">
    <!-- Left: events + renders (main workspace) -->
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
      <HelpLink text={help["game.events"].text} url={help["game.events"].url} />

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
        <HelpLink text={help["game.highlights"].text} url={help["game.highlights"].url} />
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
          <p class="text-[11px] text-text-muted mt-1">Remove generated artifacts to free disk space. <HelpLink text={help["game.prune"].text} url={help["game.prune"].url} /></p>
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

    <!-- Right: Info panel (Finder-style) -->
    {#if showInfoPanel}
      <div class="w-72 shrink-0 overflow-y-auto min-h-0 space-y-4">
        <!-- Game Image -->
        {#if gameImageSrc}
          <div class="rounded-lg overflow-hidden border border-border">
            <img
              src={gameImageSrc}
              alt="Game thumbnail"
              class="w-full object-cover"
              onerror={() => { thumbnailError = true; }}
            />
          </div>
        {/if}

        <!-- Quick Info -->
        <div class="bg-surface rounded-lg border border-border p-3 space-y-2 text-sm">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Details</h3>
          {#each INFO_FIELDS as { key, label, display }}
            {@const val = info[key]}
            {#if val || editingInfoField === key}
              <div class="flex justify-between gap-2">
                <span class="text-text-muted shrink-0">{label}</span>
                {#if editingInfoField === key}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    type="text"
                    bind:value={editInfoValue}
                    class="w-24 px-1 py-0.5 bg-bg border border-secondary rounded text-xs text-text text-right focus:outline-none"
                    onkeydown={(e) => { if (e.key === "Enter") commitInfoEdit(); if (e.key === "Escape") cancelInfoEdit(); }}
                    onblur={commitInfoEdit}
                    autofocus
                  />
                {:else}
                  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                  <span
                    class="text-text truncate text-right hover:underline hover:decoration-dotted cursor-text"
                    title={String(val)}
                    ondblclick={() => startInfoEdit(key, val)}
                  >
                    {display ? display(val) : String(val)}
                  </span>
                {/if}
              </div>
            {/if}
          {/each}
        </div>

        <!-- Livestreams -->
        {#if livestreamEntries.length > 0 || addingLivestream}
          <div class="bg-surface rounded-lg border border-border p-3 space-y-2 text-sm">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Livestreams <HelpLink text={help["game.livestream"].text} url={help["game.livestream"].url} /></h3>
            {#each livestreamEntries as [platform, url]}
              <div class="space-y-0.5">
                <div class="flex items-center justify-between">
                  <span class="text-text-muted text-xs capitalize">{platform}</span>
                  <button
                    class="text-[10px] text-text-muted hover:text-accent transition-colors"
                    onclick={() => handleRemoveLivestream(platform)}
                  >&times;</button>
                </div>
                {#if editingLivestream === platform}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    type="text"
                    bind:value={editLivestreamValue}
                    class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-xs text-text focus:outline-none"
                    onkeydown={(e) => { if (e.key === "Enter") commitLivestreamEdit(); if (e.key === "Escape") { editingLivestream = null; } }}
                    onblur={commitLivestreamEdit}
                    autofocus
                  />
                {:else}
                  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                  <a
                    href={url}
                    target="_blank"
                    rel="noopener"
                    class="text-secondary text-xs truncate block hover:underline"
                    title={url}
                    ondblclick={(e) => { e.preventDefault(); startLivestreamEdit(platform, url); }}
                  >{url}</a>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        <!-- Add Livestream -->
        {#if addingLivestream}
          <div class="bg-surface rounded-lg border border-border p-3 space-y-2 text-sm">
            <input
              type="text"
              bind:value={newPlatform}
              placeholder="Platform"
              class="w-full px-2 py-1 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
            />
            <input
              type="text"
              bind:value={newUrl}
              placeholder="URL"
              class="w-full px-2 py-1 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
              onkeydown={(e) => { if (e.key === "Enter") handleAddLivestream(); }}
            />
            <div class="flex gap-1">
              <button
                class="flex-1 px-2 py-1 bg-primary hover:bg-primary-light text-text rounded text-xs transition-colors disabled:opacity-50"
                onclick={handleAddLivestream}
                disabled={!newPlatform.trim() || !newUrl.trim()}
              >Add</button>
              <button
                class="px-2 py-1 text-xs text-text-muted hover:text-text transition-colors"
                onclick={() => { addingLivestream = false; newPlatform = ""; newUrl = ""; }}
              >Cancel</button>
            </div>
          </div>
        {:else}
          <button
            class="text-xs text-secondary hover:text-text transition-colors"
            onclick={() => addingLivestream = true}
          >+ Add Livestream</button>
        {/if}

        <!-- State -->
        <div class="bg-surface rounded-lg border border-border p-3 space-y-2 text-sm">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">State</h3>
          <div class="flex justify-between">
            <span class="text-text-muted">Created</span>
            <span class="text-text text-xs">{gs.created_at ? gs.created_at.split("T")[0] : "-"}</span>
          </div>
          {#if gs.finished}
            <div class="flex justify-between">
              <span class="text-text-muted">Finished</span>
              <span class="text-text text-xs">{gs.finished_at ? gs.finished_at.split("T")[0] : "-"}</span>
            </div>
          {/if}
          <div class="flex justify-between">
            <span class="text-text-muted">Highlighted</span>
            {#if gs.highlighted}
              <span class="text-green-400 text-xs">Yes</span>
            {:else}
              <span class="text-text-muted text-xs">No</span>
            {/if}
          </div>
          <div class="flex justify-between">
            <span class="text-text-muted">Segments</span>
            <span class="text-text text-xs">{gs.segments_processed.length}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-text-muted">Events</span>
            <span class="text-text text-xs">{gs.events.length}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-text-muted">Renders</span>
            <span class="text-text text-xs">{gs.renders.length}</span>
          </div>
          {#if gs.highlights_output}
            <div>
              <span class="text-text-muted block">Highlights</span>
              <span class="text-text text-xs truncate block" title={gs.highlights_output}>{gs.highlights_output.split("/").pop()}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}
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
