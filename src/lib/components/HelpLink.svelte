<script lang="ts">
  import { open } from "@tauri-apps/plugin-shell";
  import { getShowHelpTips } from "$lib/stores/uiPrefs.svelte";

  interface Props {
    text: string;
    url?: string;
  }

  let { text, url }: Props = $props();
  let showTip = $state(false);
  let hideTimeout: ReturnType<typeof setTimeout> | null = null;

  function show() {
    if (!getShowHelpTips()) return;
    if (hideTimeout) { clearTimeout(hideTimeout); hideTimeout = null; }
    showTip = true;
  }

  function hide() {
    hideTimeout = setTimeout(() => { showTip = false; }, 150);
  }

  function openDocs() {
    if (url) open(url);
  }
</script>

<!-- Inline help icon with tooltip — hover zone covers both icon and popup -->
<span
  class="relative inline-flex items-center ml-1"
  onmouseenter={show}
  onmouseleave={hide}
>
  <button
    type="button"
    class="w-4 h-4 rounded-full bg-border text-text-muted text-[10px] font-bold leading-none hover:bg-secondary hover:text-text transition-colors flex items-center justify-center"
    onclick={url ? openDocs : undefined}
    title={text}
  >?</button>

  {#if showTip}
    <!-- Invisible bridge fills the gap between button and tooltip -->
    <div class="absolute bottom-full left-1/2 -translate-x-1/2 w-64 h-2"></div>
    <div class="absolute z-50 bottom-full left-1/2 -translate-x-1/2 mb-2 w-64 p-2.5 bg-surface border border-border rounded-lg shadow-lg text-xs text-text">
      <p>{text}</p>
      {#if url}
        <button
          type="button"
          class="mt-1.5 text-secondary hover:underline text-[11px]"
          onclick={openDocs}
        >Learn more in docs &rarr;</button>
      {/if}
    </div>
  {/if}
</span>
