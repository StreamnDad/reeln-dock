<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { TeamProfile } from "$lib/types/team";
  import {
    listTeamLevels,
    listTeamProfiles,
    saveTeamProfile,
    deleteTeamProfile,
    cloneTeamProfile,
  } from "$lib/ipc/teams";
  import { updateTeamInMap, removeTeamFromMap, loadAllTeams } from "$lib/stores/teams.svelte";
  import { extractDominantColors } from "$lib/utils/color-extract";
  import TeamLogo from "$lib/components/TeamLogo.svelte";

  interface Props {
    teamKey: string;
    onDelete?: () => void;
    onClone?: (newKey: string) => void;
  }

  let { teamKey, onDelete, onClone }: Props = $props();

  let team = $state<TeamProfile | null>(null);
  let originalName = $state("");
  let originalLevel = $state("");
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

  // Color suggestion
  let suggestedColors = $state<string[]>([]);
  let extractingColors = $state(false);

  // Clone state
  let showClone = $state(false);
  let cloneName = $state("");
  let cloning = $state(false);

  // Delete confirmation
  let confirmDelete = $state(false);
  let deleteTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  // Known levels for dropdown
  let knownLevels = $state<string[]>([]);
  let addingLevel = $state(false);
  let newLevelName = $state("");

  $effect(() => {
    loadTeam(teamKey);
    listTeamLevels().then((lvls) => { knownLevels = lvls; }).catch(() => {});
  });

  async function loadTeam(key: string) {
    loading = true;
    message = "";
    suggestedColors = [];
    showClone = false;
    confirmDelete = false;
    const parts = key.split("/");
    level = parts[0];
    const name = key.slice(level.length + 1);
    try {
      const profiles = await listTeamProfiles(level);
      const found = profiles.find((p) => p.team_name === name);
      if (found) {
        team = found;
        originalName = found.team_name;
        originalLevel = found.level;
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

      // If name or level changed, delete old file first
      const nameChanged = originalName !== updated.team_name;
      const levelChanged = originalLevel !== updated.level;
      if (nameChanged || levelChanged) {
        try {
          await deleteTeamProfile(originalLevel, originalName);
          removeTeamFromMap(originalName);
        } catch {
          // Old file may not exist if this is a fresh team
        }
      }

      const saved = await saveTeamProfile(updated);
      team = saved;
      originalName = saved.team_name;
      originalLevel = saved.level;
      updateTeamInMap(saved);
      await loadAllTeams();
      message = "Saved.";
    } catch (e) {
      message = `Error: ${e}`;
    } finally {
      saving = false;
    }
  }

  async function browseLogo() {
    const result = await open({
      title: "Select team logo",
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "svg", "webp"] }],
      directory: false,
      multiple: false,
    });
    if (!result) return;
    logoPath = result as string;
    // Auto-suggest colors if none configured
    if (colors.length === 0) {
      await suggestColors();
    }
  }

  async function browseRoster() {
    const result = await open({
      title: "Select roster file",
      filters: [{ name: "Documents", extensions: ["pdf", "csv", "xlsx", "xls", "txt"] }],
      directory: false,
      multiple: false,
    });
    if (!result) return;
    rosterPath = result as string;
  }

  async function suggestColors() {
    if (!logoPath) return;
    extractingColors = true;
    try {
      suggestedColors = await extractDominantColors(logoPath, 5);
    } catch {
      suggestedColors = [];
    } finally {
      extractingColors = false;
    }
  }

  function acceptSuggestedColor(hex: string) {
    if (!colors.includes(hex)) {
      colors = [...colors, hex];
    }
    suggestedColors = suggestedColors.filter((c) => c !== hex);
  }

  function acceptAllSuggested() {
    for (const hex of suggestedColors) {
      if (!colors.includes(hex)) {
        colors = [...colors, hex];
      }
    }
    suggestedColors = [];
  }

  function dismissSuggestions() {
    suggestedColors = [];
  }

  // Color management
  function addColor() { colors = [...colors, "#000000"]; }
  function removeColor(index: number) { colors = colors.filter((_, i) => i !== index); }
  function updateColor(index: number, value: string) { colors = colors.map((c, i) => (i === index ? value : c)); }
  function addJerseyColor() { jerseyColors = [...jerseyColors, "#000000"]; }
  function removeJerseyColor(index: number) { jerseyColors = jerseyColors.filter((_, i) => i !== index); }
  function updateJerseyColor(index: number, value: string) { jerseyColors = jerseyColors.map((c, i) => (i === index ? value : c)); }

  // Clone
  async function handleClone() {
    if (!team || !cloneName.trim()) return;
    cloning = true;
    try {
      const cloned = await cloneTeamProfile(originalLevel, originalName, cloneName.trim());
      updateTeamInMap(cloned);
      await loadAllTeams();
      showClone = false;
      cloneName = "";
      onClone?.(`${cloned.level}/${cloned.team_name}`);
    } catch (e) {
      message = `Clone error: ${e}`;
    } finally {
      cloning = false;
    }
  }

  // Delete
  function startDelete() {
    confirmDelete = true;
    deleteTimer = setTimeout(() => { confirmDelete = false; }, 3000);
  }

  async function confirmDeleteTeam() {
    if (!team) return;
    if (deleteTimer) clearTimeout(deleteTimer);
    confirmDelete = false;
    try {
      await deleteTeamProfile(originalLevel, originalName);
      removeTeamFromMap(originalName);
      await loadAllTeams();
      onDelete?.();
    } catch (e) {
      message = `Delete error: ${e}`;
    }
  }

  function cancelDelete() {
    if (deleteTimer) clearTimeout(deleteTimer);
    confirmDelete = false;
  }

  function addNewLevel() {
    const trimmed = newLevelName.trim();
    if (trimmed && !knownLevels.includes(trimmed)) {
      knownLevels = [...knownLevels, trimmed].sort();
    }
    level = trimmed;
    addingLevel = false;
    newLevelName = "";
  }
