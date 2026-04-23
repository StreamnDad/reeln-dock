<script lang="ts">
  import { get } from "svelte/store";
  import { games, getTournamentGroups } from "$lib/stores/games";
  import type { GameSummary, GameEvent } from "$lib/types/game";

  interface Props {
    onselect: (clipPath: string) => void;
    onclose: () => void;
  }

  let { onselect, onclose }: Props = $props();

  // Snapshot games once on mount — no reactive $effect needed
  const allGames: GameSummary[] = get(games);
  const tournamentGroups = getTournamentGroups(allGames, null, "all");

  let selectedTournament = $state("");
  let selectedGameDir = $state("");
  let eventTypeFilter = $state("");

  // Unique event types across all games
  const allEventTypes: string[] = (() => {
    const types = new Set<string>();
    for (const g of allGames) {
      for (const e of g.state.events) {
        if (e.event_type) types.add(e.event_type);
      }
    }
    return Array.from(types).sort();
  })();

  // Games for selected tournament
  let filteredGames = $derived(
    selectedTournament
      ? allGames.filter(
          (g) => (g.state.game_info.tournament || "Ungrouped") === selectedTournament,
        )
      : allGames,
  );

  // Events matching filters
  let filteredEvents = $derived.by(() => {
    const source = selectedGameDir
      ? filteredGames.filter((g) => g.dir_path === selectedGameDir)
      : filteredGames;
    const events: { game: GameSummary; event: GameEvent }[] = [];
    for (const g of source) {
      for (const e of g.state.events) {
        if (eventTypeFilter && e.event_type !== eventTypeFilter) continue;
        events.push({ game: g, event: e });
      }
    }
    return events;
  });

  function resolveClipPath(game: GameSummary, clip: string): string {
    if (clip.startsWith("/")) return clip;
    return `${game.dir_path}/${clip}`;
  }

  function gameName(game: GameSummary): string {
    const info = game.state.game_info;
    return `${info.home_team} vs ${info.away_team}`;
  }

  function gameDate(game: GameSummary): string {
    return game.state.game_info.date;
  }

  function clipFileName(clip: string): string {
    const parts = clip.split("/");
    return parts[parts.length - 1] ?? clip;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
  onmousedown={(e) => { if (e.target === e.currentTarget) onclose(); }}
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="bg-surface border border-border rounded-lg shadow-xl w-[600px] max-h-[500px] flex flex-col"
    onmousedown={(e) => e.stopPropagation()}
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-border">
      <h3 class="text-sm font-semibold">Browse Clips</h3>
      <button
        class="text-text-muted hover:text-text text-lg leading-none px-1 transition-colors"
        onclick={() => onclose()}
      >&times;</button>
    </div>

    <!-- Filters -->
    <div class="flex gap-2 px-4 py-2 border-b border-border">
      <select
        bind:value={selectedTournament}
        onchange={() => { selectedGameDir = ""; }}
        class="flex-1 px-2 py-1.5 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
      >
        <option value="">All Tournaments</option>
        {#each tournamentGroups as group}
          <option value={group.tournament}>{group.tournament} ({group.games.length})</option>
        {/each}
      </select>

      <select
        bind:value={eventTypeFilter}
        class="flex-1 px-2 py-1.5 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
      >
        <option value="">All Event Types</option>
        {#each allEventTypes as et}
          <option value={et}>{et}</option>
        {/each}
      </select>
    </div>

    <!-- Game filter (shows when tournament selected) -->
    {#if selectedTournament && filteredGames.length > 0}
      <div class="px-4 py-2 border-b border-border">
        <select
          bind:value={selectedGameDir}
          class="w-full px-2 py-1.5 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
        >
          <option value="">All Games in {selectedTournament}</option>
          {#each filteredGames as game}
            <option value={game.dir_path}>
              {gameName(game)} — {gameDate(game)} ({game.state.events.length} events)
            </option>
          {/each}
        </select>
      </div>
    {/if}

    <!-- Clip list -->
    <div class="flex-1 overflow-y-auto px-2 py-2">
      {#if filteredEvents.length === 0}
        <p class="text-xs text-text-muted text-center py-8">No clips found. Try adjusting the filters.</p>
      {:else}
        <div class="space-y-0.5">
          {#each filteredEvents as { game, event }}
            <button
              class="w-full flex items-center gap-3 px-3 py-2 rounded text-left hover:bg-bg transition-colors"
              onclick={() => onselect(resolveClipPath(game, event.clip))}
            >
              <div class="flex-1 min-w-0">
                <div class="text-xs text-text truncate">{clipFileName(event.clip)}</div>
                <div class="text-[10px] text-text-muted truncate">
                  {gameName(game)} — {gameDate(game)}
                </div>
              </div>
              <div class="flex-shrink-0 flex items-center gap-1.5">
                {#if event.event_type}
                  <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{event.event_type}</span>
                {/if}
                {#if event.player}
                  <span class="text-[10px] text-text-muted">{event.player}</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="px-4 py-2 border-t border-border text-[10px] text-text-muted">
      {filteredEvents.length} clip{filteredEvents.length !== 1 ? "s" : ""} available
    </div>
  </div>
</div>
