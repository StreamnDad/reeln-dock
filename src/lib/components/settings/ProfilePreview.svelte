<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import VideoPlayer from "$lib/components/content/VideoPlayer.svelte";
  import AspectRatioPreview from "./AspectRatioPreview.svelte";
  import ClipBrowser from "./ClipBrowser.svelte";
  import { renderProfilePreview, deletePreview, suggestPreviewClip } from "$lib/ipc/render";
  import type { RenderProfile } from "$lib/types/config";

  interface Props {
    profile: Partial<RenderProfile>;
    sampleClip: string;
    onsamplechange: (clip: string) => void;
  }

  let { profile, sampleClip, onsamplechange }: Props = $props();

  let previewPath = $state("");
  let rendering = $state(false);
  let previewError = $state("");
  let showClipBrowser = $state(false);

  let suggestAttempted = false;

  // Video source URLs
  let sourceVideoSrc = $derived(sampleClip ? convertFileSrc(sampleClip) : "");
  let previewVideoSrc = $derived(previewPath ? convertFileSrc(previewPath) : "");

  // Auto-suggest a clip once on mount if none set
  $effect(() => {
    if (!sampleClip && !suggestAttempted) {
      suggestAttempted = true;
      suggestPreviewClip()
        .then((clip) => { if (clip) onsamplechange(clip); })
        .catch(() => {});
    }
  });

  async function renderPreviewClip() {
    if (!sampleClip || rendering) return;
    rendering = true;
    previewError = "";

    try {
      if (previewPath) {
        await deletePreview(previewPath).catch(() => {});
      }

      const outputDir = sampleClip.substring(0, sampleClip.lastIndexOf("/"));
      const result = await renderProfilePreview(sampleClip, outputDir, profile);
      previewPath = result;
    } catch (e) {
      previewError = String(e);
      previewPath = "";
    }
    rendering = false;
  }

  async function pickSampleClip() {
    const result = await open({
      title: "Select sample clip",
      filters: [{ name: "Video", extensions: ["mp4", "mkv", "mov", "avi", "webm"] }],
      directory: false,
      multiple: false,
    });
    if (result) {
      onsamplechange(result as string);
      previewPath = "";
    }
  }

  function handleClipBrowserSelect(clipPath: string) {
    onsamplechange(clipPath);
    showClipBrowser = false;
    previewPath = "";
  }

  function clipName(path: string): string {
    if (!path) return "";
    const parts = path.split("/");
    return parts[parts.length - 1] ?? path;
  }
</script>

<div class="space-y-4">
  <!-- Sample clip selection -->
  <div>
    <span class="block text-xs text-text-muted mb-1">Sample Clip</span>
    <div class="flex gap-1.5">
      <div
        class="flex-1 px-2 py-1.5 bg-bg border border-border rounded text-xs text-text truncate"
        title={sampleClip}
      >
        {sampleClip ? clipName(sampleClip) : "No clip selected"}
      </div>
      <button
        class="px-2 py-1.5 bg-bg border border-border rounded text-xs text-text-muted hover:text-text hover:border-secondary transition-colors"
        onclick={() => { showClipBrowser = true; }}
        title="Browse clips by tournament, game, or event type"
      >Clips</button>
      <button
        class="px-2 py-1.5 bg-bg border border-border rounded text-xs text-text-muted hover:text-text hover:border-secondary transition-colors"
        onclick={pickSampleClip}
        title="Browse filesystem"
      >File</button>
    </div>
  </div>

  <!-- Source clip — always show when a clip is selected -->
  {#if sampleClip && sourceVideoSrc}
    <div>
      <span class="block text-[10px] text-text-muted mb-1">Source</span>
      <div class="rounded overflow-hidden border border-border">
        <VideoPlayer src={sourceVideoSrc} />
      </div>
    </div>
  {/if}

  <!-- CSS Aspect Ratio Preview -->
  <AspectRatioPreview
    width={profile.width}
    height={profile.height}
    cropMode={profile.crop_mode}
    padColor={profile.pad_color}
  />

  <!-- Render Preview -->
  {#if sampleClip}
    <button
      class="w-full px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded text-sm font-medium transition-colors disabled:opacity-50"
      onclick={renderPreviewClip}
      disabled={rendering}
    >
      {rendering ? "Rendering..." : "Render Preview"}
    </button>
  {/if}

  {#if previewError}
    <p class="text-xs text-red-400">{previewError}</p>
  {/if}

  {#if previewVideoSrc}
    <div>
      <span class="block text-[10px] text-text-muted mb-1">Preview with Profile</span>
      <div class="rounded overflow-hidden border border-border">
        <VideoPlayer src={previewVideoSrc} />
      </div>
    </div>
  {/if}

  {#if !sampleClip}
    <p class="text-xs text-text-muted text-center py-4">Select a sample clip to preview rendering.</p>
  {/if}
</div>

<!-- Clip Browser Modal -->
{#if showClipBrowser}
  <ClipBrowser
    onselect={handleClipBrowserSelect}
    onclose={() => { showClipBrowser = false; }}
  />
{/if}
