<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onDestroy } from "svelte";

  interface Props {
    logoPath: string | null;
    disabled?: boolean;
    onChange: (path: string | null) => Promise<void>;
  }

  let { logoPath, disabled = false, onChange }: Props = $props();
  let busy = $state(false);
  let error = $state("");
  let revision = $state(0);
  let destroyed = false;

  onDestroy(() => { destroyed = true; });

  async function changeLogo(remove = false) {
    busy = true;
    error = "";
    try {
      const path = remove ? null : await open({
        title: "Select game sponsor logo",
        filters: [{ name: "PNG image", extensions: ["png"] }],
        multiple: false,
      });
      if (destroyed || (!remove && typeof path !== "string")) return;
      await onChange(typeof path === "string" ? path : null);
      revision = Date.now();
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="space-y-2">
  <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
    Sponsor logo <span class="normal-case font-normal tracking-normal">(optional)</span>
  </h3>
  <p class="text-xs text-text-muted">Choose an approved sRGB PNG with minimal padding. Dock fits it proportionally in the bottom-right corner.</p>
  <div class="flex flex-wrap items-center gap-2">
    {#if logoPath}
      <img
        src={`${convertFileSrc(logoPath)}?v=${revision}`}
        alt="Selected sponsor logo"
        title={logoPath}
        class="max-h-10 max-w-28 object-contain"
      />
    {/if}
    <button
      type="button"
      class="px-2 py-1 text-xs border border-border rounded text-text hover:border-secondary disabled:opacity-50"
      disabled={disabled || busy}
      onclick={() => changeLogo()}
    >{logoPath ? "Replace logo" : "Choose logo"}</button>
    {#if logoPath}
      <button
        type="button"
        class="px-2 py-1 text-xs text-text-muted hover:text-text disabled:opacity-50"
        disabled={disabled || busy}
        onclick={() => changeLogo(true)}
      >Remove</button>
    {/if}
  </div>
  {#if error}
    <p class="text-xs text-red-300" role="alert">{error}</p>
  {/if}
</div>
