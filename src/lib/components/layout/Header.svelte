<script lang="ts">
  import { open } from "@tauri-apps/plugin-shell";
  import { getConfig } from "$lib/stores/config.svelte";
  import { getBadgeCount } from "$lib/stores/renderQueue.svelte";
  import { help } from "$lib/help";
  import type { View } from "$lib/stores/navigation";

  interface Props {
    currentView: View;
    setView: (v: View) => void;
  }

  let { currentView, setView }: Props = $props();
  let config = $derived(getConfig());
  let badgeCount = $derived(getBadgeCount());
  let helpOpen = $state(false);

  const tabs: { label: string; view: View }[] = [
    { label: "Games", view: "games" },
    { label: "Queue", view: "queue" },
    { label: "Plugins", view: "plugins" },
    { label: "Registry", view: "registry" },
    { label: "Settings", view: "settings" },
  ];

  const helpLinks: { label: string; key: string }[] = [
    { label: "Documentation", key: "docs.home" },
    { label: "Quick Start Guide", key: "docs.quickstart" },
    { label: "CLI Reference", key: "docs.cli_reference" },
    { label: "Examples", key: "docs.examples" },
  ];

  const communityLinks: { label: string; key: string }[] = [
    { label: "Report an Issue", key: "community.issues" },
    { label: "Join Discussion", key: "community.discussions" },
  ];

  function openHelpLink(key: string) {
    const entry = help[key];
    if (entry?.url) open(entry.url);
    helpOpen = false;
  }
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
          class="px-3 py-1.5 text-sm rounded transition-colors flex items-center gap-1.5"
          class:bg-primary={currentView === tab.view}
          class:text-text={currentView === tab.view}
          class:text-text-muted={currentView !== tab.view}
          class:hover:text-text={currentView !== tab.view}
          onclick={() => setView(tab.view)}
        >
          {tab.label}
          {#if tab.view === "queue" && badgeCount > 0}
            <span class="px-1.5 py-0.5 text-[10px] font-bold rounded-full bg-secondary text-bg leading-none">{badgeCount}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="ml-auto flex items-center gap-3">
      <span class="text-xs text-text-muted truncate max-w-64">
        {config?.paths.output_dir ?? "No output directory set"}
      </span>

      <!-- Help menu -->
      <div class="relative">
        <button
          class="w-7 h-7 rounded-full bg-bg border border-border text-text-muted hover:text-text hover:border-secondary text-xs font-bold transition-colors flex items-center justify-center"
          onclick={() => helpOpen = !helpOpen}
          title="Help & Documentation"
        >?</button>

        {#if helpOpen}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div
            class="fixed inset-0 z-40"
            onclick={() => helpOpen = false}
          ></div>
          <div class="absolute right-0 top-full mt-1 w-56 bg-surface border border-border rounded-lg shadow-lg z-50 py-1">
            {#each helpLinks as link}
              <button
                class="w-full text-left px-4 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
                onclick={() => openHelpLink(link.key)}
              >{link.label}</button>
            {/each}
            <div class="border-t border-border my-1"></div>
            {#each communityLinks as link}
              <button
                class="w-full text-left px-4 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
                onclick={() => openHelpLink(link.key)}
              >{link.label}</button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </header>
</div>
