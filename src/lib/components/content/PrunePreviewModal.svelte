<script lang="ts">
  import { pruneGamePreview, pruneGameExecute } from "$lib/ipc/games";
  import type { PrunePreview } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";

  interface Props {
    gameDir: string;
    onClose: () => void;
    onPruned: () => void;
  }

  let { gameDir, onClose, onPruned }: Props = $props();

  let allFiles = $state(false);
  let force = $state(false);
  let preview = $state<PrunePreview | null>(null);
  let loading = $state(true);
  let executing = $state(false);
  let error = $state<string | null>(null);

  async function loadPreview() {
    loading = true;
    error = null;
    try {
      preview = await pruneGamePreview(gameDir, allFiles, force);
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function handleConfirm() {
    executing = true;
    error = null;
    try {
      const result = await pruneGameExecute(gameDir, allFiles, force);
      log.info("Prune", `Removed ${result.file_count} files, freed ${formatBytes(result.total_bytes)}`);
      onPruned();
      onClose();
    } catch (err) {
      error = String(err);
      executing = false;
    }
  }

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  // Load preview on mount, reload when allFiles or force changes
  $effect(() => {
    loadPreview();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
  onclick={handleBackdropClick}
>
  <div class="bg-bg border border-border rounded-xl shadow-2xl max-w-lg w-full mx-4 max-h-[80vh] flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
      <h3 class="text-sm font-semibold">Prune Game</h3>
      <button
        class="text-text-muted hover:text-text transition-colors text-lg leading-none"
        onclick={onClose}
      >&times;</button>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-y-auto px-4 py-3">
      {#if loading}
        <div class="text-center py-8 text-text-muted">
          <div class="w-5 h-5 border-2 border-secondary border-t-transparent rounded-full animate-spin mx-auto mb-2"></div>
          <p class="text-sm">Scanning files...</p>
        </div>
      {:else if error}
        <div class="text-center py-4">
          <p class="text-sm text-accent">{error}</p>
        </div>
      {:else if preview && !preview.eligible}
        <div class="text-center py-8">
          <p class="text-sm text-text-muted">{preview.reason}</p>
        </div>
      {:else if preview}
        <!-- Summary -->
        <div class="mb-3">
          <p class="text-sm">
            {#if preview.file_count === 0}
              Nothing to prune.
            {:else}
              <span class="font-medium">{preview.file_count}</span>
              {preview.file_count === 1 ? "file" : "files"} totaling
              <span class="font-medium">{formatBytes(preview.total_bytes)}</span>
            {/if}
          </p>
        </div>

        <!-- Prune options -->
        <label class="flex items-center gap-2 mb-2 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={force}
            disabled={allFiles}
            class="accent-secondary"
          />
          <div>
            <span class="text-sm">Include untagged clips</span>
            <p class="text-[11px] text-text-muted">Remove event clips that have no event type assigned</p>
          </div>
        </label>
        <label class="flex items-center gap-2 mb-3 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={allFiles}
            class="accent-secondary"
          />
          <div>
            <span class="text-sm">Include all event clips</span>
            <p class="text-[11px] text-text-muted">Removes everything except game.json</p>
          </div>
        </label>

        <!-- File list -->
        {#if preview.files.length > 0}
          <div class="bg-surface rounded-lg border border-border max-h-60 overflow-y-auto">
            <table class="w-full text-xs">
              <thead class="sticky top-0 bg-surface">
                <tr class="border-b border-border text-text-muted">
                  <th class="text-left px-3 py-1.5 font-medium">File</th>
                  <th class="text-right px-3 py-1.5 font-medium w-20">Size</th>
                </tr>
              </thead>
              <tbody>
                {#each preview.files as file}
                  <tr class="border-b border-border/50 hover:bg-bg/50">
                    <td class="px-3 py-1 truncate font-mono" title={file.path}>{file.path}</td>
                    <td class="px-3 py-1 text-right text-text-muted whitespace-nowrap">{formatBytes(file.bytes)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-end gap-2 px-4 py-3 border-t border-border shrink-0">
      <button
        class="px-4 py-2 text-sm text-text-muted hover:text-text transition-colors"
        onclick={onClose}
      >Cancel</button>
      {#if preview?.eligible && preview.file_count > 0}
        <button
          class="px-4 py-2 text-sm font-medium bg-accent hover:bg-accent/80 text-white rounded-lg transition-colors disabled:opacity-50"
          disabled={executing}
          onclick={handleConfirm}
        >
          {executing ? "Pruning..." : `Delete ${preview.file_count} files`}
        </button>
      {/if}
    </div>
  </div>
</div>
