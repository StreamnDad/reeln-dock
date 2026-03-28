<script lang="ts">
  import type { TeamProfile } from "$lib/types/team";
  import { listTeamLevels, listTeamProfiles } from "$lib/ipc/teams";
  import { selectedTeamKey } from "$lib/stores/navigation";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { getDockSettings, setDockSettings } from "$lib/stores/config.svelte";
  import { saveDockSettings } from "$lib/ipc/config";
  import TeamLogo from "$lib/components/TeamLogo.svelte";
  import TreeNode from "./TreeNode.svelte";

  let levels = $state<string[]>([]);
  let teamsByLevel = $state<Record<string, TeamProfile[]>>({});
  let loading = $state(true);
  let search = $state("");
  let selectedLevel = $state<string | null>(null);

  const getSelectedTeamKey = useStore(selectedTeamKey);

  let dockSettings = $derived(getDockSettings());

  let sectionOpen = $state<Record<string, boolean>>({});

  $effect(() => {
    loadTeams();
  });

  async function loadTeams() {
    loading = true;
    try {
      levels = await listTeamLevels();
      const result: Record<string, TeamProfile[]> = {};
      const defaultExpanded = dockSettings.display?.sections_expanded?.teams ?? true;
      for (const level of levels) {
        result[level] = await listTeamProfiles(level);
        if (!(level in sectionOpen)) {
          sectionOpen[level] = defaultExpanded;
        }
      }
      teamsByLevel = result;
    } catch {
      levels = [];
      teamsByLevel = {};
    } finally {
      loading = false;
    }
  }

  function teamKeyForLevel(level: string, team: TeamProfile): string {
    return `${level}/${team.team_name}`;
  }

  function handleSelect(level: string, team: TeamProfile) {
    const key = teamKeyForLevel(level, team);
    selectedTeamKey.set(getSelectedTeamKey() === key ? null : key);
  }

  async function expandAll() {
    for (const level of displayedLevels) {
      sectionOpen[level] = true;
    }
    await persistExpandState(true);
  }

  async function collapseAll() {
    for (const level of displayedLevels) {
      sectionOpen[level] = false;
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
          teams: expanded,
        },
      },
    };
    await saveDockSettings(updated);
    setDockSettings(updated);
  }

  // Total team count per level (for filter pill badges)
  let levelCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const [level, teams] of Object.entries(teamsByLevel)) {
      counts[level] = teams.length;
    }
    return counts;
  });

  let totalTeams = $derived(
    Object.values(teamsByLevel).reduce((sum, teams) => sum + teams.length, 0),
  );

  // Apply level + search filters
  let filteredTeamsByLevel = $derived.by(() => {
    const source = selectedLevel
      ? { [selectedLevel]: teamsByLevel[selectedLevel] ?? [] }
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
    (selectedLevel ? [selectedLevel] : levels).filter((l) => l in filteredTeamsByLevel),
  );

  let allExpanded = $derived(displayedLevels.length > 0 && displayedLevels.every((l) => sectionOpen[l]));
</script>

<div class="flex flex-col h-full">
  <!-- Level filter -->
  {#if levels.length > 1}
    <div class="px-3 pt-2 pb-1">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">Level</h4>
      <div class="flex flex-wrap gap-1.5">
        <button
          class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel === null ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
          onclick={() => (selectedLevel = null)}
        >
          All <span class="opacity-60 ml-0.5">{totalTeams}</span>
        </button>
        {#each levels as level}
          <button
            class="px-2.5 py-1 rounded-md text-xs font-medium transition-colors {selectedLevel === level ? 'bg-primary text-text' : 'bg-bg text-text-muted hover:text-text hover:bg-surface-hover'}"
            onclick={() => (selectedLevel = selectedLevel === level ? null : level)}
          >
            {level} <span class="opacity-60 ml-0.5">{levelCounts[level] ?? 0}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="px-3 pt-1.5 pb-1.5 flex items-center gap-2">
    <input
      type="text"
      bind:value={search}
      placeholder="Search teams..."
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
    {#if loading}
      <p class="text-text-muted text-sm text-center py-8">Loading teams...</p>
    {:else if displayedLevels.length === 0}
      <p class="text-text-muted text-sm text-center py-8">No teams found</p>
    {:else}
      {#each displayedLevels as level}
        <TreeNode label="{level} ({filteredTeamsByLevel[level].length})" bind:open={sectionOpen[level]}>
          {#each filteredTeamsByLevel[level] as team (teamKeyForLevel(level, team))}
            {@const isSelected = getSelectedTeamKey() === teamKeyForLevel(level, team)}
            {@const primaryColor = team.colors?.[0] ?? "#555"}
            <button
              class="flex items-center gap-2.5 w-full pl-1 pr-2 py-2 rounded-lg text-sm text-left transition-colors mb-0.5 overflow-hidden"
              class:bg-primary={isSelected}
              class:hover:bg-surface-hover={!isSelected}
              onclick={() => handleSelect(level, team)}
            >
              <div class="w-1 self-stretch rounded-full shrink-0" style="background: {primaryColor}"></div>
              <TeamLogo teamName={team.team_name} size="md" />
              <div class="flex flex-col min-w-0 flex-1">
                <span class="truncate font-semibold">{team.team_name}</span>
                {#if team.short_name && team.short_name !== team.team_name}
                  <span class="text-[11px] text-text-muted truncate">{team.short_name}</span>
                {/if}
                {#if team.colors && team.colors.length > 0}
                  <div class="flex gap-1 mt-1">
                    {#each team.colors.slice(0, 4) as color}
                      <span
                        class="w-3 h-3 rounded-full border border-white/20"
                        style="background: {color}"
                      ></span>
                    {/each}
                  </div>
                {/if}
              </div>
            </button>
          {/each}
        </TreeNode>
      {/each}
    {/if}
  </div>
</div>
