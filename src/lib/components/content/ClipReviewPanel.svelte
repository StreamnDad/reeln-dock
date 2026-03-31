<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import type { GameEvent, GameSummary, RenderEntry } from "$lib/types/game";
  import type { EventTypeEntry, RenderProfile } from "$lib/types/config";
  import type { RenderOverrides, IterationItem } from "$lib/types/render";
  import { probeClip, openInFinder } from "$lib/ipc/media";
  import { updateGameEvent, quickTagEvent } from "$lib/ipc/games";
  import { loadTeamRoster, type RosterEntry } from "$lib/ipc/teams";
  import { renderShort, renderIteration, renderPreview, deletePreview, listRenderProfiles } from "$lib/ipc/render";
  import { listConfigProfiles } from "$lib/ipc/plugins";
  import type { ConfigProfile } from "$lib/types/plugin";
  import { addToQueue as addToRenderQueue } from "$lib/stores/renderQueue.svelte";
  import { getDockSettings } from "$lib/stores/config.svelte";
  import * as uiPrefs from "$lib/stores/uiPrefs.svelte";
  import { log } from "$lib/stores/log.svelte";
  import type { MediaInfoResponse } from "$lib/types/media";
  import VideoPlayer from "./VideoPlayer.svelte";
  import RenderPlaybackModal from "./RenderPlaybackModal.svelte";
  import DynamicPluginFields from "./DynamicPluginFields.svelte";
  import { getActiveFieldsForScreen } from "$lib/stores/pluginUI.svelte";

  interface Props {
    event: GameEvent;
    game: GameSummary;
    eventTypes?: EventTypeEntry[];
    iterationMappings?: Record<string, string[]>;
    expanded?: boolean;
    eventIndex?: number;
    eventCount?: number;
    onClose?: () => void;
    onUpdateGame?: (dirPath: string, updater: (g: GameSummary) => GameSummary) => void;
    onNext?: () => void;
    onPrev?: () => void;
    onToggleExpand?: () => void;
  }

  let {
    event, game, eventTypes = [], iterationMappings = {},
    expanded = false, eventIndex = 0, eventCount = 0,
    onClose, onUpdateGame, onNext, onPrev, onToggleExpand,
  }: Props = $props();

  let probeInfo = $state<MediaInfoResponse | null>(null);

  // Editable fields
  let editingField = $state<string | null>(null);
  let editValue = $state("");
  let showSuggestions = $state(false);
  // Persisted across navigation via uiPrefs store (read via getter, write via setter)
  let autoPlay = $derived(uiPrefs.getAutoPlay());
  let autoAdvance = $derived(uiPrefs.getAutoAdvance());
  let showRender = $derived(uiPrefs.getShowRender());
  let showDetails = $derived(uiPrefs.getShowDetails());
  let showMediaInfo = $derived(uiPrefs.getShowMediaInfo());

  // Render profiles
  let renderProfiles = $state<RenderProfile[]>([]);
  let pluginProfiles = $state<ConfigProfile[]>([]);
  let selectedPluginProfile = $state("");
  let renderMode = $state<"short" | "apply">("short");
  let renderResult = $state<RenderEntry | null>(null);

  // Plugin-contributed render option fields
  let pluginFieldGroups = $derived(getActiveFieldsForScreen("render_options"));
  let renderQueue = $state<IterationItem[]>([]);
  let concatOutput = $state(true);
  let addProfileName = $state("");
  let showOverrides = $state(false);
  let overrides = $state<RenderOverrides>({});
  let renderLoading = $state(false);
  let renderError = $state("");
  let renderSuccess = $state("");
  let dragIndex = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  // Preview state
  let previewPath = $state<string | null>(null);
  let previewProfileName = $state("");

  // Queue state
  let queueAdded = $state(false);

  // Roster data for player assignment
  let homeRoster = $state<RosterEntry[]>([]);
  let awayRoster = $state<RosterEntry[]>([]);


  // Load profiles
  $effect(() => {
    listRenderProfiles()
      .then((profiles) => {
        renderProfiles = profiles;
        if (profiles.length > 0 && !addProfileName) {
          addProfileName = profiles[0].name;
        }
      })
      .catch(() => {});
    listConfigProfiles()
      .then((p) => { pluginProfiles = p; })
      .catch(() => {});
    // Set default plugin profile from dock settings
    const dockDefault = getDockSettings().rendering?.default_plugin_profile;
    if (dockDefault) {
      selectedPluginProfile = dockDefault;
    }
  });

  // Load rosters for home and away teams
  $effect(() => {
    const info = game.state.game_info;
    if (info.home_team && info.level) {
      loadTeamRoster(info.home_team, info.level)
        .then((r) => { homeRoster = r; })
        .catch(() => { homeRoster = []; });
    }
    if (info.away_team && info.level) {
      loadTeamRoster(info.away_team, info.level)
        .then((r) => { awayRoster = r; })
        .catch(() => { awayRoster = []; });
    }
  });

  // Auto-populate queue from dock rendering defaults, then config iteration mappings
  $effect(() => {
    const eventType = event.event_type || "default";
    const dockRendering = getDockSettings().rendering;
    const dockMappings = dockRendering?.iteration_mappings ?? {};
    // Dock overrides take priority over config iteration mappings
    const effectiveMappings = Object.keys(dockMappings).length > 0
      ? { ...iterationMappings, ...dockMappings }
      : iterationMappings;
    const profileNames = effectiveMappings[eventType] ?? effectiveMappings["default"] ?? [];
    if (profileNames.length > 0) {
      renderQueue = profileNames.map((name) => ({ profile_name: name }));
      previewProfileName = profileNames[0] ?? "";
    } else if (renderProfiles.length > 0) {
      renderQueue = [{ profile_name: renderProfiles[0].name }];
      previewProfileName = renderProfiles[0].name;
    }
    // Apply default concat setting from dock
    if (dockRendering?.concat_by_default != null) {
      concatOutput = dockRendering.concat_by_default;
    }
    // Apply default render mode from dock
    if (dockRendering?.default_render_mode) {
      renderMode = dockRendering.default_render_mode as "short" | "apply";
    }
    // Apply default render overrides from dock
    const dockOverrides = dockRendering?.overrides;
    const pluginDefaults = (dockRendering?.plugin_field_defaults as Record<string, unknown>) ?? {};
    if (dockOverrides || Object.keys(pluginDefaults).length > 0) {
      overrides = {
        ...pluginDefaults,
        crop_mode: dockOverrides?.crop_mode,
        scale: dockOverrides?.scale,
        speed: dockOverrides?.speed,
        smart: dockOverrides?.smart,
      };
    }
  });

  function resolveClipPath(clip: string): string {
    if (clip.startsWith("/")) return clip;
    return `${game.dir_path}/${clip}`;
  }

  let fullClipPath = $derived(resolveClipPath(event.clip));
  let videoSrc = $derived(convertFileSrc(fullClipPath));
  let videoError = $state(false);

  // Current tag state
  let currentTeam = $derived(
    typeof event.metadata?.team === "string" ? event.metadata.team : null,
  );

  // Player assignment from metadata
  let currentScorer = $derived(
    typeof event.metadata?.scorer === "string" ? event.metadata.scorer : "",
  );
  let currentAssist1 = $derived(
    typeof event.metadata?.assist1 === "string" ? event.metadata.assist1 : "",
  );
  let currentAssist2 = $derived(
    typeof event.metadata?.assist2 === "string" ? event.metadata.assist2 : "",
  );

  // Combined roster: team-specific if tagged, otherwise both teams
  let activeRoster = $derived.by(() => {
    if (currentTeam === "home") return homeRoster;
    if (currentTeam === "away") return awayRoster;
    // Merge both rosters, sorted by name
    return [...homeRoster, ...awayRoster].sort((a, b) => a.name.localeCompare(b.name));
  });

  function playerLabel(entry: RosterEntry): string {
    return entry.number ? `#${entry.number} ${entry.name}` : entry.name;
  }

  async function updatePlayerField(field: string, value: string) {
    try {
      const newState = await updateGameEvent(game.dir_path, event.id, field, value);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("ClipReview", `Failed to update ${field}: ${err}`);
    }
  }

  // Suggestions for inline editing
  let knownEventTypes = $derived(
    [...new Set([...eventTypes.map((e) => e.name), ...game.state.events.map((e) => e.event_type).filter(Boolean)])].sort(),
  );
  let filteredSuggestions = $derived(
    editingField === "event_type" && editValue
      ? knownEventTypes.filter((t) => t.toLowerCase().includes(editValue.toLowerCase()) && t !== editValue)
      : knownEventTypes,
  );

  $effect(() => {
    const clipPath = fullClipPath;
    probeInfo = null;
    videoError = false;
    // Clean up any active preview when navigating to a different clip
    if (previewPath) {
      deletePreview(previewPath).catch(() => {});
      previewPath = null;
    }
    probeClip(clipPath)
      .then((result) => { probeInfo = result; })
      .catch(() => {});
  });

  function formatDuration(secs: number | null): string {
    if (secs == null) return "-";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  // Quick-tag: 1-click tagging
  async function handleQuickTag(typeName: string, team?: string) {
    try {
      const newState = await quickTagEvent(game.dir_path, event.id, typeName, team);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("ClipReview", `Tagged ${event.id} as ${team ? team + " " : ""}${typeName}`);
      if (autoAdvance) {
        await tick();
        onNext?.();
      }
    } catch (err) {
      log.error("ClipReview", `Quick tag failed: ${err}`);
    }
  }

  function handleVideoEnded() {
    if (autoPlay && autoAdvance) {
      onNext?.();
    }
  }

  // Keyboard: media keys + escape
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "MediaTrackNext") {
      e.preventDefault();
      onNext?.();
    } else if (e.key === "MediaTrackPrevious") {
      e.preventDefault();
      onPrev?.();
    } else if (e.key === "Escape" && expanded) {
      e.preventDefault();
      onToggleExpand?.();
    }
  }

  // Inline editing
  function startEdit(field: string, currentValue: string) {
    editingField = field;
    editValue = currentValue;
    showSuggestions = field === "event_type";
  }

  function selectSuggestion(value: string) {
    editValue = value;
    showSuggestions = false;
    commitEdit();
  }

  async function commitEdit() {
    if (!editingField) return;
    const field = editingField;
    const value = editValue.trim();
    editingField = null;
    showSuggestions = false;
    try {
      const newState = await updateGameEvent(game.dir_path, event.id, field, value);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("ClipReview", `Failed to update: ${err}`);
    }
  }

  function cancelEdit() {
    editingField = null;
    showSuggestions = false;
  }

  function fileName(path: string): string {
    return path.split("/").pop() || path;
  }

  // Queue management
  function addToQueue() {
    if (!addProfileName) return;
    renderQueue = [...renderQueue, { profile_name: addProfileName }];
  }

  function removeFromQueue(index: number) {
    renderQueue = renderQueue.filter((_, i) => i !== index);
  }

  function handleDragStart(index: number) { dragIndex = index; }
  function handleDragOver(index: number) {
    if (dragIndex === null || dragIndex === index) return;
    dragOverIndex = index;
  }
  function handleDragEnd() {
    if (dragIndex !== null && dragOverIndex !== null && dragIndex !== dragOverIndex) {
      const items = [...renderQueue];
      const [moved] = items.splice(dragIndex, 1);
      items.splice(dragOverIndex, 0, moved);
      renderQueue = items;
    }
    dragIndex = null;
    dragOverIndex = null;
  }

  function cleanOverrides(ovr: RenderOverrides): RenderOverrides | undefined {
    const clean: RenderOverrides = {};
    if (ovr.crop_mode) clean.crop_mode = ovr.crop_mode;
    if (ovr.scale != null && ovr.scale !== 1.0) clean.scale = ovr.scale;
    if (ovr.speed != null && ovr.speed !== 1.0) clean.speed = ovr.speed;
    if (ovr.smart != null) clean.smart = ovr.smart;
    if (ovr.anchor_x != null) clean.anchor_x = ovr.anchor_x;
    if (ovr.anchor_y != null) clean.anchor_y = ovr.anchor_y;
    if (ovr.pad_color) clean.pad_color = ovr.pad_color;
    return Object.keys(clean).length > 0 ? clean : undefined;
  }

  async function handleRender() {
    if (renderQueue.length === 0) return;
    renderLoading = true;
    renderError = "";
    renderSuccess = "";
    renderResult = null;
    try {
      const outputDir = game.dir_path + "/renders";
      const effectiveOverrides = cleanOverrides(overrides);
      let resultEntry: RenderEntry | null = null;
      if (renderQueue.length === 1) {
        const item = renderQueue[0];
        const mergedOverrides = effectiveOverrides ? { ...effectiveOverrides, ...item.overrides } : item.overrides;
        const entry = await renderShort(fullClipPath, outputDir, item.profile_name, event.id, game.dir_path, mergedOverrides, renderMode);
        resultEntry = entry;
      } else {
        const items: IterationItem[] = renderQueue.map((item) => ({
          profile_name: item.profile_name,
          overrides: effectiveOverrides ? { ...effectiveOverrides, ...item.overrides } : item.overrides,
        }));
        const entries = await renderIteration(fullClipPath, outputDir, items, event.id, game.dir_path, concatOutput, renderMode);
        resultEntry = entries[0] ?? null;
      }
      const { getGameState } = await import("$lib/ipc/games");
      const newState = await getGameState(game.dir_path);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      // Auto-show the result
      if (resultEntry) {
        renderResult = resultEntry;
      }
    } catch (err) {
      renderError = String(err);
      log.error("ClipReview", `Render failed: ${err}`);
    } finally {
      renderLoading = false;
    }
  }

  async function handleRenderPreview() {
    renderLoading = true;
    renderError = "";
    renderSuccess = "";
    // Clean up any existing preview first
    if (previewPath) {
      await deletePreview(previewPath).catch(() => {});
      previewPath = null;
    }
    try {
      const outputDir = game.dir_path + "/previews";
      const profileForPreview = previewProfileName || renderQueue[0]?.profile_name || undefined;
      const output = await renderPreview(fullClipPath, outputDir, profileForPreview);
      previewPath = output;
      renderSuccess = "";
    } catch (err) {
      renderError = String(err);
    } finally {
      renderLoading = false;
    }
  }

  async function closePreview() {
    if (previewPath) {
      await deletePreview(previewPath).catch(() => {});
      previewPath = null;
    }
  }

  function handleAddToQueue() {
    if (renderQueue.length === 0) return;
    const info = game.state.game_info;
    addToRenderQueue({
      gameDir: game.dir_path,
      gameName: `${info.home_team} vs ${info.away_team}`,
      eventId: event.id,
      clipPath: fullClipPath,
      clipName: fileName(event.clip),
      profiles: [...renderQueue],
      concatOutput: concatOutput,
      overrides: cleanOverrides(overrides),
      pluginProfile: selectedPluginProfile || undefined,
    });
    queueAdded = true;
    setTimeout(() => { queueAdded = false; }, 1500);
  }

  function profileLabel(name: string): string {
    const p = renderProfiles.find((rp) => rp.name === name);
    if (p?.width && p?.height) return `${name} (${p.width}x${p.height})`;
    return name;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-full overflow-y-auto space-y-3 p-4">
  <!-- Header -->
  <div class="flex items-center justify-between">
    {#if expanded}
      <div class="flex items-center gap-3">
        <button
          class="px-2 py-1 text-sm text-text-muted hover:text-text transition-colors disabled:opacity-30"
          onclick={() => onPrev?.()}
          disabled={eventIndex <= 0}
        >&#9664; Prev</button>
        <span class="text-sm text-text-muted">{eventIndex + 1} / {eventCount}</span>
        <button
          class="px-2 py-1 text-sm text-text-muted hover:text-text transition-colors disabled:opacity-30"
          onclick={() => onNext?.()}
          disabled={eventIndex >= eventCount - 1}
        >Next &#9654;</button>
      </div>
    {:else}
      <h3 class="text-lg font-bold">Clip Review</h3>
    {/if}
    <div class="flex items-center gap-2">
      {#if onToggleExpand}
        <button
          class="text-sm text-text-muted hover:text-text transition-colors"
          onclick={() => onToggleExpand?.()}
          title={expanded ? "Collapse" : "Expand"}
        >{expanded ? "Collapse" : "Expand"}</button>
      {/if}
      <button class="text-sm text-text-muted hover:text-text transition-colors" onclick={() => onClose?.()}>&times;</button>
    </div>
  </div>

  <!-- Video Player / Preview -->
  {#if previewPath}
    <div class="rounded-lg border-2 border-secondary bg-black overflow-hidden relative">
      <div class="absolute top-2 left-2 z-10 px-2 py-0.5 bg-secondary text-bg text-xs font-medium rounded">
        Preview
      </div>
      <VideoPlayer
        src={convertFileSrc(previewPath)}
        autoplay={true}
      />
      <button
        class="absolute top-2 right-2 z-10 px-2 py-0.5 bg-bg/80 text-text-muted hover:text-text text-xs rounded transition-colors"
        onclick={closePreview}
      >Close Preview</button>
    </div>
  {:else}
    <div class="rounded-lg border border-border bg-black overflow-hidden">
      {#if videoError}
        <div class="aspect-video flex items-center justify-center bg-bg">
          <div class="text-center p-4 max-w-xs">
            <span class="text-4xl text-text-muted">&#9888;</span>
            <p class="text-accent text-sm mt-2">Could not load video</p>
            <p class="text-text-muted text-xs mt-1 truncate" title={fullClipPath}>{fileName(event.clip)}</p>
            <button
              class="mt-2 px-2 py-1 text-xs text-text-muted hover:text-text bg-surface border border-border rounded transition-colors"
              onclick={() => { if (autoAdvance) onNext?.(); }}
            >Skip</button>
          </div>
        </div>
      {:else}
        {#key event.id}
          <VideoPlayer
            src={videoSrc}
            autoplay={autoPlay}
            onended={handleVideoEnded}
            onerror={() => { videoError = true; }}
            onloadeddata={() => { videoError = false; }}
          />
        {/key}
      {/if}
    </div>
  {/if}

  <!-- Player controls -->
  <div class="flex items-center gap-3 text-sm">
    <label class="flex items-center gap-1.5 text-text-muted cursor-pointer">
      <input type="checkbox" checked={autoPlay} onchange={() => uiPrefs.setAutoPlay(!autoPlay)} class="accent-secondary" />
      Auto-play
    </label>
    <label class="flex items-center gap-1.5 text-text-muted cursor-pointer">
      <input type="checkbox" checked={autoAdvance} onchange={() => uiPrefs.setAutoAdvance(!autoAdvance)} class="accent-secondary" />
      Auto-advance
    </label>
    {#if probeInfo}
      <span class="text-text-muted ml-auto">{formatDuration(probeInfo.duration_secs)}</span>
      <span class="text-text-muted">{probeInfo.width && probeInfo.height ? `${probeInfo.width}x${probeInfo.height}` : ""}</span>
    {/if}
  </div>

  <!-- Quick-Tag Pills -->
  {#if eventTypes.length > 0}
    <div class="flex flex-wrap gap-1.5">
      {#each eventTypes as et}
        {#if et.team_specific}
          <button
            class="px-2.5 py-1 rounded text-xs font-medium transition-colors"
            class:bg-blue-700={event.event_type === et.name && currentTeam === "home"}
            class:text-white={event.event_type === et.name && currentTeam === "home"}
            class:bg-surface={event.event_type !== et.name || currentTeam !== "home"}
            class:text-text-muted={event.event_type !== et.name || currentTeam !== "home"}
            class:border={true}
            class:border-blue-600={event.event_type === et.name && currentTeam === "home"}
            class:border-border={event.event_type !== et.name || currentTeam !== "home"}
            class:hover:border-blue-500={true}
            onclick={() => handleQuickTag(et.name, "home")}
          >H {et.name}</button>
          <button
            class="px-2.5 py-1 rounded text-xs font-medium transition-colors"
            class:bg-red-700={event.event_type === et.name && currentTeam === "away"}
            class:text-white={event.event_type === et.name && currentTeam === "away"}
            class:bg-surface={event.event_type !== et.name || currentTeam !== "away"}
            class:text-text-muted={event.event_type !== et.name || currentTeam !== "away"}
            class:border={true}
            class:border-red-600={event.event_type === et.name && currentTeam === "away"}
            class:border-border={event.event_type !== et.name || currentTeam !== "away"}
            class:hover:border-red-500={true}
            onclick={() => handleQuickTag(et.name, "away")}
          >A {et.name}</button>
        {:else}
          <button
            class="px-2.5 py-1 rounded text-xs font-medium transition-colors border"
            class:bg-secondary={event.event_type === et.name}
            class:text-bg={event.event_type === et.name}
            class:border-secondary={event.event_type === et.name}
            class:bg-surface={event.event_type !== et.name}
            class:text-text-muted={event.event_type !== et.name}
            class:border-border={event.event_type !== et.name}
            class:hover:border-secondary={event.event_type !== et.name}
            onclick={() => handleQuickTag(et.name)}
          >{et.name}</button>
        {/if}
      {/each}
    </div>
  {/if}

  <!-- Player Assignment (scorer + assists) -->
  {#if event.event_type}
    <div class="space-y-1.5">
      {#each [{ id: "scorer", label: "Scorer", value: currentScorer }, { id: "assist1", label: "Assist 1", value: currentAssist1 }, { id: "assist2", label: "Assist 2", value: currentAssist2 }] as field}
        <div class="flex items-center gap-2">
          <label class="text-xs text-text-muted w-14 shrink-0" for={field.id}>{field.label}</label>
          {#if activeRoster.length > 0}
            <select
              id={field.id}
              value={field.value}
              onchange={(e) => updatePlayerField(field.id, (e.target as HTMLSelectElement).value)}
              class="flex-1 px-2 py-1 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
            >
              <option value="">-</option>
              {#each activeRoster as entry}
                <option value={entry.name}>{playerLabel(entry)}</option>
              {/each}
            </select>
          {:else}
            <input
              type="text"
              id={field.id}
              value={field.value}
              placeholder="-"
              onblur={(e) => updatePlayerField(field.id, (e.target as HTMLInputElement).value)}
              onkeydown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
              class="flex-1 px-2 py-1 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
            />
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <!-- Collapsible: Render Options -->
  <button
    class="w-full text-left text-xs text-text-muted hover:text-text transition-colors flex items-center gap-1"
    onclick={() => uiPrefs.setShowRender(!showRender)}
  >
    <span class="transition-transform" class:rotate-90={showRender}>&#9654;</span>
    Render Options
  </button>
  {#if showRender}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
      {#if renderError}
        <div class="px-3 py-2 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-300">{renderError}</div>
      {/if}
      {#if renderSuccess}
        <div class="px-3 py-2 bg-green-900/30 border border-green-800 rounded-lg text-sm text-green-300">{renderSuccess}</div>
      {/if}

      {#if renderQueue.length > 0}
        <div class="space-y-1">
          {#each renderQueue as item, i (i)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="flex items-center gap-2 px-2 py-1.5 rounded-lg border transition-colors text-sm"
              class:border-secondary={dragOverIndex === i}
              class:border-border={dragOverIndex !== i}
              class:bg-bg={dragOverIndex !== i}
              class:bg-surface-hover={dragOverIndex === i}
              draggable="true"
              ondragstart={() => handleDragStart(i)}
              ondragover={(e) => { e.preventDefault(); handleDragOver(i); }}
              ondragend={handleDragEnd}
            >
              <span class="text-text-muted cursor-grab text-xs">&#9776;</span>
              <span class="flex-1 font-medium truncate">{profileLabel(item.profile_name)}</span>
              <span class="text-xs text-text-muted">{i + 1}/{renderQueue.length}</span>
              <button class="text-text-muted hover:text-accent text-xs transition-colors" onclick={() => removeFromQueue(i)}>&times;</button>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-text-muted text-xs">No profiles in queue.</p>
      {/if}

      {#if renderProfiles.length > 0}
        <div class="flex gap-2">
          <select bind:value={addProfileName} class="flex-1 px-2 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary">
            {#each renderProfiles as profile}
              <option value={profile.name}>{profileLabel(profile.name)}</option>
            {/each}
          </select>
          <button class="px-3 py-1.5 text-xs bg-bg border border-border rounded-lg text-text-muted hover:text-text hover:border-secondary transition-colors" onclick={addToQueue}>+ Add</button>
        </div>
      {/if}

      {#if renderQueue.length > 1}
        <label class="flex items-center gap-2 text-xs text-text-muted cursor-pointer">
          <input type="checkbox" bind:checked={concatOutput} class="accent-secondary" />
          Concatenate into single output
        </label>
      {/if}

      <!-- Render mode: Short vs Apply -->
      <div>
        <label class="block text-xs text-text-muted mb-1">Render Mode</label>
        <div class="flex gap-1">
          <button
            class="flex-1 px-2 py-1 rounded text-xs font-medium transition-colors text-center"
            class:bg-secondary={renderMode === "short"}
            class:text-bg={renderMode === "short"}
            class:bg-bg={renderMode !== "short"}
            class:text-text-muted={renderMode !== "short"}
            class:border={renderMode !== "short"}
            class:border-border={renderMode !== "short"}
            onclick={() => renderMode = "short"}
          >Short (crop/scale)</button>
          <button
            class="flex-1 px-2 py-1 rounded text-xs font-medium transition-colors text-center"
            class:bg-secondary={renderMode === "apply"}
            class:text-bg={renderMode === "apply"}
            class:bg-bg={renderMode !== "apply"}
            class:text-text-muted={renderMode !== "apply"}
            class:border={renderMode !== "apply"}
            class:border-border={renderMode !== "apply"}
            onclick={() => renderMode = "apply"}
          >Apply (full-frame)</button>
        </div>
      </div>

      <!-- Plugin profile -->
      {#if pluginProfiles.length > 0}
        <div>
          <label class="block text-xs text-text-muted mb-1" for="clip-plugin-profile">Plugin Profile</label>
          <select
            id="clip-plugin-profile"
            bind:value={selectedPluginProfile}
            class="w-full px-2 py-1.5 bg-bg border border-border rounded-lg text-xs text-text focus:outline-none focus:border-secondary"
          >
            <option value="">None (no plugins)</option>
            {#each pluginProfiles as pp}
              <option value={pp.name}>{pp.name}</option>
            {/each}
          </select>
        </div>
      {/if}

      <button
        class="w-full text-left text-xs text-text-muted hover:text-text transition-colors flex items-center gap-1"
        onclick={() => showOverrides = !showOverrides}
      >
        <span class="transition-transform" class:rotate-90={showOverrides}>&#9654;</span>
        Overrides {#if cleanOverrides(overrides)}<span class="text-secondary">(active)</span>{/if}
      </button>

      {#if showOverrides}
        <div class="space-y-3 pl-2 border-l-2 border-border">
          <div>
            <label class="block text-xs text-text-muted mb-1" for="crop-mode">Crop Mode</label>
            <select id="crop-mode" bind:value={overrides.crop_mode} class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary">
              <option value="">Profile Default</option>
              <option value="pad">Pad (letterbox)</option>
              <option value="crop">Crop (fill)</option>
            </select>
          </div>
          {#if overrides.crop_mode === "pad"}
            <div>
              <label class="block text-xs text-text-muted mb-1" for="pad-color">Pad Color</label>
              <input id="pad-color" type="text" bind:value={overrides.pad_color} placeholder="black" class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary" />
            </div>
          {/if}
          <div>
            <label class="block text-xs text-text-muted mb-1" for="scale">Scale: {overrides.scale?.toFixed(1) ?? "default"}</label>
            <div class="flex items-center gap-2">
              <input id="scale" type="range" min="0.5" max="3.0" step="0.1" bind:value={overrides.scale} class="flex-1 accent-secondary" />
              <button class="text-[10px] text-text-muted hover:text-text" onclick={() => overrides.scale = undefined}>reset</button>
            </div>
          </div>
          <div>
            <label class="block text-xs text-text-muted mb-1" for="speed">Speed: {overrides.speed?.toFixed(1) ?? "default"}x</label>
            <div class="flex items-center gap-2">
              <input id="speed" type="range" min="0.5" max="2.0" step="0.1" bind:value={overrides.speed} class="flex-1 accent-secondary" />
              <button class="text-[10px] text-text-muted hover:text-text" onclick={() => overrides.speed = undefined}>reset</button>
            </div>
          </div>
          <div>
            <label class="block text-xs text-text-muted mb-1" for="anchor">Anchor</label>
            <select id="anchor" class="w-full px-2 py-1 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary" onchange={(e) => {
              const v = (e.target as HTMLSelectElement).value;
              const positions: Record<string, [number, number]> = { center: [0.5, 0.5], top: [0.5, 0], bottom: [0.5, 1], left: [0, 0.5], right: [1, 0.5] };
              if (v === "") { overrides.anchor_x = undefined; overrides.anchor_y = undefined; }
              else if (v in positions) { [overrides.anchor_x, overrides.anchor_y] = positions[v]; }
            }}>
              <option value="">Profile Default</option>
              <option value="center">Center</option>
              <option value="top">Top</option>
              <option value="bottom">Bottom</option>
              <option value="left">Left</option>
              <option value="right">Right</option>
            </select>
          </div>
          <!-- Plugin-contributed fields -->
          {#each pluginFieldGroups as group}
            <DynamicPluginFields
              fields={group.fields}
              values={overrides}
              pluginName={group.pluginName}
              onchange={(key, value) => { overrides = { ...overrides, [key]: value }; }}
            />
          {/each}
        </div>
      {/if}

      <div class="flex gap-2">
        <button
          class="flex-1 px-3 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-center"
          disabled={renderLoading || renderQueue.length === 0}
          onclick={handleRender}
        >
          {renderLoading ? "Rendering..." : renderQueue.length > 1 ? `Render ${renderQueue.length} Formats` : "Render Short"}
        </button>
        <button
          class="px-3 py-2 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors disabled:opacity-50 text-center whitespace-nowrap"
          disabled={renderQueue.length === 0}
          onclick={handleAddToQueue}
        >
          {queueAdded ? "Added" : "+ Queue"}
        </button>
      </div>
      <!-- Preview with profile selector -->
      <div class="flex gap-2 items-center">
        {#if renderProfiles.length > 0}
          <select
            bind:value={previewProfileName}
            class="flex-1 px-2 py-1.5 bg-bg border border-border rounded-lg text-xs text-text focus:outline-none focus:border-secondary"
          >
            <option value="">No profile (640p)</option>
            {#each renderProfiles as profile}
              <option value={profile.name}>{profileLabel(profile.name)}</option>
            {/each}
          </select>
        {/if}
        <button
          class="px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors disabled:opacity-50 text-center whitespace-nowrap"
          disabled={renderLoading}
          onclick={handleRenderPreview}
        >
          {renderLoading ? "..." : "Preview"}
        </button>
      </div>
    </div>
  {/if}

  <!-- Collapsible: Event Details -->
  <button
    class="w-full text-left text-xs text-text-muted hover:text-text transition-colors flex items-center gap-1"
    onclick={() => uiPrefs.setShowDetails(!showDetails)}
  >
    <span class="transition-transform" class:rotate-90={showDetails}>&#9654;</span>
    Event Details
    <span class="ml-auto text-text-muted">{event.event_type || "untagged"}{currentTeam ? ` (${currentTeam})` : ""} &middot; Seg {event.segment_number}</span>
  </button>
  {#if showDetails}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
      <div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-sm">
        <span class="text-text-muted">Type</span>
        {#if editingField === "event_type"}
          <div class="relative">
            <!-- svelte-ignore a11y_autofocus -->
            <input type="text" bind:value={editValue} class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none" onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }} onfocus={() => (showSuggestions = true)} onblur={() => { setTimeout(() => { showSuggestions = false; commitEdit(); }, 150); }} autofocus />
            {#if showSuggestions && filteredSuggestions.length > 0}
              <div class="absolute z-10 top-full left-0 right-0 mt-1 bg-surface border border-border rounded-lg shadow-lg py-1 max-h-32 overflow-y-auto">
                {#each filteredSuggestions as suggestion}
                  <button class="w-full text-left px-2 py-1 text-sm hover:bg-surface-hover transition-colors" onmousedown={() => selectSuggestion(suggestion)}>{suggestion}</button>
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="font-medium cursor-text hover:underline hover:decoration-dotted" ondblclick={() => startEdit("event_type", event.event_type)}>{event.event_type || "clip"}</span>
        {/if}

        <span class="text-text-muted">Team</span>
        <span>{currentTeam || "-"}</span>

        <span class="text-text-muted">Player</span>
        {#if editingField === "player"}
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={editValue} class="px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none" onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }} onblur={commitEdit} autofocus />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="cursor-text hover:underline hover:decoration-dotted" ondblclick={() => startEdit("player", event.player)}>{event.player || "-"}</span>
        {/if}

        <span class="text-text-muted">Segment</span>
        <span>{event.segment_number}</span>

        <span class="text-text-muted">File</span>
        {#if editingField === "clip"}
          <!-- svelte-ignore a11y_autofocus -->
          <input type="text" bind:value={editValue} class="px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none" onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }} onblur={commitEdit} autofocus />
        {:else}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="text-text-muted truncate cursor-text hover:underline hover:decoration-dotted" title={event.clip} ondblclick={() => startEdit("clip", event.clip)}>{fileName(event.clip)}</span>
        {/if}

        <span class="text-text-muted">ID</span>
        <span class="text-text-muted font-mono text-xs">{event.id}</span>
      </div>

      <div class="flex gap-2 pt-2">
        <button class="flex-1 px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center" onclick={() => openInFinder(fullClipPath)}>Open in Finder</button>
        <button class="flex-1 px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center" onclick={() => navigator.clipboard.writeText(fullClipPath)}>Copy Path</button>
      </div>
    </div>
  {/if}

  <!-- Collapsible: Media Info -->
  {#if probeInfo}
    <button
      class="w-full text-left text-xs text-text-muted hover:text-text transition-colors flex items-center gap-1"
      onclick={() => uiPrefs.setShowMediaInfo(!showMediaInfo)}
    >
      <span class="transition-transform" class:rotate-90={showMediaInfo}>&#9654;</span>
      Media Info
    </button>
    {#if showMediaInfo}
      <div class="bg-surface rounded-lg border border-border p-4">
        <div class="grid grid-cols-2 gap-2 text-sm">
          <div><span class="text-text-muted block">Codec</span><span class="font-medium">{probeInfo.codec ?? "-"}</span></div>
          <div><span class="text-text-muted block">FPS</span><span class="font-medium">{probeInfo.fps != null ? probeInfo.fps.toFixed(1) : "-"}</span></div>
        </div>
      </div>
    {/if}
  {/if}

  <!-- Metadata -->
  {#if Object.keys(event.metadata).length > 0}
    <div class="bg-surface rounded-lg border border-border p-3">
      <pre class="text-xs text-text-muted overflow-x-auto">{JSON.stringify(event.metadata, null, 2)}</pre>
    </div>
  {/if}
</div>

{#if renderResult}
  <RenderPlaybackModal render={renderResult} onClose={() => renderResult = null} />
{/if}

