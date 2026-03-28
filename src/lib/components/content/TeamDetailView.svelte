<script lang="ts">
  import type { TeamProfile } from "$lib/types/team";
  import { listTeamProfiles, saveTeamProfile } from "$lib/ipc/teams";
  import { updateTeamInMap } from "$lib/stores/teams.svelte";
  import TeamLogo from "$lib/components/TeamLogo.svelte";

  interface Props {
    teamKey: string;
  }

  let { teamKey }: Props = $props();

  let team = $state<TeamProfile | null>(null);
  let level = $state("");
  let loading = $state(true);
  let saving = $state(false);
  let message = $state("");

  // Editable fields
  let teamName = $state("");
  let shortName = $state("");
  let logoPath = $state("");
  let rosterPath = $state("");
  let colors = $state<string[]>([]);
  let jerseyColors = $state<string[]>([]);

  $effect(() => {
    loadTeam(teamKey);
  });

  async function loadTeam(key: string) {
    loading = true;
    message = "";
    const parts = key.split("/");
    level = parts[0];
    const name = key.slice(level.length + 1);
    try {
      const profiles = await listTeamProfiles(level);
      const found = profiles.find((p) => p.team_name === name);
      if (found) {
        team = found;
        teamName = found.team_name;
        shortName = found.short_name;
        logoPath = found.logo_path;
        rosterPath = found.roster_path;
        colors = [...found.colors];
        jerseyColors = [...found.jersey_colors];
      } else {
        team = null;
      }
    } catch {
      team = null;
    } finally {
      loading = false;
    }
  }

  async function handleSave() {
    if (!team) return;
    saving = true;
    message = "";
    try {
      const updated: TeamProfile = {
        ...team,
        level,
        team_name: teamName.trim(),
        short_name: shortName.trim(),
        logo_path: logoPath.trim(),
        roster_path: rosterPath.trim(),
        colors: colors.filter((c) => c.trim()),
        jersey_colors: jerseyColors.filter((c) => c.trim()),
      };
      const saved = await saveTeamProfile(updated);
      team = saved;
      // Update the global team map so logos refresh everywhere
      updateTeamInMap(saved);
      message = "Saved.";
    } catch (e) {
      message = `Error: ${e}`;
    } finally {
      saving = false;
    }
  }

  function addColor() {
    colors = [...colors, "#000000"];
  }

  function removeColor(index: number) {
    colors = colors.filter((_, i) => i !== index);
  }

  function updateColor(index: number, value: string) {
    colors = colors.map((c, i) => (i === index ? value : c));
  }

  function addJerseyColor() {
    jerseyColors = [...jerseyColors, "#000000"];
  }

  function removeJerseyColor(index: number) {
    jerseyColors = jerseyColors.filter((_, i) => i !== index);
  }

  function updateJerseyColor(index: number, value: string) {
    jerseyColors = jerseyColors.map((c, i) => (i === index ? value : c));
  }
</script>

<div>
  {#if loading}
    <p class="text-text-muted text-sm">Loading team...</p>
  {:else if !team}
    <p class="text-text-muted text-sm">Team not found.</p>
  {:else}
    <div class="flex items-center gap-4 mb-6">
      <TeamLogo teamName={team.team_name} size="lg" />
      <div>
        <h2 class="text-lg font-bold">{teamName}</h2>
        <p class="text-sm text-text-muted">{level}</p>
      </div>
    </div>

    {#if message}
      <p class="text-sm text-text-muted mb-4">{message}</p>
    {/if}

    <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm text-text-muted mb-1" for="team-name">Team Name</label>
          <input
            id="team-name"
            type="text"
            bind:value={teamName}
            class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
          />
        </div>
        <div>
          <label class="block text-sm text-text-muted mb-1" for="short-name">Short Name</label>
          <input
            id="short-name"
            type="text"
            bind:value={shortName}
            class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
          />
        </div>
      </div>

      <div>
        <label class="block text-sm text-text-muted mb-1" for="logo-path">Logo Path</label>
        <input
          id="logo-path"
          type="text"
          bind:value={logoPath}
          class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text font-mono focus:outline-none focus:border-secondary"
        />
      </div>

      <div>
        <label class="block text-sm text-text-muted mb-1" for="roster-path">Roster Path</label>
        <input
          id="roster-path"
          type="text"
          bind:value={rosterPath}
          class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text font-mono focus:outline-none focus:border-secondary"
        />
      </div>

      <!-- Team Colors -->
      <div>
        <div class="flex items-center gap-2 mb-2">
          <span class="text-sm text-text-muted">Team Colors</span>
          <button
            class="text-xs text-secondary hover:text-text transition-colors"
            onclick={addColor}
          >+ Add</button>
        </div>
        <div class="flex flex-wrap gap-2">
          {#each colors as color, i}
            <div class="flex items-center gap-1">
              <input
                type="color"
                value={color}
                onchange={(e) => updateColor(i, (e.target as HTMLInputElement).value)}
                class="w-8 h-8 rounded border border-border cursor-pointer"
              />
              <button
                class="text-xs text-text-muted hover:text-accent transition-colors"
                onclick={() => removeColor(i)}
              >&times;</button>
            </div>
          {/each}
        </div>
      </div>

      <!-- Jersey Colors -->
      <div>
        <div class="flex items-center gap-2 mb-2">
          <span class="text-sm text-text-muted">Jersey Colors</span>
          <button
            class="text-xs text-secondary hover:text-text transition-colors"
            onclick={addJerseyColor}
          >+ Add</button>
        </div>
        <div class="flex flex-wrap gap-2">
          {#each jerseyColors as color, i}
            <div class="flex items-center gap-1">
              <input
                type="color"
                value={color}
                onchange={(e) => updateJerseyColor(i, (e.target as HTMLInputElement).value)}
                class="w-8 h-8 rounded border border-border cursor-pointer"
              />
              <button
                class="text-xs text-text-muted hover:text-accent transition-colors"
                onclick={() => removeJerseyColor(i)}
              >&times;</button>
            </div>
          {/each}
        </div>
      </div>

      <div class="pt-2">
        <button
          class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          onclick={handleSave}
          disabled={saving}
        >
          {saving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>
  {/if}
</div>
