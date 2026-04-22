<script lang="ts">
  import { open } from "@tauri-apps/plugin-shell";

  interface Props {
    text: string;
    url?: string;
  }

  let { text, url }: Props = $props();
  let showTip = $state(false);

  function openDocs() {
    if (url) open(url);
  }
</script>

<!-- Inline help icon with tooltip -->
<span class="relative inline-flex items-center">
  <button
    type="button"
    class="w-4 h-4 rounded-full bg-border text-text-muted text-[10px] font-bold leading-none hover:bg-secondary hover:text-text transition-colors flex items-center justify-center"
    onmouseenter={() => showTip = true}
    onmouseleave={() => showTip = false}
    onclick={url ? openDocs : undefined}
    title={text}
  >?</button>

  {#if showTip}
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
