<script lang="ts">
  import { probeClip, openInFinder } from "$lib/ipc/media";
  import type { MediaInfoResponse } from "$lib/types/media";

  interface Props {
    clipPath: string;
  }

  let { clipPath }: Props = $props();
  let info = $state<MediaInfoResponse | null>(null);
  let error = $state("");

  $effect(() => {
    info = null;
    error = "";
    probeClip(clipPath)
      .then((result) => (info = result))
      .catch((e) => (error = String(e)));
  });

  function formatDuration(secs: number | null): string {
    if (secs == null) return "-";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  }
</script>

<div class="bg-surface rounded-lg border border-border p-4 space-y-4">
  <!-- Thumbnail placeholder -->
  <div class="aspect-video bg-bg rounded-lg flex items-center justify-center">
    <span class="text-4xl text-text-muted">&#127910;</span>
  </div>

  {#if error}
    <p class="text-accent text-sm">{error}</p>
  {:else if info}
    <div class="grid grid-cols-2 gap-2 text-sm">
      <div>
        <span class="text-text-muted">Duration</span>
        <span class="block font-medium">{formatDuration(info.duration_secs)}</span>
      </div>
      <div>
        <span class="text-text-muted">Resolution</span>
        <span class="block font-medium">
          {info.width && info.height ? `${info.width}x${info.height}` : "-"}
        </span>
      </div>
      <div>
        <span class="text-text-muted">Codec</span>
        <span class="block font-medium">{info.codec ?? "-"}</span>
      </div>
      <div>
        <span class="text-text-muted">FPS</span>
        <span class="block font-medium">{info.fps != null ? info.fps.toFixed(1) : "-"}</span>
      </div>
    </div>
  {:else}
    <div class="flex justify-center py-2">
      <div class="w-5 h-5 border-2 border-secondary border-t-transparent rounded-full animate-spin"></div>
    </div>
  {/if}

  <div class="flex gap-2">
    <button
      class="px-3 py-1.5 bg-surface-hover border border-border rounded text-sm hover:bg-border transition-colors"
      onclick={() => openInFinder(clipPath)}
    >
      Open in Finder
    </button>
    <button
      class="px-3 py-1.5 bg-surface-hover border border-border rounded text-sm hover:bg-border transition-colors"
      onclick={() => navigator.clipboard.writeText(clipPath)}
    >
      Copy Path
    </button>
  </div>
</div>
