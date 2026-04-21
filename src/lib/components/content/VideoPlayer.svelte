<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { preparePreviewProxy } from "$lib/ipc/media";

  const WEB_PLAYABLE = new Set(["mp4", "mov", "webm"]);

  interface Props {
    src: string;
    /** Raw filesystem path (before convertFileSrc). When set, triggers proxy generation for non-web formats. */
    originalPath?: string;
    autoplay?: boolean;
    class?: string;
    onended?: () => void;
    onerror?: () => void;
    onloadeddata?: () => void;
    onloadedmetadata?: () => void;
  }

  let {
    src,
    originalPath,
    autoplay = false,
    class: className = "",
    onended,
    onerror,
    onloadeddata,
    onloadedmetadata,
  }: Props = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let currentTime = $state(0);
  let duration = $state(0);
  let hovered = $state(false);
  let seeking = $state(false);
  let proxyLoading = $state(false);
  let proxySrc = $state<string | null>(null);
  let proxyError = $state<string | null>(null);

  let progress = $derived(duration > 0 ? currentTime / duration : 0);

  /** Effective video src — uses proxy when available. */
  let effectiveSrc = $derived(proxySrc ?? src);

  function needsProxy(path: string | undefined): boolean {
    if (!path) return false;
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    return ext !== "" && !WEB_PLAYABLE.has(ext);
  }

  $effect(() => {
    // Reset proxy state when source changes
    proxySrc = null;
    proxyError = null;

    if (!needsProxy(originalPath)) return;

    proxyLoading = true;
    preparePreviewProxy(originalPath!)
      .then((playablePath) => {
        proxySrc = convertFileSrc(playablePath);
        proxyLoading = false;
      })
      .catch((err) => {
        proxyError = String(err);
        proxyLoading = false;
      });
  });

  function togglePlay() {
    if (!videoEl) return;
    if (videoEl.paused) {
      videoEl.play().catch(() => {});
    } else {
      videoEl.pause();
    }
  }

  function handleTimeUpdate() {
    if (!videoEl || seeking) return;
    currentTime = videoEl.currentTime;
  }

  function handleLoadedMetadata() {
    if (!videoEl) return;
    duration = videoEl.duration;
    onloadedmetadata?.();
  }


  function seekTo(e: MouseEvent) {
    if (!videoEl || duration <= 0) return;
    const bar = e.currentTarget as HTMLElement;
    const rect = bar.getBoundingClientRect();
    const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    videoEl.currentTime = fraction * duration;
    currentTime = videoEl.currentTime;
  }

  function handleBarPointerDown(e: PointerEvent) {
    seeking = true;
    seekTo(e as unknown as MouseEvent);
    const bar = e.currentTarget as HTMLElement;
    bar.setPointerCapture(e.pointerId);
  }

  function handleBarPointerMove(e: PointerEvent) {
    if (!seeking) return;
    seekTo(e as unknown as MouseEvent);
  }

  function handleBarPointerUp() {
    seeking = false;
  }
</script>

<div
  class="relative overflow-hidden {className}"
  role="button"
  tabindex="0"
  onmouseenter={() => hovered = true}
  onmouseleave={() => { if (!seeking) hovered = false; }}
  onclick={togglePlay}
  onkeydown={(e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); togglePlay(); } }}
>
  {#if proxyLoading}
    <div class="flex items-center justify-center bg-bg/80 aspect-video w-full">
      <div class="text-center text-text-muted">
        <div class="w-6 h-6 border-2 border-secondary border-t-transparent rounded-full animate-spin mx-auto mb-2"></div>
        <p class="text-xs">Preparing preview...</p>
      </div>
    </div>
  {:else if proxyError}
    <div class="flex items-center justify-center bg-bg/80 aspect-video w-full">
      <p class="text-xs text-accent text-center px-4">Could not prepare preview: {proxyError}</p>
    </div>
  {:else}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video
      bind:this={videoEl}
      src={effectiveSrc}
      autoplay={autoplay}
      playsinline
      preload="metadata"
      class="block w-full"
      ontimeupdate={handleTimeUpdate}
      onloadedmetadata={handleLoadedMetadata}
      onended={() => { onended?.(); }}
      onerror={() => onerror?.()}
      onloadeddata={() => onloadeddata?.()}
    ></video>
  {/if}

  <!-- Thin progress bar at bottom -->
  <div
    class="absolute bottom-0 left-0 right-0 h-1 transition-opacity duration-200"
    class:opacity-100={hovered || seeking}
    class:opacity-0={!hovered && !seeking}
    onclick={(e) => e.stopPropagation()}
    onpointerdown={handleBarPointerDown}
    onpointermove={handleBarPointerMove}
    onpointerup={handleBarPointerUp}
    role="slider"
    tabindex="-1"
    aria-valuenow={Math.round(progress * 100)}
    aria-valuemin={0}
    aria-valuemax={100}
    aria-label="Video progress"
  >
    <div class="absolute inset-0 bg-black/40"></div>
    <div
      class="absolute top-0 left-0 bottom-0 bg-secondary"
      style="width: {progress * 100}%"
    ></div>
    <!-- Hover target: taller invisible area for easier clicking -->
    <div class="absolute -top-2 left-0 right-0 bottom-0 cursor-pointer"></div>
  </div>
</div>