</script>

<div class="space-y-6">
  {#if loading}
    <p class="text-text-muted text-sm">Loading team...</p>
  {:else if !team}
    <p class="text-text-muted text-sm">Team not found.</p>
  {:else}
    <!-- Header: Logo + Name + Actions -->
    <div class="flex items-start gap-5">
      <div class="shrink-0">
        <TeamLogo teamName={originalName} size="xl" />
      </div>
      <div class="flex-1 min-w-0 pt-1">
        <h2 class="text-xl font-bold truncate">{teamName}</h2>
        {#if shortName && shortName !== teamName}
          <p class="text-sm text-text-muted">{shortName}</p>
        {/if}
        <div class="flex items-center gap-2 mt-2">
          <span class="px-2 py-0.5 rounded bg-bg text-xs text-text-muted border border-border">{level}</span>
          {#if colors.length > 0}
            <div class="flex gap-1">
              {#each colors.slice(0, 5) as color}
                <span class="w-4 h-4 rounded-full border border-white/20" style="background: {color}"></span>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <div class="flex gap-2 shrink-0">
        <button
          class="px-3 py-1.5 text-sm rounded-lg border border-border text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
          onclick={() => { showClone = !showClone; cloneName = `${teamName} Copy`; }}
        >Clone</button>
        {#if !confirmDelete}
          <button
            class="px-3 py-1.5 text-sm rounded-lg border border-accent/50 text-accent/70 hover:text-accent hover:bg-accent/10 transition-colors"
            onclick={startDelete}
          >Delete</button>
        {:else}
          <div class="flex gap-1">
            <button
              class="px-3 py-1.5 text-sm rounded-lg bg-accent text-text font-medium transition-colors"
              onclick={confirmDeleteTeam}
            >Confirm</button>
            <button
              class="px-2 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
              onclick={cancelDelete}
            >Cancel</button>
          </div>
        {/if}
      </div>
    </div>

    <!-- Clone inline -->
    {#if showClone}
      <div class="bg-surface rounded-lg border border-border p-4">
        <h3 class="text-sm font-semibold mb-2">Clone Team</h3>
        <div class="flex gap-2">
          <input
            type="text"
            bind:value={cloneName}
            placeholder="New team name..."
            class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
          />
          <button
            class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors disabled:opacity-50"
            onclick={handleClone}
            disabled={cloning || !cloneName.trim()}
          >{cloning ? "Cloning..." : "Clone"}</button>
          <button
            class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
            onclick={() => showClone = false}
          >Cancel</button>
        </div>
      </div>
    {/if}

    {#if message}
      <p class="text-sm text-text-muted">{message}</p>
    {/if}

    <!-- Main form -->
    <div class="bg-surface rounded-lg border border-border p-4 space-y-5">
      <!-- Basic info -->
      <div>
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">Basic Info</h3>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm text-text-muted mb-1" for="td-team-name">Team Name</label>
            <input
              id="td-team-name"
              type="text"
              bind:value={teamName}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            />
          </div>
          <div>
            <label class="block text-sm text-text-muted mb-1" for="td-short-name">Short Name</label>
            <input
              id="td-short-name"
              type="text"
              bind:value={shortName}
              class="w-full px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            />
          </div>
        </div>
        <div class="mt-3">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label class="block text-sm text-text-muted mb-1">Level</label>
          {#if addingLevel}
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={newLevelName}
                placeholder="New level name..."
                class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
                onkeydown={(e) => { if (e.key === "Enter") addNewLevel(); }}
              />
              <button class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors" onclick={addNewLevel}>Add</button>
              <button class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors" onclick={() => addingLevel = false}>Cancel</button>
            </div>
          {:else}
            <div class="flex gap-2">
              <select
                bind:value={level}
                class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
              >
                {#each knownLevels as lvl}
                  <option value={lvl}>{lvl}</option>
                {/each}
              </select>
              <button
                class="px-3 py-1.5 text-sm text-secondary hover:text-text transition-colors"
                onclick={() => addingLevel = true}
              >+ New</button>
            </div>
          {/if}
        </div>
      </div>

      <!-- Logo -->
      <div class="border-t border-border pt-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">Logo</h3>
        <div class="flex gap-2">
          <input
            type="text"
            bind:value={logoPath}
            placeholder="Path to logo image..."
            class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text font-mono focus:outline-none focus:border-secondary"
          />
          <button
            class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
            onclick={browseLogo}
          >Browse</button>
        </div>
        {#if logoPath && colors.length === 0 && suggestedColors.length === 0}
          <button
            class="mt-2 text-xs text-secondary hover:text-text transition-colors"
            onclick={suggestColors}
            disabled={extractingColors}
          >{extractingColors ? "Extracting..." : "Suggest colors from logo"}</button>
        {/if}
      </div>

      <!-- Suggested Colors -->
      {#if suggestedColors.length > 0}
        <div class="bg-bg/50 rounded-lg border border-secondary/30 p-3">
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-semibold text-secondary">Suggested Colors</span>
            <div class="flex gap-2">
              <button class="text-xs text-secondary hover:text-text transition-colors" onclick={acceptAllSuggested}>Accept All</button>
              <button class="text-xs text-text-muted hover:text-text transition-colors" onclick={dismissSuggestions}>Dismiss</button>
            </div>
          </div>
          <div class="flex gap-2">
            {#each suggestedColors as hex}
              <button
                class="flex items-center gap-1.5 px-2 py-1 bg-surface rounded-lg border border-border hover:border-secondary transition-colors"
                onclick={() => acceptSuggestedColor(hex)}
                title="Click to add"
              >
                <span class="w-5 h-5 rounded-full border border-white/20" style="background: {hex}"></span>
                <span class="text-xs text-text-muted font-mono">{hex}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Team Colors -->
      <div class="border-t border-border pt-4">
        <div class="flex items-center gap-2 mb-2">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Team Colors</h3>
          <button class="text-xs text-secondary hover:text-text transition-colors" onclick={addColor}>+ Add</button>
          {#if logoPath}
            <button
              class="text-xs text-text-muted hover:text-secondary transition-colors ml-auto"
              onclick={suggestColors}
              disabled={extractingColors}
            >{extractingColors ? "..." : "Suggest from logo"}</button>
          {/if}
        </div>
        {#if colors.length === 0}
          <p class="text-xs text-text-muted">No colors configured.</p>
        {:else}
          <div class="flex flex-wrap gap-2">
            {#each colors as color, i}
              <div class="flex items-center gap-1">
                <input
                  type="color"
                  value={color}
                  onchange={(e) => updateColor(i, (e.target as HTMLInputElement).value)}
                  class="w-8 h-8 rounded border border-border cursor-pointer"
                />
                <span class="text-xs text-text-muted font-mono">{color}</span>
                <button
                  class="text-xs text-text-muted hover:text-accent transition-colors"
                  onclick={() => removeColor(i)}
                >&times;</button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Jersey Colors -->
      <div class="border-t border-border pt-4">
        <div class="flex items-center gap-2 mb-2">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Jersey Colors</h3>
          <button class="text-xs text-secondary hover:text-text transition-colors" onclick={addJerseyColor}>+ Add</button>
        </div>
        {#if jerseyColors.length === 0}
          <p class="text-xs text-text-muted">No jersey colors configured.</p>
        {:else}
          <div class="flex flex-wrap gap-2">
            {#each jerseyColors as color, i}
              <div class="flex items-center gap-1">
                <input
                  type="color"
                  value={color}
                  onchange={(e) => updateJerseyColor(i, (e.target as HTMLInputElement).value)}
                  class="w-8 h-8 rounded border border-border cursor-pointer"
                />
                <span class="text-xs text-text-muted font-mono">{color}</span>
                <button
                  class="text-xs text-text-muted hover:text-accent transition-colors"
                  onclick={() => removeJerseyColor(i)}
                >&times;</button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Roster -->
      <div class="border-t border-border pt-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">Roster</h3>
        <div class="flex gap-2">
          <input
            type="text"
            bind:value={rosterPath}
            placeholder="Path to roster file..."
            class="flex-1 px-3 py-1.5 bg-bg border border-border rounded-lg text-sm text-text font-mono focus:outline-none focus:border-secondary"
          />
          <button
            class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
            onclick={browseRoster}
          >Browse</button>
        </div>
        {#if rosterPath}
          <p class="text-xs text-text-muted mt-1">Reference file only. Roster editing coming soon.</p>
        {/if}
      </div>

      <!-- Save -->
      <div class="border-t border-border pt-4">
        <button
          class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          onclick={handleSave}
          disabled={saving || !teamName.trim()}
        >{saving ? "Saving..." : "Save"}</button>
      </div>
    </div>
  {/if}
</div>
