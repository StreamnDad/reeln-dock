<script lang="ts">
  import { getContext } from "svelte";
  import {
    getTournamentGroups,
    gameStatus,
    type GameStatus,
  } from "$lib/stores/games";
  import type { AppContext } from "$lib/context";
  import { getTournamentMetadata, isArchived } from "$lib/stores/tournaments.svelte";
  import { getDockSettings, setDockSettings } from "$lib/stores/config.svelte";
  import { saveDockSettings } from "$lib/ipc/config";
  import { isDragging, getDropTarget, setDropTarget } from "$lib/stores/drag.svelte";
  import { setGameTournament } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";
  import TreeNode from "./TreeNode.svelte";
  import GameTreeItem from "./GameTreeItem.svelte";
  import NewGameModal from "$lib/components/NewGameModal.svelte";

  const app = getContext<AppContext>("app");

  let showNewGame = $state(false);
  let selectedLevel_ = $state<string | null>(null);
  let statusFilter_ = $state<GameStatus>("all");

  let tournamentMeta = $derived(getTournamentMetadata());
  let dockSettings = $derived(getDockSettings());

  let archivedNames = $derived(
    new Set(tournamentMeta.filter((m) => isArchived(m.name)).map((m) => m.name)),
  );

  // Compute levels and counts from context games (reactive via $state getter)
  let gameLevels = $derived(() => {
    const s = new Set<string>();
    for (const g of app.games) { if (g.state.game_info.level) s.add(g.state.game_info.level); }
    return Array.from(s).sort();
  });
  let statusCountsVal = $derived(() => {
    const counts: Record<GameStatus, number> = { all: app.games.length, new: 0, active: 0, done: 0 };
    for (const g of app.games) { counts[gameStatus(g)]++; }
    return counts;
  });

  let groups = $derived(getTournamentGroups(app.games, selectedLevel_, statusFilter_, archivedNames, false));
  let search = $state("");
  let dragActive = $derived(isDragging());
  let currentDropTarget = $derived(getDropTarget());

  // Per-tournament open state
  let sectionOpen = $state<Record<string, boolean>>({});

  $effect(() => {
    const defaultExpanded = dockSettings.display?.sections_expanded?.games ?? true;
    for (const g of groups) {
      if (!(g.tournament in sectionOpen)) {
        sectionOpen[g.tournament] = defaultExpanded;
      }
    }
  });

  let showNewTournament = $state(false);
  let newTournamentName = $state("");
  let newTournamentForGame = $state<string | null>(null);

  function handleContextAction(action: string, dirPath: string) {
    if (action === "new-tournament") {
      newTournamentForGame = dirPath;
      showNewTournament = true;
      newTournamentName = "";
    }
  }

  async function confirmNewTournament() {
    const name = newTournamentName.trim();
    showNewTournament = false;
    if (!name || !newTournamentForGame) return;
    try {
      log.info("DnD", `creating tournament "${name}" for ${newTournamentForGame}`);
      const newState = await setGameTournament(newTournamentForGame, name);
      app.updateGameState(newTournamentForGame, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("DnD", `Failed to create tournament: ${err}`);
    }
    newTournamentForGame = null;
    newTournamentName = "";
  }

  function handlePointerEnter(tournament: string) {
    if (!dragActive) return;
    setDropTarget(tournament);
  }

  function handlePointerLeave(tournament: string) {
    if (currentDropTarget === tournament) {
      setDropTarget(null);
    }
  }

  async function expandAll() {
    for (const g of filteredGroups) {
      sectionOpen[g.tournament] = true;
    }
    await persistExpandState(true);
  }

  async function collapseAll() {
    for (const g of filteredGroups) {
      sectionOpen[g.tournament] = false;
    }
    await persistExpandState(false);
  }

  async function persistExpandState(expanded: boolean) {
    const updated = {
      ...dockSettings,
      display: {
        ...dockSettings.display,
        sections_expanded: {
          ...dockSettings.display.sections_expanded,
          games: expanded,
        },
      },
    };
    await saveDockSettings(updated);
    setDockSettings(updated);
  }

  let filteredGroups = $derived(
    groups
      .map((g) => ({
        ...g,
        games: g.games.filter((game) => {
          if (!search) return true;
          const q = search.toLowerCase();
          const info = game.state.game_info;
          return (
            info.home_team.toLowerCase().includes(q) ||
            info.away_team.toLowerCase().includes(q) ||
            info.date.includes(q) ||
            info.tournament.toLowerCase().includes(q)
          );
        }),
      }))
      .filter((g) => g.games.length > 0),
  );

  let allExpanded = $derived(filteredGroups.every((g) => sectionOpen[g.tournament]));

  const statusOptions: { label: string; value: GameStatus }[] = [
    { label: "All", value: "all" },
    { label: "New", value: "new" },
    { label: "Active", value: "active" },
    { label: "Done", value: "done" },
  ];
</script>

<div class="flex flex-col h-full">
  {#if gameLevels().length > 0}
    <div class="px-3 pt-2 pb-1">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">Level</h4>
      <div class="flex flex-wrap gap-1.5">
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel_ === null ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => selectedLevel_ = null}
        >
          All
        </button>
        {#each gameLevels() as level}
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel_ === level ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
            onclick={() => selectedLevel_ = level}
          >
            {level}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="px-3 pt-1.5 pb-1">
    <div class="flex gap-1">
      {#each statusOptions as opt}
        {@const count = statusCountsVal()[opt.value]}
        <button
          class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors"
          class:bg-primary={statusFilter_ === opt.value}
          class:text-text={statusFilter_ === opt.value}
          class:text-text-muted={statusFilter_ !== opt.value}
          class:hover:text-text={statusFilter_ !== opt.value}
          onclick={() => statusFilter_ = opt.value}
        >
          {opt.label}
          <span class="opacity-60 ml-0.5">{count}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="px-3 pt-1.5 pb-0">
    <button
      class="w-full px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors"
      onclick={() => (showNewGame = true)}
    >
      + New Game
    </button>
  </div>

  <div class="px-3 pt-2 pb-1.5 flex items-center gap-2">
    <input
      type="text"
      bind:value={search}
      placeholder="Search games..."
      class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
    />
    <button
      class="px-1.5 py-1 text-[10px] text-text-muted hover:text-text transition-colors shrink-0"
      onclick={() => allExpanded ? collapseAll() : expandAll()}
      title={allExpanded ? "Collapse all" : "Expand all"}
    >
      {allExpanded ? "−" : "+"}
    </button>
  </div>

  <div class="flex-1 overflow-y-auto px-2 pb-2">
    {#if filteredGroups.length === 0}
      <p class="text-text-muted text-sm text-center py-8">No games found</p>
    {:else}
      {#each filteredGroups as group}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="rounded transition-colors"
          class:bg-surface-hover={currentDropTarget === group.tournament}
          class:ring-1={currentDropTarget === group.tournament}
          class:ring-secondary={currentDropTarget === group.tournament}
          data-drop-tournament={group.tournament}
          onpointerenter={() => handlePointerEnter(group.tournament)}
          onpointerleave={() => handlePointerLeave(group.tournament)}
        >
          <TreeNode label="{group.tournament} ({group.games.length})" bind:open={sectionOpen[group.tournament]}>
            {#each group.games as game}
              <GameTreeItem {game} oncontextaction={handleContextAction} />
            {/each}
          </TreeNode>
        </div>
      {/each}
    {/if}
  </div>

  {#if showNewGame}
    <NewGameModal onclose={() => (showNewGame = false)} />
  {/if}

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
</div>
