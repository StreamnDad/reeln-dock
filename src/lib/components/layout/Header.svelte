<script lang="ts">
  import { getConfig } from "$lib/stores/config.svelte";
  import type { View } from "$lib/stores/navigation";

  interface Props {
    currentView: View;
    setView: (v: View) => void;
  }

  let { currentView, setView }: Props = $props();
  let config = $derived(getConfig());

  const tabs: { label: string; view: View }[] = [
    { label: "Games", view: "games" },
    { label: "Plugins", view: "plugins" },
    { label: "Registry", view: "registry" },
    { label: "Settings", view: "settings" },
  ];
</script>

<div class="shrink-0 bg-surface border-b border-border select-none">
  <header class="flex items-center h-12 px-4" data-tauri-drag-region>
    <div class="flex items-center gap-3">
      <img src="/logo.png" alt="reeln" class="w-7 h-7" />
      <span class="font-semibold text-sm tracking-wide">reeln dock</span>
    </div>

    <nav class="flex items-center gap-1 ml-8">
      {#each tabs as tab}
        <button
          class="px-3 py-1.5 text-sm rounded transition-colors"
          class:bg-primary={currentView === tab.view}
          class:text-text={currentView === tab.view}
          class:text-text-muted={currentView !== tab.view}
          class:hover:text-text={currentView !== tab.view}
          onclick={() => setView(tab.view)}
        >
          {tab.label}
        </button>
      {/each}
    </nav>

    <div class="ml-auto text-xs text-text-muted truncate max-w-64">
      {config?.paths.output_dir ?? "No output directory set"}
    </div>
  </header>
</div>
