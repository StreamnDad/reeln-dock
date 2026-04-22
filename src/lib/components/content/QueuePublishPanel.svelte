<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { openInFinder, getPlatform, revealLabel, type Platform } from "$lib/ipc/media";
  import VideoPlayer from "./VideoPlayer.svelte";
  import type { CliQueueItem } from "$lib/types/queue";
  import {
    editCliItem,
    publishCliItem,
  } from "$lib/stores/renderQueue.svelte";
  import { listConfigProfiles, listPluginsForProfile, fetchPluginRegistry } from "$lib/ipc/plugins";
  import { help } from "$lib/help";
  import HelpLink from "$lib/components/HelpLink.svelte";

  interface Props {
    item: CliQueueItem;
  }

  let { item }: Props = $props();

  let editingTitle = $state(false);
  let editingDescription = $state(false);
  let titleDraft = $state(item.title);
  let descriptionDraft = $state(item.description);
  let publishing = $state<string | null>(null);
  let error = $state<string | null>(null);
  let discoveredTargets = $state<string[]>([]);
  let loadingTargets = $state(false);
  let targetsError = $state<string | null>(null);
  let videoLoaded = $state(false);
  let platform = $state<Platform>("other");
  let revealButtonLabel = $derived(revealLabel(platform));

  getPlatform().then((p) => { platform = p; }).catch(() => {});

  let videoSrc = $derived(convertFileSrc(item.output));

  // Extract filename from output path for display
  let outputFilename = $derived(item.output.split("/").pop() ?? item.output);

  // Discover publish targets on mount when item has none stored.
  // Checks enabled plugins with hook:POST_RENDER capability (the publish mechanism).
  async function fetchTargets() {
    loadingTargets = true;
    try {
      let profilePath: string | undefined;
      if (item.config_profile) {
        const profiles = await listConfigProfiles();
        profilePath = profiles.find((p) => p.name === item.config_profile)?.path;
      }

      const enabledPlugins = profilePath
        ? await listPluginsForProfile(profilePath)
        : [];
      const enabledNames = new Set(
        enabledPlugins.filter((p) => p.enabled).map((p) => p.name),
      );

      const registry = await fetchPluginRegistry();
      const targets = registry
        .filter((r) => enabledNames.has(r.name) && r.capabilities.includes("uploader"))
        .map((r) => r.name)
        .sort();

      discoveredTargets = targets;
    } catch (err) {
      targetsError = String(err);
      discoveredTargets = [];
    }
    loadingTargets = false;
  }

  if (item.publish_targets.length === 0) {
    fetchTargets();
  }

  function formatSize(bytes: number | null): string {
    if (bytes == null) return "—";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatDuration(seconds: number | null): string {
    if (seconds == null) return "—";
    const m = Math.floor(seconds / 60);
    const s = Math.floor(seconds % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function saveTitle() {
    if (titleDraft === item.title) {
      editingTitle = false;
      return;
    }
    try {
      await editCliItem(item.game_dir, item.id, titleDraft);
      editingTitle = false;
      error = null;
    } catch (err) {
      error = `Failed to save title: ${err}`;
    }
  }

  async function saveDescription() {
    if (descriptionDraft === item.description) {
      editingDescription = false;
      return;
    }
    try {
      await editCliItem(item.game_dir, item.id, undefined, descriptionDraft);
      editingDescription = false;
      error = null;
    } catch (err) {
      error = `Failed to save description: ${err}`;
    }
  }

  async function handlePublishTarget(target: string) {
    publishing = target;
    error = null;
    try {
      await publishCliItem(item.game_dir, item.id, target);
    } catch (err) {
      error = `Publish to ${target} failed: ${err}`;
    }
    publishing = null;
  }

  async function handlePublishAll() {
    publishing = "all";
    error = null;
    try {
      // Publish every target that is NOT already successfully published —
      // covers PENDING, SKIPPED (e.g. config disabled earlier), and FAILED
      // targets in a single click. The CLI's single-target publish path
      // bypasses the pending filter, so per-target invocation is the way
      // to force a retry. Already-published targets are left alone to
      // avoid accidental double-uploads.
      const retryTargets = item.publish_targets.filter(
        (pt) => pt.status !== "published",
      );
      for (const pt of retryTargets) {
        await publishCliItem(item.game_dir, item.id, pt.target);
      }
    } catch (err) {
      error = `Publish failed: ${err}`;
    }
    publishing = null;
  }

  function targetStatusBadge(status: string): string {
    switch (status) {
      case "published": return "bg-green-900/30 text-green-400";
      case "failed": return "bg-red-900/30 text-red-400";
      case "skipped": return "bg-yellow-900/30 text-yellow-400";
      default: return "bg-bg text-text-muted";
    }
  }

  // "Retry All" label kicks in when at least one target has already been
  // touched (skipped/failed/published) — distinguishes a fresh publish
  // from a re-attempt for UX clarity.
  function hasNonPublishedNonPending(
    targets: readonly { status: string }[],
  ): boolean {
    return targets.some(
      (t) => t.status === "skipped" || t.status === "failed",
    );
  }
</script>

<div class="border-t border-border px-4 py-4 space-y-4">
  <!-- Video preview — portrait-aware sizing for 9:16 shorts -->
  <div class="relative w-full max-w-sm mx-auto overflow-hidden rounded bg-bg">
    {#if !videoLoaded}
      <div class="absolute inset-0 flex items-center justify-center text-text-muted text-xs z-10">
        Loading video...
      </div>
    {/if}
    <div class:opacity-0={!videoLoaded}>
      <VideoPlayer src={videoSrc} onloadedmetadata={() => videoLoaded = true} />
    </div>
  </div>

  <!-- Output file path + open -->
  <div class="flex items-center gap-2 text-xs">
    <span class="text-text-muted truncate flex-1" title={item.output}>{outputFilename}</span>
    <button
      class="px-2 py-1 text-text-muted hover:text-text bg-bg rounded transition-colors shrink-0"
      onclick={() => openInFinder(item.output)}
    >{revealButtonLabel}</button>
  </div>

  <!-- Error banner -->
  {#if error}
    <div class="px-3 py-2 bg-red-900/20 border border-red-800/30 rounded text-xs text-red-400">
      {error}
    </div>
  {/if}

  <!-- Title -->
  <div>
    <label class="text-xs text-text-muted block mb-1">Title</label>
    {#if editingTitle}
      <input
        type="text"
        class="w-full px-2 py-1.5 text-sm bg-bg border border-border rounded focus:border-primary focus:outline-none"
        bind:value={titleDraft}
        onblur={saveTitle}
        onkeydown={(e) => { if (e.key === "Enter") saveTitle(); if (e.key === "Escape") { titleDraft = item.title; editingTitle = false; }}}
      />
    {:else}
      <div
        class="text-sm cursor-pointer hover:bg-bg/50 px-2 py-1.5 rounded transition-colors"
        onclick={() => { titleDraft = item.title; editingTitle = true; }}
        title="Click to edit"
      >
        {item.title || "Untitled — click to edit"}
      </div>
    {/if}
  </div>

  <!-- Description -->
  <div>
    <label class="text-xs text-text-muted block mb-1">Description</label>
    {#if editingDescription}
      <textarea
        class="w-full px-2 py-1.5 text-sm bg-bg border border-border rounded focus:border-primary focus:outline-none resize-y min-h-[60px]"
        bind:value={descriptionDraft}
        onblur={saveDescription}
        onkeydown={(e) => { if (e.key === "Escape") { descriptionDraft = item.description; editingDescription = false; }}}
        rows="3"
      ></textarea>
    {:else}
      <div
        class="text-sm cursor-pointer hover:bg-bg/50 px-2 py-1.5 rounded transition-colors whitespace-pre-wrap"
        onclick={() => { descriptionDraft = item.description; editingDescription = true; }}
        title="Click to edit"
      >
        {item.description || "No description — click to edit"}
      </div>
    {/if}
  </div>

  <!-- Metadata -->
  <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
    {#if item.format}
      <span class="text-text-muted">Format</span>
      <span>{item.format}</span>
    {/if}
    {#if item.crop_mode}
      <span class="text-text-muted">Crop</span>
      <span>{item.crop_mode}</span>
    {/if}
    <span class="text-text-muted">Duration</span>
    <span>{formatDuration(item.duration_seconds)}</span>
    <span class="text-text-muted">Size</span>
    <span>{formatSize(item.file_size_bytes)}</span>
    {#if item.render_profile}
      <span class="text-text-muted">Profile</span>
      <span>{item.render_profile}</span>
    {/if}
    {#if item.sport}
      <span class="text-text-muted">Sport</span>
      <span>{item.sport}{item.level ? ` | ${item.level}` : ""}</span>
    {/if}
    {#if item.player}
      <span class="text-text-muted">Player</span>
      <span>{item.player}</span>
    {/if}
    {#if item.assists}
      <span class="text-text-muted">Assists</span>
      <span>{item.assists}</span>
    {/if}
    {#if item.config_profile}
      <span class="text-text-muted">Config</span>
      <span>{item.config_profile}</span>
    {/if}
  </div>

  <!-- Publish targets -->
  <div>
    <div class="flex items-center justify-between mb-2">
      <label class="text-xs text-text-muted">Publish Targets <HelpLink text={help["queue.publish"].text} url={help["queue.publish"].url} /></label>
      {#if item.publish_targets.some((pt) => pt.status !== "published") || discoveredTargets.length > 0}
        <button
          class="px-3 py-1 text-xs bg-primary hover:bg-primary-light text-text rounded transition-colors disabled:opacity-50"
          disabled={publishing !== null || item.status === "publishing"}
          onclick={handlePublishAll}
          title="Publish every target that isn't already successfully published (includes pending, skipped, and failed)"
        >{publishing === "all" ? "Publishing..." : hasNonPublishedNonPending(item.publish_targets) ? "Retry All" : "Publish All"}</button>
      {/if}
    </div>

    {#if item.publish_targets.length === 0 && discoveredTargets.length === 0}
      <p class="text-xs text-text-muted">
        {#if loadingTargets}
          Loading targets...
        {:else if targetsError}
          <span class="text-red-400">{targetsError}</span>
        {:else}
          No publish targets configured. Check your plugin profile.
        {/if}
      </p>
    {:else if item.publish_targets.length === 0 && discoveredTargets.length > 0}
      <div class="space-y-1.5">
        {#each discoveredTargets as target}
          <div class="flex items-center gap-3 px-3 py-2 bg-bg rounded">
            <span class="text-sm font-medium min-w-[80px]">{target}</span>
            <span class="px-2 py-0.5 rounded text-[10px] font-medium bg-bg text-text-muted">pending</span>
            <div class="ml-auto">
              <button
                class="px-2 py-1 text-xs text-secondary hover:text-text bg-surface rounded transition-colors disabled:opacity-50"
                disabled={publishing !== null}
                onclick={() => handlePublishTarget(target)}
              >{publishing === target ? "Publishing..." : "Publish"}</button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="space-y-1.5">
        {#each item.publish_targets as pt}
          <div class="flex items-center gap-3 px-3 py-2 bg-bg rounded">
            <span class="text-sm font-medium min-w-[80px]">{pt.target}</span>
            <span class="px-2 py-0.5 rounded text-[10px] font-medium {targetStatusBadge(pt.status)}">{pt.status}</span>

            {#if pt.url}
              <a
                href={pt.url}
                target="_blank"
                rel="noopener noreferrer"
                class="text-xs text-secondary hover:text-text truncate max-w-48"
                title={pt.url}
              >{pt.url}</a>
            {/if}

            {#if pt.error}
              <span class="text-xs text-red-400 truncate max-w-48" title={pt.error}>{pt.error}</span>
            {/if}

            <div class="ml-auto">
              {#if pt.status === "pending"}
                <button
                  class="px-2 py-1 text-xs text-secondary hover:text-text bg-surface rounded transition-colors disabled:opacity-50"
                  disabled={publishing !== null || item.status === "publishing"}
                  onclick={() => handlePublishTarget(pt.target)}
                >{publishing === pt.target ? "Publishing..." : "Publish"}</button>
              {:else if pt.status === "failed"}
                <button
                  class="px-2 py-1 text-xs text-orange-400 hover:text-text bg-surface rounded transition-colors disabled:opacity-50"
                  disabled={publishing !== null || item.status === "publishing"}
                  onclick={() => handlePublishTarget(pt.target)}
                  title="Retry the upload for this target"
                >{publishing === pt.target ? "Publishing..." : "Retry"}</button>
              {:else if pt.status === "skipped"}
                <button
                  class="px-2 py-1 text-xs text-yellow-400 hover:text-text bg-surface rounded transition-colors disabled:opacity-50"
                  disabled={publishing !== null || item.status === "publishing"}
                  onclick={() => handlePublishTarget(pt.target)}
                  title="Force a publish attempt for this skipped target"
                >{publishing === pt.target ? "Publishing..." : "Retry"}</button>
              {:else if pt.status === "published"}
                <button
                  class="px-2 py-1 text-xs text-text-muted hover:text-text bg-surface rounded transition-colors disabled:opacity-50"
                  disabled={publishing !== null || item.status === "publishing"}
                  onclick={() => handlePublishTarget(pt.target)}
                  title="Re-upload to this target (creates a duplicate)"
                >{publishing === pt.target ? "Publishing..." : "Re-publish"}</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
