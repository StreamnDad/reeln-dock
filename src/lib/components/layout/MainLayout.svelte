<script lang="ts">
  import Header from "./Header.svelte";
  import DragGhost from "$lib/components/DragGhost.svelte";
  import PluginManager from "$lib/components/plugins/PluginManager.svelte";
  import PluginRegistryView from "$lib/components/plugins/PluginRegistryView.svelte";
  import SettingsView from "$lib/components/settings/SettingsView.svelte";
  import GameView from "$lib/components/content/GameView.svelte";
  import ClipReviewPanel from "$lib/components/content/ClipReviewPanel.svelte";
  import RenderQueueView from "$lib/components/content/RenderQueueView.svelte";
  import NewGameModal from "$lib/components/NewGameModal.svelte";
  import TeamLogo from "$lib/components/TeamLogo.svelte";
  import { getConfig } from "$lib/stores/config.svelte";
  import { setGames } from "$lib/stores/games";
  import type { GameSummary } from "$lib/types/game";
  import type { TeamProfile } from "$lib/types/team";
  import { listGames, setGameTournament, getGameState, getEventTypes } from "$lib/ipc/games";
  import { listTeamLevels, listTeamProfiles } from "$lib/ipc/teams";
  import { moveDrag, endDrag, cancelDrag } from "$lib/stores/drag.svelte";
  import { initJobListener } from "$lib/stores/jobs.svelte";
  import { initQueue } from "$lib/stores/renderQueue.svelte";
  import { initPluginUI } from "$lib/stores/pluginUI.svelte";
  import { loadTournamentMetadata, isArchived } from "$lib/stores/tournaments.svelte";
  import { loadAllTeams } from "$lib/stores/teams.svelte";
  import { settingsTeamTarget, settingsTournamentTarget, editingQueueItem } from "$lib/stores/navigation";
  import { log } from "$lib/stores/log.svelte";
  import { gameStatus, getTournamentGroups, type GameStatus } from "$lib/stores/games";
  import { getTournamentMetadata } from "$lib/stores/tournaments.svelte";
  import type { View } from "$lib/stores/navigation";
  import { useStore } from "$lib/stores/bridge.svelte";

  type SidebarTab = "games" | "teams" | "tournaments";

  let view = $state<View>("games");
  let sidebarTab = $state<SidebarTab>("games");
  let gamesData = $state<GameSummary[]>([]);
  let selectedGameDir = $state<string | null>(null);
  let selectedEventId_ = $state<string | null>(null);
  let expandedClipReview = $state(false);

  // Structured event types from config
  import type { EventTypeEntry } from "$lib/types/config";
  let configuredEventTypes_ = $state<EventTypeEntry[]>([]);

  $effect(() => {
    getEventTypes()
      .then((types) => { configuredEventTypes_ = types; })
      .catch(() => { configuredEventTypes_ = []; });
  });
  let levelFilter = $state<string | null>(null);
  let statusFilter = $state<GameStatus>("all");
  let search = $state("");
  let showNewGame = $state(false);
  let showEnded = $state(false);

  // Teams data
  let teamLevels = $state<string[]>([]);
  let teamsByLevel = $state<Record<string, TeamProfile[]>>({});

  function setView(v: View) {
    view = v;
    // Only reset sidebar tab, preserve game selection so navigation persists
    if (v === "games") {
      sidebarTab = "games";
    }
  }

  /** Select a game and reload its state from disk (picks up CLI changes). */
  async function selectGame(dirPath: string | null) {
    if (dirPath === selectedGameDir) { selectedGameDir = null; selectedEventId_ = null; return; }
    selectedEventId_ = null;
    if (dirPath) {
      try {
        const freshState = await getGameState(dirPath);
        gamesData = gamesData.map(g => g.dir_path === dirPath ? { ...g, state: freshState } : g);
      } catch { /* use cached state */ }
    }
    selectedGameDir = dirPath;
  }
  function handleUpdateGame(dirPath: string, updater: (g: GameSummary) => GameSummary) {
    gamesData = gamesData.map(g => g.dir_path === dirPath ? updater(g) : g);
    setGames(gamesData);
  }

  // Event navigation for clip review
  function getEventIds(): string[] {
    const game = gamesData.find(g => g.dir_path === selectedGameDir);
    if (!game) return [];
    return game.state.events.map(e => e.id);
  }

  function advanceToNextEvent() {
    const ids = getEventIds();
    const idx = ids.indexOf(selectedEventId_ ?? "");
    if (idx >= 0 && idx < ids.length - 1) {
      selectedEventId_ = ids[idx + 1];
    }
  }

  function advanceToPrevEvent() {
    const ids = getEventIds();
    const idx = ids.indexOf(selectedEventId_ ?? "");
    if (idx > 0) {
      selectedEventId_ = ids[idx - 1];
    }
  }

  $effect(() => {
    initJobListener().catch((e) => log.error("Jobs", `Failed to init listener: ${e}`));
    initQueue().catch((e) => log.error("RenderQueue", `Failed to load queue: ${e}`));
    initPluginUI().catch((e) => log.error("PluginUI", `Failed to init: ${e}`));
  });

  // React to edit-from-queue navigation requests
  const getEditRequest = useStore(editingQueueItem);
  $effect(() => {
    const req = getEditRequest();
    if (req) {
      // Navigate to the game and select the event
      view = "games";
      sidebarTab = "games";
      selectGame(req.gameDir).then(() => {
        selectedEventId_ = req.eventId;
      });
    }
  });

  let config = $derived(getConfig());

  $effect(() => {
    if (config?.paths.output_dir) {
      listGames(config.paths.output_dir).then((data) => {
        gamesData = data;
        setGames(data);
      }).catch((e) => log.error("Games", `Failed to load: ${e}`));
      loadTournamentMetadata().catch((e) => log.error("Tournaments", `Failed to load metadata: ${e}`));
    }
  });

  $effect(() => {
    loadAllTeams().catch((e) => log.error("Teams", `Failed to load: ${e}`));
    // Load team profiles for sidebar
    listTeamLevels().then(async (lvls) => {
      teamLevels = lvls;
      const result: Record<string, TeamProfile[]> = {};
      for (const l of lvls) { result[l] = await listTeamProfiles(l); }
      teamsByLevel = result;
    }).catch((e) => log.error("Teams", `Failed to load profiles: ${e}`));
  });

  function handlePointerMove(e: PointerEvent) { moveDrag(e.clientX, e.clientY); }
  async function handlePointerUp(_e: PointerEvent) {
    const result = endDrag();
    if (!result) return;
    cancelDrag();
  }
  function handleKeyDown(e: KeyboardEvent) { if (e.key === "Escape") cancelDrag(); }
