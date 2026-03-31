<script lang="ts">
  import { games, gameStatus, getGamesForTournament, updateGameState, setSelectedGameDir } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { isArchived, getTournamentMetadata } from "$lib/stores/tournaments.svelte";
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
  let startDate = $state("");
  let endDate = $state("");
  let url = $state("");

  $effect(() => {
    notes = meta?.notes ?? "";
    startDate = meta?.start_date ?? "";
    endDate = meta?.end_date ?? "";
    url = meta?.url ?? "";
    saveMessage = "";
    renaming = false;
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

  let saving = $state(false);
  let saveMessage = $state("");

  async function saveMeta() {
    saving = true;
    saveMessage = "";
    try {
      await updateTournamentMetadata({
        name: tournamentName,
        archived: false,
        notes: notes.trim(),
        start_date: startDate,
        end_date: endDate,
        url: url.trim(),
      });
      saveMessage = "Saved.";
    } catch (err) {
      saveMessage = `Error: ${err}`;
      log.error("Tournament", `Failed to save metadata: ${err}`);
    } finally {
      saving = false;
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

    {#if archived}
      <span class="ml-auto text-xs px-2 py-1 rounded-full bg-bg text-text-muted border border-border">Ended</span>
    {/if}
  </div>

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

  <!-- Dates, URL & Notes -->
  <div class="bg-surface rounded-lg border border-border p-4 mb-6 space-y-4">
    <div class="grid grid-cols-2 gap-4">
      <div>
        <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="start-date">
          Start Date
        </label>
        <input
          id="start-date"
          type="date"
          bind:value={startDate}
          class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
        />
      </div>
      <div>
        <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="end-date">
          End Date
        </label>
        <input
          id="end-date"
          type="date"
          bind:value={endDate}
          class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
        />
      </div>
    </div>
    <div>
      <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="url">
        URL
      </label>
      <input
        id="url"
        type="url"
        bind:value={url}
        placeholder="https://..."
        class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text font-mono placeholder:text-text-muted focus:outline-none focus:border-secondary"
      />
    </div>
    <div>
      <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="notes">
        Notes
      </label>
      <textarea
        id="notes"
        bind:value={notes}
        placeholder="Tournament notes..."
        rows="2"
        class="w-full px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary resize-y"
      ></textarea>
    </div>
    <div class="flex items-center gap-3 pt-1">
      <button
        class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
        onclick={saveMeta}
        disabled={saving}
      >{saving ? "Saving..." : "Save"}</button>
      {#if saveMessage}
        <span class="text-sm text-text-muted">{saveMessage}</span>
      {/if}
    </div>
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
