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
  let buttonEl = $state<HTMLButtonElement | null>(null);
  let tipX = $state(0);
  let tipY = $state(0);
  let flipBelow = $state(false);

  function show() {
    if (!getShowHelpTips()) return;
    if (hideTimeout) { clearTimeout(hideTimeout); hideTimeout = null; }
    if (buttonEl) {
      const rect = buttonEl.getBoundingClientRect();
      const tipWidth = 256; // w-64 = 16rem = 256px
      // Center on button, but clamp so tooltip stays within viewport
      let x = rect.left + rect.width / 2;
      x = Math.max(tipWidth / 2 + 8, Math.min(x, window.innerWidth - tipWidth / 2 - 8));
      tipX = x;
      // If too close to top, flip tooltip below
      flipBelow = rect.top < 120;
      tipY = flipBelow ? rect.bottom + 8 : rect.top - 8;
    }
    showTip = true;
  }

  function hide() {
    hideTimeout = setTimeout(() => { showTip = false; }, 150);
  }

  function openDocs() {
    if (url) open(url);
  }
</script>

<!-- Inline help icon with tooltip — hidden when help tips are disabled -->
{#if getShowHelpTips()}
<span
  class="inline-flex items-center ml-1"
  onmouseenter={show}
  onmouseleave={hide}
>
  <button
    bind:this={buttonEl}
    type="button"
    class="w-4 h-4 rounded-full bg-border text-text-muted text-[10px] font-bold leading-none hover:bg-secondary hover:text-text transition-colors flex items-center justify-center"
    onclick={url ? openDocs : undefined}
  >?</button>
</span>

{#if showTip}
  <!-- Fixed-position tooltip rendered at viewport level — never clipped by parent overflow -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed z-[9999] w-64"
    style="left: {tipX}px; top: {tipY}px; transform: translateX(-50%) {flipBelow ? '' : 'translateY(-100%)'};"
    onmouseenter={show}
    onmouseleave={hide}
  >
    {#if !flipBelow}
      <!-- Bridge above button when tooltip is above -->
      <div class="h-2"></div>
    {/if}
    <div class="p-2.5 bg-surface border border-border rounded-lg shadow-lg text-xs text-text">
      <p>{text}</p>
      {#if url}
        <button
          type="button"
          class="mt-1.5 text-secondary hover:underline text-[11px]"
          onclick={openDocs}
        >Learn more in docs &rarr;</button>
      {/if}
    </div>
    {#if flipBelow}
      <!-- Bridge below button when tooltip is below -->
      <div class="h-2"></div>
    {/if}
  </div>
{/if}
{/if}
