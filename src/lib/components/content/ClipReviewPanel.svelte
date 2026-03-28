<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { GameEvent, GameSummary } from "$lib/types/game";
  import type { RenderProfile } from "$lib/types/config";
  // Store functions passed via props now
  import { probeClip, openInFinder } from "$lib/ipc/media";
  import { updateGameEvent } from "$lib/ipc/games";
  import { renderShort, renderPreview, listRenderProfiles } from "$lib/ipc/render";
  import { log } from "$lib/stores/log.svelte";
  import type { MediaInfoResponse } from "$lib/types/media";

  interface Props {
    event: GameEvent;
    game: GameSummary;
    onClose?: () => void;
    onUpdateGame?: (dirPath: string, updater: (g: GameSummary) => GameSummary) => void;
  }

  let { event, game, onClose, onUpdateGame }: Props = $props();
  let probeInfo = $state<MediaInfoResponse | null>(null);
  let probeError = $state("");

  // Editable fields
  let editingField = $state<string | null>(null);
  let editValue = $state("");
  let showSuggestions = $state(false);

  // Auto-play toggle
  let autoPlay = $state(false);

  // Render controls
  let renderProfiles = $state<RenderProfile[]>([]);
  let selectedProfile = $state("");
  let renderLoading = $state(false);
  let renderError = $state("");
  let renderSuccess = $state("");

  $effect(() => {
    listRenderProfiles()
      .then((profiles) => {
        renderProfiles = profiles;
        if (profiles.length > 0 && !selectedProfile) {
          selectedProfile = profiles[0].name;
        }
      })
      .catch(() => {});
  });

  /** Resolve clip path — if relative, join with game dir. */
  function resolveClipPath(clip: string): string {
    if (clip.startsWith("/")) return clip;
    return `${game.dir_path}/${clip}`;
  }

  let fullClipPath = $derived(resolveClipPath(event.clip));
  let videoSrc = $derived(convertFileSrc(fullClipPath));
  let videoError = $state(false);

  // Collect unique event types from game for suggestions
  let knownEventTypes = $derived(
    [...new Set(game.state.events.map((e) => e.event_type).filter(Boolean))].sort(),
  );

  let filteredSuggestions = $derived(
    editingField === "event_type" && editValue
      ? knownEventTypes.filter((t) => t.toLowerCase().includes(editValue.toLowerCase()) && t !== editValue)
      : knownEventTypes,
  );

  // Re-probe when event changes
  $effect(() => {
    const clipPath = fullClipPath;
    probeInfo = null;
    probeError = "";
    videoError = false;
    probeClip(clipPath)
      .then((result) => { probeInfo = result; })
      .catch((e) => { probeError = String(e); });
  });

  function formatDuration(secs: number | null): string {
    if (secs == null) return "-";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

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

  async function handleRenderShort() {
    if (!selectedProfile) return;
    renderLoading = true;
    renderError = "";
    renderSuccess = "";
    try {
      const outputDir = game.dir_path + "/renders";
      const entry = await renderShort(
        fullClipPath,
        outputDir,
        selectedProfile,
        event.id,
        game.dir_path,
      );
      renderSuccess = `Rendered: ${fileName(entry.output)}`;
      // Refresh game state to pick up the new render entry
      const { getGameState } = await import("$lib/ipc/games");
      const newState = await getGameState(game.dir_path);
      onUpdateGame?.(game.dir_path, (g) => ({ ...g, state: newState }));
      log.info("ClipReview", `Rendered short: ${entry.output}`);
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
    try {
      const outputDir = game.dir_path + "/previews";
      const output = await renderPreview(fullClipPath, outputDir);
      renderSuccess = `Preview: ${fileName(output)}`;
      log.info("ClipReview", `Preview rendered: ${output}`);
    } catch (err) {
      renderError = String(err);
      log.error("ClipReview", `Preview failed: ${err}`);
    } finally {
      renderLoading = false;
    }
  }

</script>

<div class="h-full overflow-y-auto space-y-4 p-4">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <h3 class="text-lg font-bold">Clip Review</h3>
    <button
      class="text-sm text-text-muted hover:text-text transition-colors"
      onclick={() => onClose?.()}
    >
      Close &times;
    </button>
  </div>

  <!-- Video Player -->
  <div class="rounded-lg border border-border bg-black overflow-hidden">
    {#if videoError}
      <div class="aspect-video flex items-center justify-center bg-bg">
        <div class="text-center p-4">
          <span class="text-4xl text-text-muted">&#9888;</span>
          <p class="text-accent text-sm mt-2">Could not load video</p>
          {#if probeError}
            <p class="text-text-muted text-xs mt-1">{probeError}</p>
          {/if}
        </div>
      </div>
    {:else}
      {#key event.id}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          class="clip-video"
          src={videoSrc}
          controls
          autoplay={autoPlay}
          playsinline
          preload="metadata"
          onerror={() => { videoError = true; }}
          onloadeddata={() => { videoError = false; }}
        ></video>
      {/key}
    {/if}
  </div>

  <!-- Player controls -->
  <div class="flex items-center gap-3 text-sm">
    <label class="flex items-center gap-1.5 text-text-muted cursor-pointer">
      <input type="checkbox" bind:checked={autoPlay} class="accent-secondary" />
      Auto-play
    </label>
    {#if probeInfo}
      <span class="text-text-muted ml-auto">{formatDuration(probeInfo.duration_secs)}</span>
      <span class="text-text-muted">
        {probeInfo.width && probeInfo.height ? `${probeInfo.width}x${probeInfo.height}` : ""}
      </span>
    {/if}
  </div>

  <!-- Actions -->
  <div class="flex gap-2">
    <button
      class="flex-1 px-3 py-2 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center"
      onclick={() => openInFinder(fullClipPath)}
    >
      Open in Finder
    </button>
    <button
      class="flex-1 px-3 py-2 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center"
      onclick={() => navigator.clipboard.writeText(fullClipPath)}
    >
      Copy Path
    </button>
  </div>

  <!-- Render Controls -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Render</h4>

    {#if renderError}
      <div class="px-3 py-2 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-300">
        {renderError}
      </div>
    {/if}
    {#if renderSuccess}
      <div class="px-3 py-2 bg-green-900/30 border border-green-800 rounded-lg text-sm text-green-300">
        {renderSuccess}
      </div>
    {/if}

    {#if renderProfiles.length > 0}
      <div>
        <label class="block text-xs text-text-muted mb-1" for="render-profile">Profile</label>
        <select
          id="render-profile"
          bind:value={selectedProfile}
          class="w-full px-2 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
        >
          {#each renderProfiles as profile}
            <option value={profile.name}>{profile.name}{profile.width && profile.height ? ` (${profile.width}x${profile.height})` : ""}</option>
          {/each}
        </select>
      </div>
    {/if}

    <div class="flex gap-2">
      <button
        class="flex-1 px-3 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50 text-center"
        disabled={renderLoading || !selectedProfile}
        onclick={handleRenderShort}
      >
        {renderLoading ? "Rendering..." : "Render Short"}
      </button>
      <button
        class="flex-1 px-3 py-2 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors disabled:opacity-50 text-center"
        disabled={renderLoading}
        onclick={handleRenderPreview}
      >
        Preview
      </button>
    </div>
  </div>

  <!-- Event Details -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Event Details</h4>

    <div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-sm">
      <!-- Type (combobox with suggestions) -->
      <span class="text-text-muted">Type</span>
      {#if editingField === "event_type"}
        <div class="relative">
          <!-- svelte-ignore a11y_autofocus -->
          <input
            type="text"
            bind:value={editValue}
            class="w-full px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
            onkeydown={(e) => {
              if (e.key === "Enter") commitEdit();
              if (e.key === "Escape") cancelEdit();
            }}
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
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="font-medium cursor-text hover:underline hover:decoration-dotted"
          ondblclick={() => startEdit("event_type", event.event_type)}
        >
          {event.event_type || "clip"}
        </span>
      {/if}

      <!-- Player -->
      <span class="text-text-muted">Player</span>
      {#if editingField === "player"}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={editValue}
          class="px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
          onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
          onblur={commitEdit}
          autofocus
        />
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="cursor-text hover:underline hover:decoration-dotted"
          ondblclick={() => startEdit("player", event.player)}
        >
          {event.player || "-"}
        </span>
      {/if}

      <!-- Segment -->
      <span class="text-text-muted">Segment</span>
      <span>{event.segment_number}</span>

      <!-- Clip path -->
      <span class="text-text-muted">File</span>
      {#if editingField === "clip"}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={editValue}
          class="px-1 py-0.5 bg-bg border border-secondary rounded text-sm text-text focus:outline-none"
          onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
          onblur={commitEdit}
          autofocus
        />
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="text-text-muted truncate cursor-text hover:underline hover:decoration-dotted"
          title={event.clip}
          ondblclick={() => startEdit("clip", event.clip)}
        >
          {fileName(event.clip)}
        </span>
      {/if}

      <!-- Created -->
      <span class="text-text-muted">Created</span>
      <span class="text-text-muted">{event.created_at || "-"}</span>

      <!-- ID -->
      <span class="text-text-muted">ID</span>
      <span class="text-text-muted font-mono text-xs">{event.id}</span>
    </div>
  </div>

  <!-- Media Info -->
  {#if probeInfo}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Media Info</h4>
      <div class="grid grid-cols-2 gap-2 text-sm">
        <div>
          <span class="text-text-muted block">Codec</span>
          <span class="font-medium">{probeInfo.codec ?? "-"}</span>
        </div>
        <div>
          <span class="text-text-muted block">FPS</span>
          <span class="font-medium">{probeInfo.fps != null ? probeInfo.fps.toFixed(1) : "-"}</span>
        </div>
      </div>
    </div>
  {/if}

  <!-- Metadata -->
  {#if Object.keys(event.metadata).length > 0}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Metadata</h4>
      <pre class="text-xs text-text-muted overflow-x-auto">{JSON.stringify(event.metadata, null, 2)}</pre>
    </div>
  {/if}
</div>

<style>
  .clip-video {
    display: block;
    width: 100%;
    max-width: none;
  }
</style>
