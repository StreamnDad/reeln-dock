<script lang="ts">
  import { deleteGamePreview, deleteGame } from "$lib/ipc/games";
  import type { DeletePreview } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";

  interface Props {
    gameDir: string;
    onClose: () => void;
    onDeleted: () => void;
  }

  let { gameDir, onClose, onDeleted }: Props = $props();

  let preview = $state<DeletePreview | null>(null);
  let loading = $state(true);
  let deleting = $state(false);
  let error = $state<string | null>(null);
  let confirmText = $state("");

  async function loadPreview() {
    loading = true;
    error = null;
    try {
      preview = await deleteGamePreview(gameDir);
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function confirmMatch(): boolean {
    return confirmText.trim().toLowerCase() === "delete";
  }

  async function handleDelete() {
    if (!confirmMatch()) return;
    deleting = true;
    error = null;
    try {
      await deleteGame(gameDir);
      log.info("Delete", `Deleted game: ${preview?.home_team} vs ${preview?.away_team}`);
      onDeleted();
      onClose();
    } catch (err) {
      error = String(err);
      deleting = false;
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

  $effect(() => {
    loadPreview();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
  onclick={handleBackdropClick}
>
  <div class="bg-bg border border-border rounded-xl shadow-2xl max-w-lg w-full mx-4 flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
      <h3 class="text-sm font-semibold text-accent">Delete Game</h3>
      <button
        class="text-text-muted hover:text-text transition-colors text-lg leading-none"
        onclick={onClose}
      >&times;</button>
    </div>

    <!-- Body -->
    <div class="px-4 py-3">
      {#if loading}
        <div class="text-center py-8 text-text-muted">
          <div class="w-5 h-5 border-2 border-secondary border-t-transparent rounded-full animate-spin mx-auto mb-2"></div>
          <p class="text-sm">Loading game info...</p>
        </div>
      {:else if error}
        <div class="text-center py-4">
          <p class="text-sm text-accent">{error}</p>
        </div>
      {:else if preview}
        <div class="bg-accent/10 border border-accent/30 rounded-lg p-3 mb-3">
          <p class="text-sm font-medium text-accent">This action is permanent and cannot be undone.</p>
          <p class="text-xs text-text-muted mt-1">The entire game directory and all its contents will be deleted.</p>
        </div>

        <div class="bg-surface rounded-lg border border-border p-3 mb-3 space-y-1">
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">Game</span>
            <span class="font-medium">{preview.home_team} vs {preview.away_team}</span>
          </div>
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">Date</span>
            <span>{preview.date}</span>
          </div>
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">Files</span>
            <span>{preview.file_count} {preview.file_count === 1 ? "file" : "files"}</span>
          </div>
          <div class="flex justify-between text-sm">
            <span class="text-text-muted">Total size</span>
            <span class="font-medium">{formatBytes(preview.total_bytes)}</span>
          </div>
        </div>

        <div class="mb-1">
          <label for="confirm-delete" class="text-xs text-text-muted">
            Type <span class="font-mono font-medium text-text">delete</span> to confirm
          </label>
          <input
            id="confirm-delete"
            type="text"
            bind:value={confirmText}
            class="w-full mt-1 px-3 py-2 text-sm bg-surface border border-border rounded-lg focus:outline-none focus:border-secondary"
            placeholder="delete"
            autocomplete="off"
            disabled={deleting}
          />
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-end gap-2 px-4 py-3 border-t border-border shrink-0">
      <button
        class="px-4 py-2 text-sm text-text-muted hover:text-text transition-colors"
        onclick={onClose}
      >Cancel</button>
      {#if preview && !loading}
        <button
          class="px-4 py-2 text-sm font-medium bg-accent hover:bg-accent/80 text-white rounded-lg transition-colors disabled:opacity-50"
          disabled={!confirmMatch() || deleting}
          onclick={handleDelete}
        >
          {deleting ? "Deleting..." : "Delete Game"}
        </button>
      {/if}
    </div>
  </div>
</div>
