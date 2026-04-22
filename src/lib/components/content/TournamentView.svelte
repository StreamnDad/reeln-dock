<script lang="ts">
  import { getContext } from "svelte";
  import { getTournamentGroups } from "$lib/stores/games";
  import type { AppContext } from "$lib/context";
  import { getTournamentMetadata, isArchived } from "$lib/stores/tournaments.svelte";
  import { getDropTarget, setDropTarget, isDragging, getDraggingGameDir, startDrag } from "$lib/stores/drag.svelte";
  import { setGameTournament } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";
  import type { GameSummary } from "$lib/types/game";
  import TeamLogo from "$lib/components/TeamLogo.svelte";

  const app = getContext<AppContext>("app");

  let tournamentMeta = $derived(getTournamentMetadata());
  let archivedNames = $derived(
    new Set(tournamentMeta.filter((m) => isArchived(m.name)).map((m) => m.name)),
  );

  let groups = $derived(getTournamentGroups(app.games, null, "all", archivedNames, false));
  let dragActive = $derived(isDragging());
  let currentDropTarget = $derived(getDropTarget());
  let draggingDir = $derived(getDraggingGameDir());

  // Rename state
  let renamingTournament = $state<string | null>(null);
  let renameValue = $state("");

  // New tournament prompt
  let showNewTournament = $state(false);
  let newTournamentName = $state("");
  let newTournamentForGame = $state<string | null>(null);

  function handlePointerEnter(tournament: string) {
    if (!dragActive) return;
    log.debug("DnD", `pointer entered tournament: ${tournament}`);
    setDropTarget(tournament);
  }

  function handlePointerLeave(tournament: string) {
    if (currentDropTarget === tournament) {
      setDropTarget(null);
    }
  }

  function openNewTournamentDialog() {
    newTournamentForGame = null;
    showNewTournament = true;
    newTournamentName = "";
  }

  /** Called externally when a drag ends over a "new tournament" zone. */
  export function promptNewTournamentForGame(dirPath: string) {
    newTournamentForGame = dirPath;
    showNewTournament = true;
    newTournamentName = "";
  }

  async function moveTo(dirPath: string, tournament: string) {
    try {
      log.info("DnD", `moving ${dirPath} → "${tournament}"`);
      const newState = await setGameTournament(dirPath, tournament);
      log.info("DnD", `IPC success`);
      app.updateGameState(dirPath, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("DnD", `IPC failed: ${err}`);
    }
  }

  async function confirmNewTournament() {
    const name = newTournamentName.trim();
    showNewTournament = false;
    if (!name) return;
    if (newTournamentForGame) {
      await moveTo(newTournamentForGame, name);
    }
    newTournamentForGame = null;
    newTournamentName = "";
  }

  async function removeFromTournament(game: GameSummary) {
    await moveTo(game.dir_path, "");
  }

  function startRename(tournament: string) {
    renamingTournament = tournament;
    renameValue = tournament === "Ungrouped" ? "" : tournament;
  }

  async function finishRename(oldTournament: string) {
    const newName = renameValue.trim();
    renamingTournament = null;
    if (!newName || newName === oldTournament) return;

    const group = groups.find((g) => g.tournament === oldTournament);
    if (!group) return;

    for (const game of group.games) {
      await moveTo(game.dir_path, newName);
    }
  }

  function handleCardPointerDown(e: PointerEvent, game: GameSummary) {
    if (e.button !== 0) return;
    const label = `${game.state.game_info.home_team} vs ${game.state.game_info.away_team}`;
    startDrag(game.dir_path, label, e.clientX, e.clientY);
  }

  function handleCardClick(_dirPath: string) {
    if (dragActive) return;
    // TODO: wire up game selection via context
  }
</script>

<div>
  <h2 class="text-lg font-bold mb-4">All Games</h2>

  {#if groups.length === 0}
    <div class="text-text-muted text-center py-12">
      <p class="text-lg mb-2">No games yet</p>
      <p class="text-sm">Games will appear here once you set up an output directory.</p>
    </div>
  {:else}
    {#each groups as group (group.tournament)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="mb-6 p-3 rounded-lg border-2 border-dashed transition-colors"
        class:border-secondary={currentDropTarget === group.tournament}
        class:bg-surface-hover={currentDropTarget === group.tournament}
        class:border-transparent={!dragActive || currentDropTarget !== group.tournament}
        class:border-border={dragActive && currentDropTarget !== group.tournament}
        data-drop-tournament={group.tournament}
        onpointerenter={() => handlePointerEnter(group.tournament)}
        onpointerleave={() => handlePointerLeave(group.tournament)}
      >
        <div class="flex items-center gap-2 mb-3">
          {#if renamingTournament === group.tournament}
            <input
              type="text"
              bind:value={renameValue}
              class="px-2 py-0.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
              onkeydown={(e) => {
                if (e.key === "Enter") finishRename(group.tournament);
                if (e.key === "Escape") (renamingTournament = null);
              }}
              onblur={() => finishRename(group.tournament)}
              autofocus
            />
          {:else}
            <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted">
              {group.tournament}
            </h3>
            {#if group.tournament !== "Ungrouped"}
              <button
                class="text-xs text-text-muted hover:text-text transition-colors"
                onclick={() => startRename(group.tournament)}
                title="Rename tournament"
              >
                &#9998;
              </button>
            {/if}
          {/if}
          <span class="text-xs text-text-muted ml-auto">{group.games.length} game{group.games.length !== 1 ? "s" : ""}</span>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each group.games as game (game.dir_path)}
            <button
              class="p-4 bg-surface rounded-lg border border-border hover:border-secondary transition-colors select-none text-left cursor-grab active:cursor-grabbing"
              class:opacity-40={draggingDir === game.dir_path}
              data-game-dir={game.dir_path}
              onpointerdown={(e) => handleCardPointerDown(e, game)}
              onclick={() => handleCardClick(game.dir_path)}
            >
              <div class="flex items-center gap-2 font-medium">
                <TeamLogo teamName={game.state.game_info.home_team} size="sm" />
                <span>{game.state.game_info.home_team} vs {game.state.game_info.away_team}</span>
                <TeamLogo teamName={game.state.game_info.away_team} size="sm" />
              </div>
              <div class="text-sm text-text-muted mt-1">{game.state.game_info.date}</div>
              <div class="flex items-center gap-2 mt-2 text-xs">
                <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted">
                  {game.state.game_info.sport}
                </span>
                {#if game.state.finished}
                  <span class="px-2 py-0.5 rounded-full bg-green-900 text-green-300">Done</span>
                {:else if game.state.segments_processed.length > 0}
                  <span class="px-2 py-0.5 rounded-full bg-blue-900 text-secondary">In Progress</span>
                {/if}
                {#if game.state.events.length > 0}
                  <span class="text-text-muted">{game.state.events.length} events</span>
                {/if}
              </div>
              {#if group.tournament !== "Ungrouped"}
                <button
                  class="mt-2 text-xs text-text-muted hover:text-accent transition-colors"
                  onclick={(e) => { e.stopPropagation(); removeFromTournament(game); }}
                >
                  Remove from tournament
                </button>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    {/each}

    <!-- New tournament zone — drop target + clickable -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="mb-6 p-6 rounded-lg border-2 border-dashed transition-colors text-center cursor-pointer"
      class:border-secondary={currentDropTarget === "__new__"}
      class:bg-surface-hover={currentDropTarget === "__new__"}
      class:border-border={currentDropTarget !== "__new__"}
      data-drop-tournament="__new__"
      onpointerenter={() => handlePointerEnter("__new__")}
      onpointerleave={() => handlePointerLeave("__new__")}
      onclick={openNewTournamentDialog}
      role="button"
      tabindex="0"
      onkeydown={(e) => { if (e.key === "Enter") openNewTournamentDialog(); }}
    >
      <span class="text-sm text-text-muted">+ Create new tournament</span>
    </div>

    <!-- New tournament name modal -->
    {#if showNewTournament}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        onclick={(e) => { if (e.target === e.currentTarget) showNewTournament = false; }}
      >
        <div class="bg-surface rounded-xl border border-border p-6 w-80">
          <h3 class="font-bold mb-3">New Tournament</h3>
          <input
            type="text"
            bind:value={newTournamentName}
            placeholder="Tournament name"
            class="w-full px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            onkeydown={(e) => {
              if (e.key === "Enter") confirmNewTournament();
              if (e.key === "Escape") (showNewTournament = false);
            }}
            autofocus
          />
          <div class="flex justify-end gap-2 mt-4">
            <button
              class="px-4 py-1.5 text-text-muted hover:text-text text-sm transition-colors"
              onclick={() => (showNewTournament = false)}
            >
              Cancel
            </button>
            <button
              class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
              onclick={confirmNewTournament}
            >
              Create
            </button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>
