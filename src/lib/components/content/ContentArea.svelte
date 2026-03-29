<script lang="ts">
  import type { View, SidebarMode } from "$lib/stores/navigation";
  import { selectedGame, selectedEvent } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import GameView from "./GameView.svelte";
  import TournamentView from "./TournamentView.svelte";
  import TeamDetailView from "./TeamDetailView.svelte";
  import TournamentDetailView from "./TournamentDetailView.svelte";
  import ClipReviewPanel from "./ClipReviewPanel.svelte";
  import { getEventTypes } from "$lib/ipc/games";
  import PluginManager from "$lib/components/plugins/PluginManager.svelte";
  import PluginRegistryView from "$lib/components/plugins/PluginRegistryView.svelte";
  import SettingsView from "$lib/components/settings/SettingsView.svelte";

  interface Props {
    currentView: View;
    sidebarMode: SidebarMode;
    selectedTeamKey: string | null;
    selectedTournamentName: string | null;
    setTeamKey: (k: string | null) => void;
    setTournamentName: (n: string | null) => void;
    setSidebarMode: (m: SidebarMode) => void;
  }

  let {
    currentView,
    sidebarMode,
    selectedTeamKey,
    selectedTournamentName,
    setTeamKey,
    setTournamentName,
    setSidebarMode,
  }: Props = $props();

  const getGame = useStore(selectedGame);
  const getEvent = useStore(selectedEvent);

  import type { EventTypeEntry } from "$lib/types/config";
  let configuredEventTypes_ = $state<EventTypeEntry[]>([]);
  $effect(() => {
    getEventTypes()
      .then((types) => { configuredEventTypes_ = types; })
      .catch(() => { configuredEventTypes_ = []; });
  });

  let panelWidth = $state(480);
  let resizing = $state(false);
  let containerEl = $state<HTMLDivElement | null>(null);

  function onDividerPointerDown(e: PointerEvent) {
    resizing = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onDividerPointerMove(e: PointerEvent) {
    if (!resizing || !containerEl) return;
    const containerRect = containerEl.getBoundingClientRect();
    panelWidth = Math.min(containerRect.width * 0.7, Math.max(300, containerRect.right - e.clientX));
  }
  function onDividerPointerUp() { resizing = false; }

  let tournamentViewRef: TournamentView | undefined = $state();

  $effect(() => {
    function handleNewTournamentDrop(e: Event) {
      const detail = (e as CustomEvent).detail;
      tournamentViewRef?.promptNewTournamentForGame(detail.dirPath);
    }
    window.addEventListener("reeln:new-tournament-drop", handleNewTournamentDrop);
    return () => window.removeEventListener("reeln:new-tournament-drop", handleNewTournamentDrop);
  });
</script>

<div class="h-full" bind:this={containerEl}>
  {#if currentView === "plugins"}
    <div class="h-full p-4"><PluginManager /></div>
  {:else if currentView === "registry"}
    <div class="h-full p-4 overflow-y-auto"><PluginRegistryView /></div>
  {:else if currentView === "settings"}
    <div class="h-full p-4 overflow-y-auto"><SettingsView /></div>
  {:else if sidebarMode === "teams" && selectedTeamKey}
    <div class="h-full p-4 overflow-y-auto"><TeamDetailView teamKey={selectedTeamKey} /></div>
  {:else if sidebarMode === "teams"}
    <div class="h-full flex items-center justify-center text-text-muted text-sm">Select a team from the sidebar to view details.</div>
  {:else if sidebarMode === "tournaments" && selectedTournamentName}
    <div class="h-full p-4 overflow-y-auto"><TournamentDetailView tournamentName={selectedTournamentName} /></div>
  {:else if sidebarMode === "tournaments"}
    <div class="h-full flex items-center justify-center text-text-muted text-sm">Select a tournament from the sidebar to view details.</div>
  {:else if getGame() && getEvent()}
    <div class="flex h-full">
      <div class="flex-1 overflow-y-auto p-4 min-w-0"><GameView game={getGame()!} /></div>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="w-1 shrink-0 cursor-col-resize transition-colors hover:bg-secondary" class:bg-secondary={resizing} class:bg-border={!resizing} onpointerdown={onDividerPointerDown} onpointermove={onDividerPointerMove} onpointerup={onDividerPointerUp}></div>
      <div class="shrink-0 overflow-y-auto h-full" style="width: {panelWidth}px"><ClipReviewPanel event={getEvent()!} game={getGame()!} eventTypes={configuredEventTypes_} /></div>
    </div>
  {:else if getGame()}
    <div class="h-full p-4"><GameView game={getGame()!} /></div>
  {:else}
    <div class="h-full p-4"><TournamentView bind:this={tournamentViewRef} /></div>
  {/if}
</div>
