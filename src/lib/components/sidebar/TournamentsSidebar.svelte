<script lang="ts">
  import { games, allTournamentNames, levels, gameStatus, getGamesForTournament } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { isArchived } from "$lib/stores/tournaments.svelte";
  import { selectedTournamentName } from "$lib/stores/navigation";

  type StatusFilter = "all" | "active" | "archived";

  const getAllTournaments = useStore(allTournamentNames);
  const getSelectedName = useStore(selectedTournamentName);
  const getGames = useStore(games);
  const getLevels = useStore(levels);

  let statusFilterLocal = $state<StatusFilter>("active");
  let search = $state("");
  let selectedLevel = $state<string | null>(null);

  interface TournamentEntry {
    name: string;
    gameCount: number;
    activeCount: number;
    doneCount: number;
    newCount: number;
    archived: boolean;
    levels: Set<string>;
  }

  let allEntries = $derived.by(() => {
    const result: TournamentEntry[] = [];
    for (const name of getAllTournaments()) {
      const tGames = getGamesForTournament(getGames(), name);
      const archived = isArchived(name);

      let activeCount = 0;
      let doneCount = 0;
      let newCount = 0;
      const tourneyLevels = new Set<string>();
      for (const g of tGames) {
        const s = gameStatus(g);
        if (s === "active") activeCount++;
        else if (s === "done") doneCount++;
        else newCount++;
        if (g.state.game_info.level) tourneyLevels.add(g.state.game_info.level);
      }

      result.push({
        name,
        gameCount: tGames.length,
        activeCount,
        doneCount,
        newCount,
        archived,
        levels: tourneyLevels,
      });
    }
    return result.sort((a, b) => a.name.localeCompare(b.name));
  });

  // Filter counts for pills
  let activeTourneyCount = $derived(allEntries.filter((e) => !e.archived).length);
  let archivedTourneyCount = $derived(allEntries.filter((e) => e.archived).length);

  let filteredEntries = $derived.by(() => {
    return allEntries.filter((entry) => {
      // Status filter
      if (statusFilterLocal === "active" && entry.archived) return false;
      if (statusFilterLocal === "archived" && !entry.archived) return false;

      // Level filter
      if (selectedLevel && !entry.levels.has(selectedLevel)) return false;

      // Search
      if (search) {
        const q = search.toLowerCase();
        if (!entry.name.toLowerCase().includes(q)) return false;
      }

      return true;
    });
  });

  const statusOptions: { label: string; value: StatusFilter; count: number }[] = $derived([
    { label: "Active", value: "active", count: activeTourneyCount },
    { label: "Archived", value: "archived", count: archivedTourneyCount },
    { label: "All", value: "all", count: allEntries.length },
  ]);
</script>

<div class="flex flex-col h-full">
  <!-- Status filter -->
  <div class="px-3 pt-2 pb-1">
    <div class="flex gap-1">
      {#each statusOptions as opt}
        <button
          class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
          class:bg-primary={statusFilterLocal === opt.value}
          class:text-text={statusFilterLocal === opt.value}
          class:text-text-muted={statusFilterLocal !== opt.value}
          class:hover:text-text={statusFilterLocal !== opt.value}
          onclick={() => (statusFilterLocal = opt.value)}
        >
          {opt.label}
          <span class="opacity-60 ml-0.5">{opt.count}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Level filter -->
  {#if getLevels().length > 1}
    <div class="px-3 pt-1 pb-1">
      <div class="flex flex-wrap gap-1.5">
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel === null ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => (selectedLevel = null)}
        >
          All Levels
        </button>
        {#each getLevels() as level}
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel === level ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
            onclick={() => (selectedLevel = selectedLevel === level ? null : level)}
          >
            {level}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="px-3 pt-1.5 pb-1.5">
    <input
      type="text"
      bind:value={search}
      placeholder="Search tournaments..."
      class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
    />
  </div>

  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {#if filteredEntries.length === 0}
      <p class="text-text-muted text-sm text-center py-8">No tournaments found</p>
    {:else}
      {#each filteredEntries as entry (entry.name)}
        <button
          class="w-full text-left px-3 py-2.5 rounded-lg mb-1 transition-colors"
          class:bg-primary={getSelectedName() === entry.name}
          class:hover:bg-surface-hover={getSelectedName() !== entry.name}
          class:opacity-50={entry.archived}
          onclick={() => selectedTournamentName.set(getSelectedName() === entry.name ? null : entry.name)}
        >
          <div class="flex items-center gap-2">
            <span class="font-medium text-sm truncate flex-1">{entry.name}</span>
            {#if entry.archived}
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-bg text-text-muted">Archived</span>
            {/if}
          </div>
          <div class="flex items-center gap-3 mt-1 text-xs text-text-muted">
            <span>{entry.gameCount} game{entry.gameCount !== 1 ? "s" : ""}</span>
            {#if entry.newCount > 0}
              <span>{entry.newCount} new</span>
            {/if}
            {#if entry.activeCount > 0}
              <span class="text-secondary">{entry.activeCount} active</span>
            {/if}
            {#if entry.doneCount > 0}
              <span class="text-green-400">{entry.doneCount} done</span>
            {/if}
          </div>
        </button>
      {/each}
    {/if}
  </div>
</div>
