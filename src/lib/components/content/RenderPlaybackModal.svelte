<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { RenderEntry } from "$lib/types/game";
  import { openInFinder } from "$lib/ipc/media";
  import VideoPlayer from "./VideoPlayer.svelte";

  interface Props {
    render: RenderEntry;
    onClose: () => void;
  }

  let { render, onClose }: Props = $props();

  let videoSrc = $derived(convertFileSrc(render.output));
  let videoError = $state(false);
  let expanded = $state(false);

  function fileName(path: string): string {
    return path.split("/").pop() || path;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (expanded) {
        expanded = false;
      } else {
        onClose();
      }
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
  onclick={handleBackdropClick}
>
  <div
    class="bg-bg border border-border shadow-2xl overflow-hidden transition-all duration-200 flex flex-col"
    class:rounded-xl={!expanded}
    class:max-w-2xl={!expanded}
    class:w-full={!expanded}
    class:mx-4={!expanded}
    class:max-h-[90vh]={!expanded}
    class:inset-4={expanded}
    class:fixed={expanded}
    class:rounded-lg={expanded}
    style={expanded ? "max-width: none; margin: 0;" : ""}
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2.5 border-b border-border shrink-0">
      <h3 class="text-sm font-semibold truncate mr-4">{render.format} &middot; {fileName(render.output)}</h3>
      <div class="flex items-center gap-2 shrink-0">
        <button
          class="text-xs text-text-muted hover:text-text transition-colors"
          onclick={() => expanded = !expanded}
          title={expanded ? "Collapse" : "Expand"}
        >{expanded ? "Collapse" : "Expand"}</button>
        <button
          class="text-text-muted hover:text-text transition-colors text-lg leading-none"
          onclick={onClose}
        >&times;</button>
      </div>
    </div>

    <!-- Video -->
    <div class="bg-black flex-1 min-h-0 overflow-hidden" style={expanded ? "display: flex; flex-direction: column;" : ""}>
      {#if videoError}
        <div class="aspect-video flex items-center justify-center">
          <div class="text-center p-4">
            <span class="text-4xl text-text-muted">&#9888;</span>
            <p class="text-accent text-sm mt-2">Could not load render — file may have been moved or deleted</p>
          </div>
        </div>
      {:else}
        <VideoPlayer
          src={videoSrc}
          autoplay={true}
          class={expanded ? "max-h-[calc(100vh-12rem)]" : "max-h-[60vh]"}
          onerror={() => { videoError = true; }}
        />
      {/if}
    </div>

    <!-- Metadata + Actions -->
    <div class="px-4 py-3 space-y-2 border-t border-border shrink-0">
      <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
        <div>
          <span class="text-text-muted">Format</span>
          <span class="ml-2 font-medium">{render.format}</span>
        </div>
        <div>
          <span class="text-text-muted">Crop</span>
          <span class="ml-2">{render.crop_mode || "-"}</span>
        </div>
        <div>
          <span class="text-text-muted">Segment</span>
          <span class="ml-2">{render.segment_number}</span>
        </div>
        <div>
          <span class="text-text-muted">Rendered</span>
          <span class="ml-2">{render.rendered_at}</span>
        </div>
      </div>

      <div class="text-xs text-text-muted truncate" title={render.output}>
        {render.output}
      </div>

      <div class="flex gap-2">
        <button
          class="flex-1 px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center"
          onclick={() => openInFinder(render.output)}
        >Open in Finder</button>
        <button
          class="flex-1 px-3 py-1.5 bg-surface border border-border rounded-lg text-sm hover:bg-surface-hover transition-colors text-center"
          onclick={() => navigator.clipboard.writeText(render.output)}
        >Copy Path</button>
      </div>
    </div>
  </div>
</div>
