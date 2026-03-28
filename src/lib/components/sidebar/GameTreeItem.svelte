<script lang="ts">
  import type { GameSummary } from "$lib/types/game";
  import { games, selectedGameDir, setSelectedGameDir, getTournamentGroups, updateGameState, selectedLevel, statusFilter } from "$lib/stores/games";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { startDrag, isDragging } from "$lib/stores/drag.svelte";
  import { setGameTournament } from "$lib/ipc/games";
  import { log } from "$lib/stores/log.svelte";
  import TeamLogo from "$lib/components/TeamLogo.svelte";

  interface Props {
    game: GameSummary;
    oncontextaction?: (action: string, dirPath: string) => void;
  }

  let { game, oncontextaction }: Props = $props();

  const getSelectedDir = useStore(selectedGameDir);
  const getGames = useStore(games);
  const getLevel = useStore(selectedLevel);
  const getStatus = useStore(statusFilter);

  let selected = $derived(getSelectedDir() === game.dir_path);

  const info = $derived(game.state.game_info);
  const status = $derived(
    game.state.finished ? "done" : game.state.segments_processed.length > 0 ? "active" : "new",
  );

  let showContextMenu = $state(false);
  let contextMenuPos = $state({ x: 0, y: 0 });

  function handlePointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const label = `${info.home_team} vs ${info.away_team}`;
    log.debug("DnD", `pointerdown on: ${label}`);
    startDrag(game.dir_path, label, e.clientX, e.clientY);
  }

  function handleClick() {
    if (isDragging()) return;
    setSelectedGameDir(selected ? null : game.dir_path);
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    contextMenuPos = { x: e.clientX, y: e.clientY };
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
  }

  let groups = $derived(getTournamentGroups(getGames(), getLevel(), getStatus()));

  async function moveToTournament(tournament: string) {
    showContextMenu = false;
    const targetName = tournament === "Ungrouped" ? "" : tournament;
    try {
      log.info("DnD", `context menu move: ${game.dir_path} → "${targetName}"`);
      const newState = await setGameTournament(game.dir_path, targetName);
      updateGameState(game.dir_path, (g) => ({ ...g, state: newState }));
    } catch (err) {
      log.error("DnD", `Failed to update tournament: ${err}`);
    }
  }

  function requestNewTournament() {
    showContextMenu = false;
    oncontextaction?.("new-tournament", game.dir_path);
  }
</script>

<div class="relative">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="flex items-center gap-2 w-full px-2 py-1.5 rounded text-sm text-left transition-colors cursor-grab active:cursor-grabbing"
    class:bg-primary={selected}
    class:hover:bg-surface-hover={!selected}
    onpointerdown={handlePointerDown}
    onclick={handleClick}
    oncontextmenu={handleContextMenu}
  >
    <span
      class="w-2 h-2 rounded-full shrink-0"
      class:bg-green-500={status === "done"}
      class:bg-secondary={status === "active"}
      class:bg-text-muted={status === "new"}
    ></span>
    <TeamLogo teamName={info.home_team} size="xs" />
    <div class="flex flex-col min-w-0 flex-1">
      <span class="truncate font-medium">
        {info.home_team} vs {info.away_team}
      </span>
      <span class="text-xs text-text-muted truncate">{info.date}</span>
    </div>
    <TeamLogo teamName={info.away_team} size="xs" />
  </div>

  {#if showContextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-40"
      onclick={closeContextMenu}
      oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    ></div>
    <div
      class="fixed z-50 bg-surface border border-border rounded-lg shadow-lg py-1 min-w-48"
      style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
    >
      <div class="px-3 py-1.5 text-xs text-text-muted uppercase tracking-wider">Move to Tournament</div>
      {#each groups as group (group.tournament)}
        {#if group.tournament !== (game.state.game_info.tournament || "Ungrouped")}
          <button
            class="w-full text-left px-3 py-1.5 text-sm text-text hover:bg-surface-hover transition-colors"
            onclick={() => moveToTournament(group.tournament)}
          >
            {group.tournament}
          </button>
        {/if}
      {/each}
      <button
        class="w-full text-left px-3 py-1.5 text-sm text-secondary hover:bg-surface-hover transition-colors"
        onclick={requestNewTournament}
      >
        + New Tournament...
      </button>
      {#if game.state.game_info.tournament}
        <div class="border-t border-border my-1"></div>
        <button
          class="w-full text-left px-3 py-1.5 text-sm text-accent hover:bg-surface-hover transition-colors"
          onclick={() => moveToTournament("Ungrouped")}
        >
          Remove from Tournament
        </button>
      {/if}
    </div>
  {/if}
</div>
