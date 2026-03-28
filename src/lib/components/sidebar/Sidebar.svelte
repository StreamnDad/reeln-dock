<script lang="ts">
  import type { SidebarMode } from "$lib/stores/navigation";
  import GamesSidebar from "./GamesSidebar.svelte";
  import TeamsSidebar from "./TeamsSidebar.svelte";
  import TournamentsSidebar from "./TournamentsSidebar.svelte";

  interface Props {
    sidebarMode: SidebarMode;
    setSidebarMode: (m: SidebarMode) => void;
  }

  let { sidebarMode, setSidebarMode }: Props = $props();

  const modes: { label: string; value: SidebarMode }[] = [
    { label: "Games", value: "games" },
    { label: "Teams", value: "teams" },
    { label: "Tournaments", value: "tournaments" },
  ];
</script>

<div class="flex flex-col h-full bg-surface">
  <div class="px-3 pt-2 pb-1 border-b border-border">
    <div class="flex gap-0.5 bg-bg rounded-lg p-0.5">
      {#each modes as m}
        <button
          class="flex-1 px-2 py-1 rounded-md text-xs font-medium transition-colors text-center"
          class:bg-primary={sidebarMode === m.value}
          class:text-text={sidebarMode === m.value}
          class:text-text-muted={sidebarMode !== m.value}
          class:hover:text-text={sidebarMode !== m.value}
          onclick={() => setSidebarMode(m.value)}
        >
          {m.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="flex-1 min-h-0 overflow-hidden flex flex-col">
    {#if sidebarMode === "games"}
      <GamesSidebar />
    {:else if sidebarMode === "teams"}
      <TeamsSidebar />
    {:else}
      <TournamentsSidebar />
    {/if}
  </div>
</div>