</script>

<svelte:window onpointermove={handlePointerMove} onpointerup={handlePointerUp} onkeydown={handleKeyDown} />

<div class="flex flex-col h-screen w-screen">
  <Header currentView={view} {setView} />

  <!-- Everything inline, no snippets, no child routing components -->
  <div class="flex flex-1 overflow-hidden">
    {#if view === "games"}
      {@const levels = [...new Set(gamesData.map(g => g.state.game_info.level).filter(Boolean))].sort()}
      {@const statusCounts = (() => { const c = { all: gamesData.length, new: 0, active: 0, done: 0 }; for (const g of gamesData) { const s = gameStatus(g); if (s in c) c[s]++; } return c; })()}
      {@const tournamentMeta = getTournamentMetadata()}
      {@const archivedNames = new Set(tournamentMeta.filter(m => isArchived(m.name)).map(m => m.name))}
      {@const groups = getTournamentGroups(gamesData, levelFilter, statusFilter, archivedNames, showEnded)}
      {@const filteredGroups = groups.map(g => ({ ...g, games: g.games.filter(game => { if (!search) return true; const q = search.toLowerCase(); const info = game.state.game_info; return info.home_team.toLowerCase().includes(q) || info.away_team.toLowerCase().includes(q) || info.date.includes(q) || info.tournament.toLowerCase().includes(q); })})).filter(g => g.games.length > 0)}
      {@const allTournamentNames = [...new Set(gamesData.map(g => g.state.game_info.tournament).filter(Boolean))].sort()}
      <div class="w-72 shrink-0 overflow-y-auto border-r border-border bg-surface flex flex-col">
        <!-- Sidebar tabs -->
        <div class="px-3 pt-2 pb-1 border-b border-border">
          <div class="flex gap-0.5 bg-bg rounded-lg p-0.5">
            {#each [{ label: "Games", value: "games" }, { label: "Teams", value: "teams" }, { label: "Tournaments", value: "tournaments" }] as tab}
              <button
                class="flex-1 px-2 py-1 rounded-md text-xs font-medium transition-colors text-center"
                class:bg-primary={sidebarTab === tab.value}
                class:text-text={sidebarTab === tab.value}
                class:text-text-muted={sidebarTab !== tab.value}
                onclick={() => sidebarTab = tab.value as SidebarTab}
              >{tab.label}</button>
            {/each}
          </div>
        </div>

        {#if sidebarTab === "games"}
          <!-- Level filter -->
          {#if levels.length > 0}
            <div class="px-3 pt-2 pb-1">
              <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">Level</h4>
              <div class="flex flex-wrap gap-1.5">
                <button class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {levelFilter === null ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text'}" onclick={() => levelFilter = null}>All</button>
                {#each levels as level}
                  <button class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {levelFilter === level ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text'}" onclick={() => levelFilter = level}>{level}</button>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Status filter -->
          <div class="px-3 pt-1.5 pb-1">
            <div class="flex gap-1 items-center">
              {#each ["all", "new", "active", "done"] as opt}
                <button class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors" class:bg-primary={statusFilter === opt} class:text-text={statusFilter === opt} class:text-text-muted={statusFilter !== opt} onclick={() => statusFilter = opt as GameStatus}>{opt} <span class="opacity-60">{statusCounts[opt as GameStatus]}</span></button>
              {/each}
              <button
                class="ml-auto px-1.5 py-0.5 rounded text-[10px] transition-colors {showEnded ? 'text-secondary' : 'text-text-muted hover:text-text'}"
                onclick={() => showEnded = !showEnded}
                title={showEnded ? "Hide ended tournaments" : "Show ended tournaments"}
              >{showEnded ? "ended" : "+ended"}</button>
            </div>
          </div>

          <!-- New Game -->
          <div class="px-3 pt-1.5 pb-0">
            <button class="w-full px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors" onclick={() => showNewGame = true}>+ New Game</button>
          </div>

          <!-- Search -->
          <div class="px-3 pt-1.5 pb-1.5">
            <input type="text" bind:value={search} placeholder="Search games..." class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary" />
          </div>

          <!-- Game tree -->
          <div class="flex-1 overflow-y-auto px-2 pb-2">
            {#if filteredGroups.length === 0}
              <p class="text-text-muted text-sm text-center py-8">No games found</p>
            {:else}
              {#each filteredGroups as group}
                <div class="mb-1">
                  <div class="flex items-center gap-1 w-full px-2 py-1 text-xs font-semibold uppercase tracking-wider text-text-muted">
                    {group.tournament} ({group.games.length})
                  </div>
                  <div class="ml-2">
                    {#each group.games as game (game.dir_path)}
                      {@const info = game.state.game_info}
                      {@const status = gameStatus(game)}
                      <button
                        class="flex items-center gap-2 w-full px-2 py-1.5 rounded text-sm text-left transition-colors"
                        class:bg-primary={selectedGameDir === game.dir_path}
                        class:hover:bg-surface-hover={selectedGameDir !== game.dir_path}
                        onclick={() => selectGame(game.dir_path)}
                      >
                        <span class="w-2 h-2 rounded-full shrink-0" class:bg-green-500={status === "done"} class:bg-secondary={status === "active"} class:bg-text-muted={status === "new"}></span>
                        <TeamLogo teamName={info.home_team} size="xs" />
                        <div class="flex flex-col min-w-0 flex-1">
                          <span class="truncate font-medium">{info.home_team} vs {info.away_team}</span>
                          <span class="text-xs text-text-muted truncate">{info.date}</span>
                        </div>
                        <TeamLogo teamName={info.away_team} size="xs" />
                      </button>
                    {/each}
                  </div>
                </div>
              {/each}
            {/if}
          </div>

        {:else if sidebarTab === "teams"}
          <!-- Teams list -->
          <div class="flex-1 overflow-y-auto px-2 pb-2 pt-2">
            {#if teamLevels.length === 0}
              <p class="text-text-muted text-sm text-center py-8">No teams found</p>
            {:else}
              {#each teamLevels as level}
                <div class="mb-2">
                  <div class="px-2 py-1 text-xs font-semibold uppercase tracking-wider text-text-muted">{level} ({teamsByLevel[level]?.length ?? 0})</div>
                  {#each teamsByLevel[level] ?? [] as team}
                    {@const primaryColor = team.colors?.[0] ?? "#555"}
                    <button
                      class="flex items-center gap-2.5 w-full pl-1 pr-2 py-2 rounded-lg text-sm mb-0.5 text-left transition-colors hover:bg-surface-hover"
                      onclick={() => { settingsTeamTarget.set(`${level}/${team.team_name}`); view = "settings"; }}
                      title="Edit in Settings"
                    >
                      <div class="w-1 self-stretch rounded-full shrink-0" style="background: {primaryColor}"></div>
                      <TeamLogo teamName={team.team_name} size="md" />
                      <div class="flex flex-col min-w-0 flex-1">
                        <span class="truncate font-semibold">{team.team_name}</span>
                        {#if team.short_name && team.short_name !== team.team_name}
                          <span class="text-[11px] text-text-muted truncate">{team.short_name}</span>
                        {/if}
                      </div>
                    </button>
                  {/each}
                </div>
              {/each}
            {/if}
          </div>

        {:else}
          <!-- Tournaments list -->
          <div class="flex-1 overflow-y-auto px-2 pb-2 pt-2">
            {#if allTournamentNames.length === 0}
              <p class="text-text-muted text-sm text-center py-8">No tournaments found</p>
            {:else}
              {#each allTournamentNames as name}
                {@const tGames = gamesData.filter(g => g.state.game_info.tournament === name)}
                {@const doneCount = tGames.filter(g => g.state.finished).length}
                <button
                  class="w-full text-left px-3 py-2.5 rounded-lg mb-1 transition-colors hover:bg-surface-hover"
                  onclick={() => { settingsTournamentTarget.set(name); view = "settings"; }}
                  title="Manage in Settings"
                >
                  <div class="font-medium text-sm">{name}</div>
                  <div class="text-xs text-text-muted mt-1">
                    {tGames.length} game{tGames.length !== 1 ? "s" : ""}
                    {#if doneCount > 0}
                      <span class="text-green-400 ml-2">{doneCount} done</span>
                    {/if}
                  </div>
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <div class="flex-1 overflow-y-auto p-4">
      {#if view === "plugins"}
        <PluginManager />
      {:else if view === "registry"}
        <PluginRegistryView />
      {:else if view === "queue"}
        <RenderQueueView />
      {:else if view === "settings"}
        <SettingsView />
      {:else if selectedGameDir}
        {@const game = gamesData.find(g => g.dir_path === selectedGameDir)}
        {@const selectedEvent = game?.state.events.find(e => e.id === selectedEventId_)}
        {#if game}
          {@const eventIds = game.state.events.map(e => e.id)}
          {@const currentEventIndex = eventIds.indexOf(selectedEventId_ ?? "")}
          {#if expandedClipReview && selectedEvent}
            <div class="h-full overflow-y-auto">
              <ClipReviewPanel
                event={selectedEvent}
                {game}
                eventTypes={configuredEventTypes_}
                iterationMappings={config?.iterations?.mappings ?? {}}
                expanded={true}
                eventIndex={currentEventIndex}
                eventCount={eventIds.length}
                onClose={() => { selectedEventId_ = null; expandedClipReview = false; }}
                onUpdateGame={handleUpdateGame}
                onNext={advanceToNextEvent}
                onPrev={advanceToPrevEvent}
                onToggleExpand={() => { expandedClipReview = false; }}
              />
            </div>
          {:else}
            <div class="flex h-full">
              <div class="flex-1 overflow-y-auto min-w-0">
                <GameView
                  {game}
                  activeEventId={selectedEventId_}
                  onBack={() => { selectedGameDir = null; selectedEventId_ = null; expandedClipReview = false; }}
                  onSelectEvent={(id) => { selectedEventId_ = id; }}
                  onUpdateGame={handleUpdateGame}
                />
              </div>
              {#if selectedEvent}
                <div class="w-1 shrink-0 bg-border"></div>
                <div class="shrink-0 overflow-y-auto" style="width: 480px">
                  <ClipReviewPanel
                    event={selectedEvent}
                    {game}
                    eventTypes={configuredEventTypes_}
                    iterationMappings={config?.iterations?.mappings ?? {}}
                    eventIndex={currentEventIndex}
                    eventCount={eventIds.length}
                    onClose={() => { selectedEventId_ = null; }}
                    onUpdateGame={handleUpdateGame}
                    onNext={advanceToNextEvent}
                    onPrev={advanceToPrevEvent}
                    onToggleExpand={() => { expandedClipReview = true; }}
                  />
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      {:else}
        <!-- All games grid grouped by tournament -->
        <h2 class="text-lg font-bold mb-4">All Games ({gamesData.length})</h2>
        {#if gamesData.length === 0}
          <p class="text-text-muted">Loading games...</p>
        {:else}
          {@const tournamentMeta2 = getTournamentMetadata()}
          {@const archivedNames2 = new Set(tournamentMeta2.filter(m => isArchived(m.name)).map(m => m.name))}
          {@const contentGroups = getTournamentGroups(gamesData, levelFilter, statusFilter, archivedNames2, showEnded)}
          {#each contentGroups as group}
            <div class="mb-6">
              <h3 class="text-sm font-semibold uppercase tracking-wider text-text-muted mb-3">{group.tournament} <span class="text-xs ml-1">{group.games.length} game{group.games.length !== 1 ? "s" : ""}</span></h3>
              <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                {#each group.games as game (game.dir_path)}
                  {@const info = game.state.game_info}
                  <button
                    class="p-4 bg-surface rounded-lg border border-border hover:border-secondary transition-colors text-left"
                    onclick={() => selectGame(game.dir_path)}
                  >
                    <div class="font-medium">{info.home_team} vs {info.away_team}</div>
                    <div class="text-sm text-text-muted mt-1">{info.date}</div>
                    <div class="flex items-center gap-2 mt-2 text-xs">
                      <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted">{info.sport}</span>
                      {#if game.state.finished}
                        <span class="px-2 py-0.5 rounded-full bg-green-900 text-green-300">Done</span>
                      {:else if game.state.segments_processed.length > 0}
                        <span class="px-2 py-0.5 rounded-full bg-blue-900 text-secondary">In Progress</span>
                      {/if}
                      {#if game.state.events.length > 0}
                        <span class="text-text-muted">{game.state.events.length} events</span>
                      {/if}
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          {/each}
        {/if}
      {/if}
    </div>
  </div>
  <DragGhost />

  {#if showNewGame}
    <NewGameModal
      onclose={() => showNewGame = false}
      allGames={gamesData}
      onGameCreated={(game) => { gamesData = [...gamesData, game]; setGames(gamesData); }}
      onSelectGame={(dir) => { selectGame(dir); showNewGame = false; }}
    />
  {/if}
</div>
