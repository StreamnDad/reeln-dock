<script lang="ts">
  import { games, allTournamentNames, gameStatus, getGamesForTournament } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { isArchived, loadTournamentMetadata, getTournamentMetadata } from "$lib/stores/tournaments.svelte";
  import { settingsTournamentTarget } from "$lib/stores/navigation";
  import TournamentDetailView from "$lib/components/content/TournamentDetailView.svelte";

  let selectedName = $state<string | null>(null);
  let statusFilter = $state<"all" | "active" | "ended">("all");

  const getNames = useStore(allTournamentNames);
  const getGames = useStore(games);
  const getTarget = useStore(settingsTournamentTarget);

  $effect(() => {
    loadTournamentMetadata().catch(() => {});
  });

  // Check for navigation target from sidebar
  $effect(() => {
    const target = getTarget();
    if (target) {
      selectedName = target;
      settingsTournamentTarget.set(null);
    }
  });

  let tournamentNames = $derived(getNames());
  let allGames = $derived(getGames());

  function metaFor(name: string) {
    return getTournamentMetadata().find((m) => m.name === name);
  }

  // Sort: active tournaments with start_date descending, then alphabetically
  let sortedNames = $derived.by(() => {
    return [...tournamentNames].sort((a, b) => {
      const ma = metaFor(a);
      const mb = metaFor(b);
      const da = ma?.start_date || "";
      const db = mb?.start_date || "";
      if (da && db) return db.localeCompare(da); // newest first
      if (da) return -1; // dated before undated
      if (db) return 1;
      return a.localeCompare(b);
    });
  });

  let filteredNames = $derived.by(() => {
    if (statusFilter === "all") return sortedNames;
    return sortedNames.filter((name) => {
      const ended = isArchived(name);
      return statusFilter === "ended" ? ended : !ended;
    });
  });

  function gameCountFor(name: string): number {
    return getGamesForTournament(allGames, name).length;
  }

  function statusBreakdown(name: string): { newCount: number; activeCount: number; doneCount: number } {
    const tGames = getGamesForTournament(allGames, name);
    let newCount = 0;
    let activeCount = 0;
    let doneCount = 0;
    for (const g of tGames) {
      const s = gameStatus(g);
      if (s === "new") newCount++;
      else if (s === "active") activeCount++;
      else if (s === "done") doneCount++;
    }
    return { newCount, activeCount, doneCount };
  }

  let endedCount = $derived(tournamentNames.filter((n) => isArchived(n)).length);
  let activeCount = $derived(tournamentNames.length - endedCount);
</script>

<div class="flex gap-0 h-[calc(100vh-200px)]">
  <!-- Left panel: Tournament list -->
  <div class="w-64 shrink-0 border-r border-border flex flex-col overflow-hidden">
    <!-- Status filter -->
    <div class="px-3 pt-3 pb-2 border-b border-border">
      <div class="flex gap-1.5">
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {statusFilter === 'all' ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => (statusFilter = "all")}
        >
          All <span class="opacity-60 ml-0.5">{tournamentNames.length}</span>
        </button>
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {statusFilter === 'active' ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => (statusFilter = "active")}
        >
          Active <span class="opacity-60 ml-0.5">{activeCount}</span>
        </button>
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {statusFilter === 'ended' ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => (statusFilter = "ended")}
        >
          Ended <span class="opacity-60 ml-0.5">{endedCount}</span>
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-2 py-2">
      {#if tournamentNames.length === 0}
        <p class="text-text-muted text-sm text-center py-4">No tournaments yet. Create one when starting a new game.</p>
      {:else if filteredNames.length === 0}
        <p class="text-text-muted text-sm text-center py-4">No {statusFilter} tournaments</p>
      {:else}
        {#each filteredNames as name (name)}
          {@const isSelected = selectedName === name}
          {@const count = gameCountFor(name)}
          {@const { doneCount } = statusBreakdown(name)}
          {@const meta = metaFor(name)}
          {@const dateRange = meta?.start_date
            ? meta.end_date
              ? `${meta.start_date} \u2013 ${meta.end_date}`
              : meta.start_date
            : ""}
          <button
            class="w-full text-left px-3 py-2.5 rounded-lg mb-1 transition-colors"
            class:bg-primary={isSelected}
            class:hover:bg-surface-hover={!isSelected}
            onclick={() => (selectedName = isSelected ? null : name)}
          >
            <div class="flex items-center gap-2">
              <span class="font-medium text-sm flex-1 truncate">{name}</span>
              {#if isArchived(name)}
                <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-bg text-text-muted border border-border">Ended</span>
              {/if}
            </div>
            <div class="text-xs text-text-muted mt-1">
              {count} game{count !== 1 ? "s" : ""}
              {#if doneCount > 0}
                <span class="text-green-400 ml-2">{doneCount} done</span>
              {/if}
            </div>
            {#if dateRange}
              <div class="text-[10px] text-text-muted mt-0.5">{dateRange}</div>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Right panel: Detail -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if selectedName}
      <TournamentDetailView tournamentName={selectedName} />
    {:else}
      <div class="flex items-center justify-center h-full text-text-muted text-sm">
        <p>Select a tournament to view details.</p>
      </div>
    {/if}
  </div>
</div>
