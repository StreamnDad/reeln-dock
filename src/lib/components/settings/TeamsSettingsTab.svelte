<script lang="ts">
  import type { TeamProfile } from "$lib/types/team";
  import {
    listTeamLevels,
    listTeamProfiles,
    saveTeamProfile,
    renameTeamLevel,
    deleteTeamLevel,
  } from "$lib/ipc/teams";
  import { updateTeamInMap, loadAllTeams } from "$lib/stores/teams.svelte";
  import { settingsTeamTarget } from "$lib/stores/navigation";
  import { useStore } from "$lib/stores/bridge.svelte";
  import TeamDetailView from "$lib/components/content/TeamDetailView.svelte";
  import TeamLogo from "$lib/components/TeamLogo.svelte";

  let levels = $state<string[]>([]);
  let teamsByLevel = $state<Record<string, TeamProfile[]>>({});
  let loading = $state(true);
  let search = $state("");
  let selectedKey = $state<string | null>(null);
  let creatingNew = $state(false);
  let levelFilter = $state<string | null>(null);

  // New team form state
  let newTeamName = $state("");
  let newShortName = $state("");
  let newLevel = $state("");
  let newTeamSaving = $state(false);
  let newTeamMessage = $state("");

  // Level management state
  let renamingLevel = $state<string | null>(null);
  let renameLevelValue = $state("");
  let levelMessage = $state("");

  const getTarget = useStore(settingsTeamTarget);

  $effect(() => {
    loadTeams();
  });

  // Check for navigation target from sidebar
  $effect(() => {
    const target = getTarget();
    if (target) {
      selectedKey = target;
      creatingNew = false;
      settingsTeamTarget.set(null);
    }
  });

  async function loadTeams() {
    loading = true;
    try {
      levels = await listTeamLevels();
      const result: Record<string, TeamProfile[]> = {};
      for (const level of levels) {
        result[level] = await listTeamProfiles(level);
      }
      teamsByLevel = result;
      if (!newLevel && levels.length > 0) {
        newLevel = levels[0];
      }
      // Reset filter if filtered level no longer exists
      if (levelFilter && !levels.includes(levelFilter)) {
        levelFilter = null;
      }
    } catch {
      levels = [];
      teamsByLevel = {};
    } finally {
      loading = false;
    }
  }

  function teamKey(level: string, team: TeamProfile): string {
    return `${level}/${team.team_name}`;
  }

  function selectTeam(key: string) {
    selectedKey = selectedKey === key ? null : key;
    creatingNew = false;
  }

  function startCreateNew() {
    creatingNew = true;
    selectedKey = null;
    newTeamName = "";
    newShortName = "";
    newLevel = levelFilter ?? (levels.length > 0 ? levels[0] : "");
    newTeamMessage = "";
  }

  async function createTeam() {
    if (!newTeamName.trim() || !newLevel.trim()) return;
    newTeamSaving = true;
    newTeamMessage = "";
    try {
      const profile: TeamProfile = {
        team_name: newTeamName.trim(),
        short_name: newShortName.trim() || newTeamName.trim(),
        level: newLevel.trim(),
        logo_path: "",
        roster_path: "",
        colors: [],
        jersey_colors: [],
        metadata: {},
      };
      const saved = await saveTeamProfile(profile);
      updateTeamInMap(saved);
      await loadAllTeams();
      await loadTeams();
      selectedKey = `${saved.level}/${saved.team_name}`;
      creatingNew = false;
    } catch (e) {
      newTeamMessage = `Error: ${e}`;
    } finally {
      newTeamSaving = false;
    }
  }

  function handleTeamDeleted() {
    selectedKey = null;
    loadTeams();
  }

  function handleTeamCloned(newKey: string) {
    loadTeams();
    selectedKey = newKey;
  }

  // Level management
  function startRenameLevel(level: string) {
    renamingLevel = level;
    renameLevelValue = level;
    levelMessage = "";
  }

  async function finishRenameLevel() {
    if (!renamingLevel) return;
    const newName = renameLevelValue.trim();
    const oldName = renamingLevel;
    renamingLevel = null;
    if (!newName || newName === oldName) return;
    try {
      await renameTeamLevel(oldName, newName);
      await loadAllTeams();
      await loadTeams();
      if (levelFilter === oldName) levelFilter = newName;
      // Update selectedKey if it was in the renamed level
      if (selectedKey?.startsWith(`${oldName}/`)) {
        selectedKey = selectedKey.replace(`${oldName}/`, `${newName}/`);
      }
      levelMessage = "";
    } catch (e) {
      levelMessage = `Rename error: ${e}`;
    }
  }

  async function handleDeleteLevel(level: string) {
    levelMessage = "";
    try {
      await deleteTeamLevel(level);
      await loadAllTeams();
      await loadTeams();
      if (levelFilter === level) levelFilter = null;
    } catch (e) {
      levelMessage = `${e}`;
    }
  }

  // Filtering
  let filteredTeamsByLevel = $derived.by(() => {
    const source = levelFilter
      ? { [levelFilter]: teamsByLevel[levelFilter] ?? [] }
      : teamsByLevel;

    if (!search) return source;
    const q = search.toLowerCase();
    const result: Record<string, TeamProfile[]> = {};
    for (const [level, teams] of Object.entries(source)) {
      const filtered = teams.filter(
        (t) =>
          t.team_name.toLowerCase().includes(q) ||
          t.short_name.toLowerCase().includes(q),
      );
      if (filtered.length > 0) result[level] = filtered;
    }
    return result;
  });

  let displayedLevels = $derived(
    (levelFilter ? [levelFilter] : levels).filter((l) => l in filteredTeamsByLevel),
  );

  let totalTeams = $derived(
    Object.values(teamsByLevel).reduce((sum, teams) => sum + teams.length, 0),
  );

  let levelCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const [level, teams] of Object.entries(teamsByLevel)) {
      counts[level] = teams.length;
    }
    return counts;
  });
