<script lang="ts">
  import { games, gameStatus, getGamesForTournament, updateGameState, setSelectedGameDir } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { isArchived, toggleArchived, getTournamentMetadata } from "$lib/stores/tournaments.svelte";
  import { selectedTournamentName, setSidebarMode } from "$lib/stores/navigation";
  import { setGameTournament } from "$lib/ipc/games";
  import { updateTournamentMetadata } from "$lib/ipc/tournaments";
  import { log } from "$lib/stores/log.svelte";

  interface Props {
    tournamentName: string;
  }

  let { tournamentName }: Props = $props();

  const getGames = useStore(games);

  let tourneyGames = $derived(getGamesForTournament(getGames(), tournamentName));
  let archived = $derived(isArchived(tournamentName));
  let meta = $derived(getTournamentMetadata().find((m) => m.name === tournamentName));

  // Rename
  let renaming = $state(false);
  let renameValue = $state("");
  let notes = $state("");

  $effect(() => {
    notes = meta?.notes ?? "";
  });

  function startRename() {
    renameValue = tournamentName;
    renaming = true;
  }

  async function finishRename() {
    renaming = false;
    const newName = renameValue.trim();
    if (!newName || newName === tournamentName) return;

    for (const game of tourneyGames) {
      try {
        const newState = await setGameTournament(game.dir_path, newName);
        updateGameState(game.dir_path, (g) => ({ ...g, state: newState }));
      } catch (err) {
        log.error("Tournament", `Failed to rename: ${err}`);
      }
    }
    selectedTournamentName.set(newName);
  }

  async function handleToggleArchive() {
    await toggleArchived(tournamentName);
  }

  async function saveNotes() {
    try {
      await updateTournamentMetadata({
        name: tournamentName,
        archived,
        notes: notes.trim(),
      });
    } catch (err) {
      log.error("Tournament", `Failed to save notes: ${err}`);
    }
  }

  function navigateToGame(dirPath: string) {
    setSidebarMode("games");
    setSelectedGameDir(dirPath);
  }

  // Stats
  let newCount = $derived(tourneyGames.filter((g) => gameStatus(g) === "new").length);
  let activeCount = $derived(tourneyGames.filter((g) => gameStatus(g) === "active").length);
  let doneCount = $derived(tourneyGames.filter((g) => gameStatus(g) === "done").length);
</script>

<div>
  <!-- Header -->
  <div class="flex items-center gap-3 mb-6">
    {#if renaming}
      <input
        type="text"
        bind:value={renameValue}
        class="text-lg font-bold px-2 py-0.5 bg-bg border border-border rounded focus:outline-none focus:border-secondary"
        onkeydown={(e) => {
          if (e.key === "Enter") finishRename();
          if (e.key === "Escape") (renaming = false);
        }}
        onblur={() => finishRename()}
        autofocus
      />
    {:else}
      <h2 class="text-lg font-bold">{tournamentName}</h2>
      <button
        class="text-xs text-text-muted hover:text-text transition-colors"
        onclick={startRename}
        title="Rename"
      >&#9998;</button>
    {/if}

    <div class="ml-auto flex items-center gap-2">
      <button
        class="px-3 py-1.5 text-xs border rounded-lg transition-colors {archived
          ? 'border-secondary text-secondary hover:bg-secondary/10'
          : 'border-border text-text-muted hover:text-text hover:border-secondary'}"
        onclick={handleToggleArchive}
      >
        {archived ? "Unarchive" : "Archive"}
      </button>
    </div>
  </div>

  {#if archived}
    <div class="bg-bg border border-border rounded-lg p-3 mb-4 text-sm text-text-muted">
      This tournament is archived and hidden from the Games sidebar by default.
    </div>
  {/if}

  <!-- Stats row -->
  <div class="grid grid-cols-4 gap-3 mb-6">
    <div class="bg-surface rounded-lg border border-border p-3 text-center">
      <div class="text-2xl font-bold">{tourneyGames.length}</div>
      <div class="text-xs text-text-muted">Games</div>
    </div>
    <div class="bg-surface rounded-lg border border-border p-3 text-center">
      <div class="text-2xl font-bold text-text-muted">{newCount}</div>
      <div class="text-xs text-text-muted">New</div>
    </div>
    <div class="bg-surface rounded-lg border border-border p-3 text-center">
      <div class="text-2xl font-bold text-secondary">{activeCount}</div>
      <div class="text-xs text-text-muted">Active</div>
    </div>
    <div class="bg-surface rounded-lg border border-border p-3 text-center">
      <div class="text-2xl font-bold text-green-400">{doneCount}</div>
      <div class="text-xs text-text-muted">Done</div>
    </div>
  </div>

  <!-- Notes -->
  <div class="mb-6">
    <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="notes">
      Notes
    </label>
    <textarea
      id="notes"
      bind:value={notes}
      onblur={saveNotes}
      placeholder="Tournament notes..."
      rows="2"
      class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary resize-y"
    ></textarea>
  </div>

  <!-- Games list -->
  <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">
    Games ({tourneyGames.length})
  </h3>
  {#if tourneyGames.length === 0}
    <p class="text-text-muted text-sm py-4">No games in this tournament.</p>
  {:else}
    <div class="space-y-2">
      {#each tourneyGames as game (game.dir_path)}
        {@const info = game.state.game_info}
        {@const status = gameStatus(game)}
        <button
          class="w-full text-left p-3 bg-surface rounded-lg border border-border hover:border-secondary transition-colors"
          onclick={() => navigateToGame(game.dir_path)}
        >
          <div class="flex items-center gap-3">
            <span
              class="w-2 h-2 rounded-full shrink-0"
              class:bg-green-500={status === "done"}
              class:bg-secondary={status === "active"}
              class:bg-text-muted={status === "new"}
            ></span>
            <div class="flex-1 min-w-0">
              <div class="font-medium text-sm">{info.home_team} vs {info.away_team}</div>
              <div class="text-xs text-text-muted mt-0.5">
                {info.date} &middot; {info.sport}
                {#if game.state.events.length > 0}
                  &middot; {game.state.events.length} events
                {/if}
              </div>
            </div>
            {#if status === "done"}
              <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-green-900 text-green-300">Done</span>
            {:else if status === "active"}
              <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-blue-900 text-secondary">Active</span>
            {/if}
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>