</script>

<div class="flex gap-0 h-[calc(100vh-200px)]">
  <!-- Left panel: Team list -->
  <div class="w-64 shrink-0 border-r border-border flex flex-col overflow-hidden">
    <div class="px-3 pt-3 pb-2 space-y-2 border-b border-border">
      <button
        class="w-full px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors"
        onclick={startCreateNew}
      >+ New Team</button>
      <input
        type="text"
        bind:value={search}
        placeholder="Search teams..."
        class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
      />
    </div>

    <!-- Level filter -->
    {#if levels.length > 1}
      <div class="px-3 pt-2 pb-1 border-b border-border">
        <div class="flex flex-wrap gap-1.5">
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {levelFilter === null ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
            onclick={() => (levelFilter = null)}
          >
            All <span class="opacity-60 ml-0.5">{totalTeams}</span>
          </button>
          {#each levels as level}
            <button
              class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {levelFilter === level ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
              onclick={() => (levelFilter = levelFilter === level ? null : level)}
            >
              {level} <span class="opacity-60 ml-0.5">{levelCounts[level] ?? 0}</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if levelMessage}
      <div class="px-3 py-1.5 text-xs text-accent">{levelMessage}</div>
    {/if}

    <div class="flex-1 overflow-y-auto px-2 py-2">
      {#if loading}
        <p class="text-text-muted text-sm text-center py-4">Loading...</p>
      {:else if totalTeams === 0}
        <p class="text-text-muted text-sm text-center py-4">No teams yet. Create one above.</p>
      {:else if displayedLevels.length === 0}
        <p class="text-text-muted text-sm text-center py-4">No matches</p>
      {:else}
        {#each displayedLevels as level}
          <div class="mb-2">
            <div class="flex items-center gap-1 px-2 py-1 group">
              {#if renamingLevel === level}
                <input
                  type="text"
                  bind:value={renameLevelValue}
                  class="flex-1 px-1.5 py-0.5 bg-bg border border-border rounded text-xs font-semibold uppercase tracking-wider text-text focus:outline-none focus:border-secondary"
                  onkeydown={(e) => {
                    if (e.key === "Enter") finishRenameLevel();
                    if (e.key === "Escape") (renamingLevel = null);
                  }}
                  onblur={() => finishRenameLevel()}
                />
              {:else}
                <span class="flex-1 text-xs font-semibold uppercase tracking-wider text-text-muted">
                  {level} ({filteredTeamsByLevel[level].length})
                </span>
                <button
                  class="text-[10px] text-text-muted hover:text-secondary transition-colors opacity-0 group-hover:opacity-100"
                  onclick={() => startRenameLevel(level)}
                  title="Rename level"
                >&#9998;</button>
                {#if (levelCounts[level] ?? 0) === 0}
                  <button
                    class="text-[10px] text-text-muted hover:text-accent transition-colors opacity-0 group-hover:opacity-100"
                    onclick={() => handleDeleteLevel(level)}
                    title="Delete empty level"
                  >&times;</button>
                {/if}
              {/if}
            </div>
            {#each filteredTeamsByLevel[level] as team (teamKey(level, team))}
              {@const key = teamKey(level, team)}
              {@const isSelected = selectedKey === key}
              {@const primaryColor = team.colors?.[0] ?? "#555"}
              <button
                class="flex items-center gap-2 w-full px-2 py-1.5 rounded-lg text-sm text-left transition-colors mb-0.5"
                class:bg-primary={isSelected}
                class:hover:bg-surface-hover={!isSelected}
                onclick={() => selectTeam(key)}
              >
                <div class="w-1 self-stretch rounded-full shrink-0" style="background: {primaryColor}"></div>
                <TeamLogo teamName={team.team_name} size="sm" />
                <div class="flex flex-col min-w-0 flex-1">
                  <span class="truncate font-medium">{team.team_name}</span>
                  {#if team.short_name && team.short_name !== team.team_name}
                    <span class="text-[10px] text-text-muted truncate">{team.short_name}</span>
                  {/if}
                </div>
              </button>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Right panel: Detail or Create -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if creatingNew}
      <h3 class="text-lg font-bold mb-4">New Team</h3>
      <div class="bg-surface rounded-lg border border-border p-4 space-y-4 max-w-lg">
        <div>
          <label class="block text-sm text-text-muted mb-1" for="new-team-name">Team Name</label>
          <input
            id="new-team-name"
            type="text"
            bind:value={newTeamName}
            placeholder="e.g. Wildcats"
            class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
          />
        </div>
        <div>
          <label class="block text-sm text-text-muted mb-1" for="new-short-name">Short Name</label>
          <input
            id="new-short-name"
            type="text"
            bind:value={newShortName}
            placeholder="e.g. WC"
            class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
          />
        </div>
        <div>
          <label class="block text-sm text-text-muted mb-1" for="new-level">Level</label>
          <div class="flex gap-2">
            <input
              id="new-level"
              type="text"
              bind:value={newLevel}
              placeholder="e.g. 16U"
              list="level-options"
              class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            />
            <datalist id="level-options">
              {#each levels as lvl}
                <option value={lvl}></option>
              {/each}
            </datalist>
          </div>
        </div>
        {#if newTeamMessage}
          <p class="text-sm text-accent">{newTeamMessage}</p>
        {/if}
        <div class="flex gap-2">
          <button
            class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
            onclick={createTeam}
            disabled={newTeamSaving || !newTeamName.trim() || !newLevel.trim()}
          >{newTeamSaving ? "Creating..." : "Create Team"}</button>
          <button
            class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
            onclick={() => creatingNew = false}
          >Cancel</button>
        </div>
      </div>
    {:else if selectedKey}
      <TeamDetailView
        teamKey={selectedKey}
        onDelete={handleTeamDeleted}
        onClone={handleTeamCloned}
      />
    {:else}
      <div class="flex items-center justify-center h-full text-text-muted text-sm">
        <p>Select a team to edit, or create a new one.</p>
      </div>
    {/if}
  </div>
</div>
