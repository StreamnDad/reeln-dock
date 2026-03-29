<script lang="ts">
  import type { SportAlias } from "$lib/types/sport";
  import type { TeamProfile } from "$lib/types/team";
  import type { ConfigProfile, PluginDetail } from "$lib/types/plugin";
  import { initGame, listSports, executePluginHook } from "$lib/ipc/games";
  import { listTeamLevels, listTeamProfiles, saveTeamProfile } from "$lib/ipc/teams";
  import { listConfigProfiles, listPluginsForProfile } from "$lib/ipc/plugins";
  import { getPromptTemplate, savePromptTemplate } from "$lib/ipc/prompts";
  import type { GameSummary } from "$lib/types/game";
  import { log } from "$lib/stores/log.svelte";

  interface Props {
    onclose: () => void;
    allGames?: GameSummary[];
    onGameCreated?: (game: GameSummary) => void;
    onSelectGame?: (dirPath: string) => void;
  }

  let { onclose, allGames = [], onGameCreated, onSelectGame }: Props = $props();

  // Wizard step
  type Step = "level" | "teams" | "details" | "ai" | "hooks";
  let step = $state<Step>("level");

  // Data sources
  let sports = $state<SportAlias[]>([]);
  let knownLevels = $state<string[]>([]);
  let teamsForLevel = $state<TeamProfile[]>([]);
  let configProfiles = $state<ConfigProfile[]>([]);
  let selectedConfigProfile = $state<ConfigProfile | null>(null);
  let profilePlugins = $state<PluginDetail[]>([]);

  let historyLevels = $derived(
    [...new Set(allGames.map((g) => g.state.game_info.level).filter(Boolean))].sort(),
  );
  let allLevels = $derived(
    [...new Set([...knownLevels, ...historyLevels])].sort(),
  );
  let allTournaments = $derived(
    [...new Set(allGames.map((g) => g.state.game_info.tournament).filter(Boolean))].sort(),
  );
  let allVenues = $derived(
    [...new Set(allGames.map((g) => g.state.game_info.venue).filter(Boolean))].sort(),
  );

  // Form state
  let level = $state("");
  let addingNewLevel = $state(false);
  let newLevelName = $state("");
  let sport = $state("hockey");
  let homeTeam = $state("");
  let awayTeam = $state("");
  let addingNewHome = $state(false);
  let addingNewAway = $state(false);
  let newHomeProfile = $state<Partial<TeamProfile>>({ team_name: "", short_name: "", colors: [] });
  let newAwayProfile = $state<Partial<TeamProfile>>({ team_name: "", short_name: "", colors: [] });
  let date = $state(new Date().toISOString().slice(0, 10));
  let venue = $state("");
  let addingNewVenue = $state(false);
  let newVenueName = $state("");
  let gameTime = $state("");
  let tournament = $state("");
  let addingNewTournament = $state(false);
  let newTournamentName = $state("");

  // AI preview state
  interface PromptState {
    name: string;
    rawTemplate: string;       // Original template with {{vars}}
    rendered: string;          // Rendered with variables substituted
    expanded: boolean;
    editing: boolean;
    editText: string;          // Current edit buffer (raw template text)
    overridden: boolean;       // True if user has a temp override for this run
  }
  let prompts = $state<PromptState[]>([]);
  let promptsLoading = $state(false);

  // Hook execution state
  interface HookStep {
    id: string;
    label: string;
    description: string;
    status: "pending" | "running" | "done" | "failed";
    result?: string;
    error?: string;
    imageUrl?: string;
    expanded: boolean;
  }
  let hookSteps = $state<HookStep[]>([]);
  let hooksDone = $state(false);
  let createdDirPath = $state("");
  let error = $state("");

  // Known teams for selected level from game history
  let historyTeamsForLevel = $derived(() => {
    if (!level) return [];
    const teamSet = new Set<string>();
    for (const g of allGames) {
      if (g.state.game_info.level === level) {
        if (g.state.game_info.home_team) teamSet.add(g.state.game_info.home_team);
        if (g.state.game_info.away_team) teamSet.add(g.state.game_info.away_team);
      }
    }
    return Array.from(teamSet).sort();
  });

  // Combined team list: profiles + history
  let allTeamNames = $derived(() => {
    const profileNames = teamsForLevel.map((t) => t.team_name);
    const historyNames = historyTeamsForLevel();
    return [...new Set([...profileNames, ...historyNames])].sort();
  });

  // Load initial data
  $effect(() => {
    listSports()
      .then((s) => { sports = s; })
      .catch((e) => log.error("NewGame", `Failed to load sports: ${e}`));
    listTeamLevels()
      .then((l) => { knownLevels = l; })
      .catch(() => {});
    listConfigProfiles()
      .then((p) => {
        configProfiles = p;
        // Auto-select the active profile
        const active = p.find((cp) => cp.active);
        if (active) selectConfigProfile(active);
        else if (p.length > 0) selectConfigProfile(p[0]);
      })
      .catch((e) => log.error("NewGame", `Failed to load config profiles: ${e}`));
  });

  function selectConfigProfile(profile: ConfigProfile) {
    selectedConfigProfile = profile;
    listPluginsForProfile(profile.path)
      .then((p) => { profilePlugins = p; })
      .catch(() => { profilePlugins = []; });
  }

  let enabledPluginNames = $derived(
    profilePlugins.filter((p) => p.enabled).map((p) => p.name),
  );

  // Load team profiles when level changes
  $effect(() => {
    if (level) {
      listTeamProfiles(level)
        .then((t) => { teamsForLevel = t; })
        .catch(() => { teamsForLevel = []; });
    } else {
      teamsForLevel = [];
    }
  });

  function confirmNewLevel() {
    const name = newLevelName.trim();
    if (name) {
      level = name;
      addingNewLevel = false;
      newLevelName = "";
    }
  }

  function confirmNewTournament() {
    const name = newTournamentName.trim();
    if (name) {
      tournament = name;
      addingNewTournament = false;
      newTournamentName = "";
    }
  }

  function confirmNewVenue() {
    const name = newVenueName.trim();
    if (name) {
      venue = name;
      addingNewVenue = false;
      newVenueName = "";
    }
  }

  async function saveNewTeam(side: "home" | "away") {
    const profile = side === "home" ? newHomeProfile : newAwayProfile;
    if (!profile.team_name?.trim()) return;
    const full: TeamProfile = {
      team_name: profile.team_name!.trim(),
      short_name: profile.short_name?.trim() || profile.team_name!.trim(),
      level,
      logo_path: "",
      roster_path: "",
      colors: (profile.colors || []).filter(Boolean),
      jersey_colors: [],
      metadata: {},
    };
    try {
      await saveTeamProfile(full);
      teamsForLevel = [...teamsForLevel, full];
      if (side === "home") {
        homeTeam = full.team_name;
        addingNewHome = false;
      } else {
        awayTeam = full.team_name;
        addingNewAway = false;
      }
      log.info("NewGame", `Saved team profile: ${full.team_name}`);
    } catch (err) {
      log.error("NewGame", `Failed to save team: ${err}`);
    }
  }

  function canAdvanceFrom(s: Step): boolean {
    switch (s) {
      case "level": return !!level && !!sport;
      case "teams": return !!homeTeam && !!awayTeam;
      case "details": return !!date;
      case "ai": return true;
      default: return false;
    }
  }

  function nextStep() {
    switch (step) {
      case "level": step = "teams"; break;
      case "teams": step = "details"; break;
      case "details": step = "ai"; loadPromptPreviews(); break;
      case "ai": runHookPipeline(); break;
    }
  }

  function prevStep() {
    switch (step) {
      case "teams": step = "level"; break;
      case "details": step = "teams"; break;
      case "ai": step = "details"; break;
    }
  }

  function getPromptVars(): Record<string, string> {
    return {
      home_team: homeTeam,
      away_team: awayTeam,
      date,
      sport,
      venue,
      game_time: gameTime,
      level,
      tournament,
      description: "",
    };
  }

  function substituteVars(template: string, vars: Record<string, string>): string {
    let result = template;
    for (const [key, value] of Object.entries(vars)) {
      result = result.replaceAll(`{{${key}}}`, value);
    }
    return result;
  }

  async function loadPromptPreviews() {
    promptsLoading = true;
    const vars = getPromptVars();
    const relevant = ["livestream_title", "livestream_description", "game_image"];
    const loaded: PromptState[] = [];

    for (const name of relevant) {
      try {
        const tpl = await getPromptTemplate(name);
        const rendered = substituteVars(tpl.content, vars);
        loaded.push({
          name,
          rawTemplate: tpl.content,
          rendered,
          expanded: false,
          editing: false,
          editText: tpl.content,
          overridden: false,
        });
      } catch {
        // Template not found — skip
      }
    }
    prompts = loaded;
    promptsLoading = false;
  }

  function togglePromptExpanded(name: string) {
    prompts = prompts.map((p) =>
      p.name === name ? { ...p, expanded: !p.expanded, editing: false } : p,
    );
  }

  function startEditingPrompt(name: string) {
    prompts = prompts.map((p) =>
      p.name === name ? { ...p, editing: true, editText: p.overridden ? p.editText : p.rawTemplate } : p,
    );
  }

  function cancelEditingPrompt(name: string) {
    prompts = prompts.map((p) =>
      p.name === name ? { ...p, editing: false } : p,
    );
  }

  function applyPromptThisRun(name: string) {
    const vars = getPromptVars();
    prompts = prompts.map((p) => {
      if (p.name !== name) return p;
      const rendered = substituteVars(p.editText, vars);
      return { ...p, rendered, overridden: true, editing: false };
    });
  }

  async function savePromptPermanently(name: string) {
    const prompt = prompts.find((p) => p.name === name);
    if (!prompt) return;
    try {
      await savePromptTemplate(name, prompt.editText);
      const vars = getPromptVars();
      const rendered = substituteVars(prompt.editText, vars);
      prompts = prompts.map((p) => {
        if (p.name !== name) return p;
        return { ...p, rawTemplate: prompt.editText, rendered, overridden: false, editing: false };
      });
      log.info("NewGame", `Saved template: ${name}`);
    } catch (err) {
      log.error("NewGame", `Failed to save template: ${err}`);
    }
  }

  function updatePromptEditText(name: string, text: string) {
    prompts = prompts.map((p) =>
      p.name === name ? { ...p, editText: text } : p,
    );
  }

  function toggleHookExpanded(id: string) {
    hookSteps = hookSteps.map((h) =>
      h.id === id ? { ...h, expanded: !h.expanded } : h,
    );
  }

  function updateHook(id: string, updates: Partial<HookStep>) {
    hookSteps = hookSteps.map((h) =>
      h.id === id ? { ...h, ...updates } : h,
    );
  }

  async function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // Shared context passed between hook phases (on_game_init → on_game_ready)
  let hookShared = $state<Record<string, unknown>>({});

  async function runSingleHook(hook: HookStep): Promise<void> {
    log.info("NewGame", `Running hook: ${hook.id}`);
    updateHook(hook.id, { status: "running", expanded: true });

    try {
      switch (hook.id) {
        case "create_game": {
          const result = await initGame({
            sport,
            homeTeam: homeTeam.trim(),
            awayTeam: awayTeam.trim(),
            date,
            venue: venue.trim() || undefined,
            gameTime: gameTime.trim() || undefined,
            level: level.trim() || undefined,
            tournament: tournament.trim() || undefined,
          });
          onGameCreated?.(result);
          createdDirPath = result.dir_path;
          updateHook(hook.id, {
            status: "done",
            result: `Created: ${result.dir_path}`,
          });
          break;
        }
        case "on_game_init":
        case "on_game_ready": {
          const homeProfile = teamsForLevel.find((t) => t.team_name === homeTeam.trim());
          const awayProfile = teamsForLevel.find((t) => t.team_name === awayTeam.trim());

          const contextData: Record<string, unknown> = {
            game_dir: createdDirPath,
            game_info: {
              sport,
              home_team: homeTeam.trim(),
              away_team: awayTeam.trim(),
              date,
              venue: venue.trim(),
              game_time: gameTime.trim(),
              level: level.trim(),
              tournament: tournament.trim(),
            },
            ...(homeProfile ? { home_profile: homeProfile } : {}),
            ...(awayProfile ? { away_profile: awayProfile } : {}),
          };

          const result = await executePluginHook(
            hook.id,
            contextData,
            hookShared,
            selectedConfigProfile?.path,
          );

          // Accumulate shared state for next phase
          hookShared = { ...hookShared, ...result.shared };

          const logSummary = result.logs
            .filter((l) => !l.startsWith("reeln.plugins.loader:"))
            .join("\n");

          if (!result.success) {
            updateHook(hook.id, {
              status: "failed",
              error: result.errors.join("\n") || "Hook execution failed",
              result: logSummary || undefined,
              expanded: true,
            });
          } else if (result.errors.length > 0) {
            updateHook(hook.id, {
              status: "done",
              result: logSummary + "\n\nWarnings:\n" + result.errors.join("\n"),
            });
          } else {
            updateHook(hook.id, {
              status: "done",
              result: logSummary || "Hooks completed",
            });
          }
          break;
        }
        default:
          updateHook(hook.id, { status: "done" });
      }
    } catch (err) {
      const message = String(err);
      if (message.includes("reeln CLI not found")) {
        updateHook(hook.id, {
          status: "failed",
          error: "reeln CLI not installed. Install with: uv pip install reeln",
          expanded: true,
        });
      } else {
        updateHook(hook.id, {
          status: "failed",
          error: message,
          expanded: true,
        });
      }
    }
  }

  async function rerunHook(id: string) {
    const hook = hookSteps.find((h) => h.id === id);
    if (!hook) return;
    updateHook(id, { status: "pending", result: undefined, error: undefined, imageUrl: undefined });
    await sleep(100);
    await runSingleHook(hook);
  }

  function buildHookSteps(): HookStep[] {
    const steps: HookStep[] = [
      {
        id: "create_game",
        label: "Create Game",
        description: "Create game directory, segment folders, and game.json",
        status: "pending",
        expanded: false,
      },
    ];

    // Add real lifecycle hooks when plugins are enabled
    if (enabledPluginNames.length > 0) {
      steps.push({
        id: "on_game_init",
        label: "Plugin Hooks (init)",
        description: `Run ON_GAME_INIT for: ${enabledPluginNames.join(", ")}`,
        status: "pending",
        expanded: false,
      });
      steps.push({
        id: "on_game_ready",
        label: "Plugin Hooks (ready)",
        description: `Run ON_GAME_READY for: ${enabledPluginNames.join(", ")}`,
        status: "pending",
        expanded: false,
      });
    }

    return steps;
  }

  async function runHookPipeline() {
    step = "hooks";
    error = "";
    hooksDone = false;
    hookShared = {};

    hookSteps = buildHookSteps();

    // Snapshot the step IDs to iterate — avoid issues with $state proxy
    // reassignment during updateHook calls
    const stepIds = hookSteps.map((h) => h.id);

    for (const id of stepIds) {
      const hook = hookSteps.find((h) => h.id === id);
      if (hook) {
        await runSingleHook(hook);
        // Stop pipeline if a step failed
        if (hookSteps.find((h) => h.id === id)?.status === "failed") break;
      }
    }

    hooksDone = true;
    log.info("NewGame", `Game created: ${createdDirPath}`);
  }

  function handleDone() {
    if (createdDirPath) {
      onSelectGame?.(createdDirPath);
    }
    onclose();
  }

  const stepOrder: Step[] = ["level", "teams", "details", "ai"];
  let stepIndex = $derived(stepOrder.indexOf(step));

  // Status indicator colors
  function statusColor(status: HookStep["status"]): string {
    switch (status) {
      case "pending": return "bg-zinc-600";
      case "running": return "bg-blue-500 animate-pulse";
      case "done": return "bg-green-500";
      case "failed": return "bg-red-500";
    }
  }

  function statusIcon(status: HookStep["status"]): string {
    switch (status) {
      case "pending": return "\u25CB"; // ○
      case "running": return "\u25CF"; // ●
      case "done": return "\u2713";    // ✓
      case "failed": return "\u2717";  // ✗
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  onclick={(e) => { if (e.target === e.currentTarget && step !== "hooks") onclose(); }}
>
  <div class="bg-surface rounded-xl border border-border w-[36rem] max-h-[85vh] flex flex-col">
    <!-- Header with step indicator -->
    <div class="px-6 pt-5 pb-3 border-b border-border">
      <h2 class="text-lg font-bold mb-3">New Game</h2>
      {#if step !== "hooks"}
        <div class="flex gap-1">
          {#each stepOrder as _, i}
            <div
              class="flex-1 h-1 rounded-full transition-colors"
              class:bg-primary={i <= stepIndex}
              class:bg-border={i > stepIndex}
            ></div>
          {/each}
        </div>
        <div class="flex justify-between mt-1.5 text-[10px] uppercase tracking-wider text-text-muted">
          <span class:text-text={step === "level"}>Level</span>
          <span class:text-text={step === "teams"}>Teams</span>
          <span class:text-text={step === "details"}>Details</span>
          <span class:text-text={step === "ai"}>AI & Plugins</span>
        </div>
      {:else}
        <p class="text-sm text-text-muted">
          {#if hooksDone}
            All hooks complete
          {:else}
            Running hooks...
          {/if}
        </p>
      {/if}
    </div>

    <!-- Step content -->
    <div class="flex-1 overflow-y-auto px-6 py-4">
      {#if error && step !== "hooks"}
        <div class="mb-3 px-3 py-2 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-300">
          {error}
        </div>
      {/if}

      <!-- STEP: Level & Sport -->
      {#if step === "level"}
        <div class="space-y-4">
          <div>
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="ng-level">
              Level
            </label>
            {#if addingNewLevel}
              <div class="flex gap-2">
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  type="text"
                  bind:value={newLevelName}
                  placeholder="e.g. Varsity, JV, Bantam AA"
                  class="flex-1 px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                  onkeydown={(e) => { if (e.key === "Enter") confirmNewLevel(); if (e.key === "Escape") (addingNewLevel = false); }}
                  autofocus
                />
                <button
                  class="px-3 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
                  onclick={confirmNewLevel}
                >
                  Add
                </button>
                <button
                  class="px-3 py-2 text-text-muted hover:text-text text-sm transition-colors"
                  onclick={() => (addingNewLevel = false)}
                >
                  Cancel
                </button>
              </div>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#each allLevels as l}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={level === l}
                    class:border-primary={level === l}
                    class:text-text={level === l}
                    class:bg-bg={level !== l}
                    class:border-border={level !== l}
                    class:text-text-muted={level !== l}
                    class:hover:border-secondary={level !== l}
                    onclick={() => (level = l)}
                  >
                    {l}
                  </button>
                {/each}
                <button
                  class="px-3 py-2 rounded-lg text-sm font-medium border border-dashed border-border text-text-muted hover:border-secondary hover:text-text transition-colors"
                  onclick={() => (addingNewLevel = true)}
                >
                  + New Level
                </button>
              </div>
            {/if}
          </div>

          <div>
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="ng-sport">
              Sport
            </label>
            <select
              id="ng-sport"
              bind:value={sport}
              class="w-full px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            >
              {#each sports as s}
                <option value={s.sport}>{s.sport} ({s.segment_count} {s.segment_name}s)</option>
              {/each}
              {#if sports.length === 0}
                <option value="hockey">hockey</option>
              {/if}
            </select>
          </div>

          <!-- Config Profile -->
          {#if configProfiles.length > 0}
            <div>
              <!-- svelte-ignore a11y_label_has_associated_control -->
              <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">
                Config Profile
              </label>
              <div class="flex flex-wrap gap-2">
                {#each configProfiles as cp (cp.path)}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={selectedConfigProfile?.path === cp.path}
                    class:border-primary={selectedConfigProfile?.path === cp.path}
                    class:text-text={selectedConfigProfile?.path === cp.path}
                    class:bg-bg={selectedConfigProfile?.path !== cp.path}
                    class:border-border={selectedConfigProfile?.path !== cp.path}
                    class:text-text-muted={selectedConfigProfile?.path !== cp.path}
                    class:hover:border-secondary={selectedConfigProfile?.path !== cp.path}
                    onclick={() => selectConfigProfile(cp)}
                  >
                    {cp.name}
                    {#if cp.active}
                      <span class="ml-1 text-xs opacity-60">(active)</span>
                    {/if}
                  </button>
                {/each}
              </div>
              {#if enabledPluginNames.length > 0}
                <p class="text-xs text-text-muted mt-1.5">
                  Plugins: {enabledPluginNames.join(", ")}
                </p>
              {/if}
            </div>
          {/if}
        </div>

      <!-- STEP: Teams -->
      {:else if step === "teams"}
        <div class="space-y-5">
          <!-- Home Team -->
          <div>
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">
              Home Team
            </label>
            {#if addingNewHome}
              <div class="bg-bg rounded-lg border border-border p-3 space-y-2">
                <input
                  type="text"
                  bind:value={newHomeProfile.team_name}
                  placeholder="Team name (e.g. Roseville Raiders)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                />
                <input
                  type="text"
                  bind:value={newHomeProfile.short_name}
                  placeholder="Short name (e.g. Roseville)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                />
                <input
                  type="text"
                  placeholder="Colors (comma-separated, e.g. #CC0000, #FFFFFF)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                  oninput={(e) => {
                    newHomeProfile.colors = (e.currentTarget as HTMLInputElement).value.split(",").map(s => s.trim());
                  }}
                />
                <div class="flex gap-2">
                  <button
                    class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
                    onclick={() => saveNewTeam("home")}
                  >
                    Save Team
                  </button>
                  <button
                    class="px-3 py-1.5 text-text-muted hover:text-text text-sm transition-colors"
                    onclick={() => (addingNewHome = false)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#each allTeamNames() as team}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={homeTeam === team}
                    class:border-primary={homeTeam === team}
                    class:text-text={homeTeam === team}
                    class:bg-bg={homeTeam !== team}
                    class:border-border={homeTeam !== team}
                    class:text-text-muted={homeTeam !== team}
                    class:hover:border-secondary={homeTeam !== team}
                    onclick={() => (homeTeam = team)}
                  >
                    {team}
                  </button>
                {/each}
                <button
                  class="px-3 py-2 rounded-lg text-sm font-medium border border-dashed border-border text-text-muted hover:border-secondary hover:text-text transition-colors"
                  onclick={() => (addingNewHome = true)}
                >
                  + New Team
                </button>
              </div>
            {/if}
          </div>

          <!-- Away Team -->
          <div>
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">
              Away Team
            </label>
            {#if addingNewAway}
              <div class="bg-bg rounded-lg border border-border p-3 space-y-2">
                <input
                  type="text"
                  bind:value={newAwayProfile.team_name}
                  placeholder="Team name (e.g. Mahtomedi Zephyrs)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                />
                <input
                  type="text"
                  bind:value={newAwayProfile.short_name}
                  placeholder="Short name (e.g. Mahtomedi)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                />
                <input
                  type="text"
                  placeholder="Colors (comma-separated, e.g. #003399, #FFD700)"
                  class="w-full px-3 py-2 bg-surface border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                  oninput={(e) => {
                    newAwayProfile.colors = (e.currentTarget as HTMLInputElement).value.split(",").map(s => s.trim());
                  }}
                />
                <div class="flex gap-2">
                  <button
                    class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
                    onclick={() => saveNewTeam("away")}
                  >
                    Save Team
                  </button>
                  <button
                    class="px-3 py-1.5 text-text-muted hover:text-text text-sm transition-colors"
                    onclick={() => (addingNewAway = false)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#each allTeamNames() as team}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={awayTeam === team}
                    class:border-primary={awayTeam === team}
                    class:text-text={awayTeam === team}
                    class:bg-bg={awayTeam !== team}
                    class:border-border={awayTeam !== team}
                    class:text-text-muted={awayTeam !== team}
                    class:hover:border-secondary={awayTeam !== team}
                    onclick={() => (awayTeam = team)}
                  >
                    {team}
                  </button>
                {/each}
                <button
                  class="px-3 py-2 rounded-lg text-sm font-medium border border-dashed border-border text-text-muted hover:border-secondary hover:text-text transition-colors"
                  onclick={() => (addingNewAway = true)}
                >
                  + New Team
                </button>
              </div>
            {/if}
          </div>
        </div>

      <!-- STEP: Details -->
      {:else if step === "details"}
        <div class="space-y-4">
          <div>
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1" for="ng-date">
              Date
            </label>
            <input
              id="ng-date"
              type="date"
              bind:value={date}
              class="w-full px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text focus:outline-none focus:border-secondary"
            />
          </div>

          <!-- Tournament selector -->
          <div>
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">
              Tournament
            </label>
            {#if addingNewTournament}
              <div class="flex gap-2">
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  type="text"
                  bind:value={newTournamentName}
                  placeholder="e.g. Section 4AA, State Tournament"
                  class="flex-1 px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                  onkeydown={(e) => { if (e.key === "Enter") confirmNewTournament(); if (e.key === "Escape") (addingNewTournament = false); }}
                  autofocus
                />
                <button
                  class="px-3 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
                  onclick={confirmNewTournament}
                >
                  Add
                </button>
                <button
                  class="px-3 py-2 text-text-muted hover:text-text text-sm transition-colors"
                  onclick={() => (addingNewTournament = false)}
                >
                  Cancel
                </button>
              </div>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#if tournament}
                  <button
                    class="px-3 py-1.5 rounded-lg text-xs font-medium border border-dashed border-border text-text-muted hover:text-text transition-colors"
                    onclick={() => (tournament = "")}
                  >
                    Clear
                  </button>
                {/if}
                {#each allTournaments as t}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={tournament === t}
                    class:border-primary={tournament === t}
                    class:text-text={tournament === t}
                    class:bg-bg={tournament !== t}
                    class:border-border={tournament !== t}
                    class:text-text-muted={tournament !== t}
                    class:hover:border-secondary={tournament !== t}
                    onclick={() => (tournament = t)}
                  >
                    {t}
                  </button>
                {/each}
                <button
                  class="px-3 py-2 rounded-lg text-sm font-medium border border-dashed border-border text-text-muted hover:border-secondary hover:text-text transition-colors"
                  onclick={() => (addingNewTournament = true)}
                >
                  + New
                </button>
              </div>
              {#if allTournaments.length === 0 && !tournament}
                <p class="text-xs text-text-muted mt-1">No tournaments yet. Optional — add one or skip.</p>
              {/if}
            {/if}
          </div>

          <!-- Venue selector -->
          <div>
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5">
              Venue
            </label>
            {#if addingNewVenue}
              <div class="flex gap-2">
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  type="text"
                  bind:value={newVenueName}
                  placeholder="e.g. Roseville Ice Arena"
                  class="flex-1 px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
                  onkeydown={(e) => { if (e.key === "Enter") confirmNewVenue(); if (e.key === "Escape") (addingNewVenue = false); }}
                  autofocus
                />
                <button
                  class="px-3 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors"
                  onclick={confirmNewVenue}
                >
                  Add
                </button>
                <button
                  class="px-3 py-2 text-text-muted hover:text-text text-sm transition-colors"
                  onclick={() => (addingNewVenue = false)}
                >
                  Cancel
                </button>
              </div>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#if venue}
                  <button
                    class="px-3 py-1.5 rounded-lg text-xs font-medium border border-dashed border-border text-text-muted hover:text-text transition-colors"
                    onclick={() => (venue = "")}
                  >
                    Clear
                  </button>
                {/if}
                {#each allVenues as v}
                  <button
                    class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
                    class:bg-primary={venue === v}
                    class:border-primary={venue === v}
                    class:text-text={venue === v}
                    class:bg-bg={venue !== v}
                    class:border-border={venue !== v}
                    class:text-text-muted={venue !== v}
                    class:hover:border-secondary={venue !== v}
                    onclick={() => (venue = v)}
                  >
                    {v}
                  </button>
                {/each}
                <button
                  class="px-3 py-2 rounded-lg text-sm font-medium border border-dashed border-border text-text-muted hover:border-secondary hover:text-text transition-colors"
                  onclick={() => (addingNewVenue = true)}
                >
                  + New
                </button>
              </div>
              {#if allVenues.length === 0 && !venue}
                <p class="text-xs text-text-muted mt-1">No venues yet. Optional — add one or skip.</p>
              {/if}
            {/if}
          </div>

          <div>
            <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1" for="ng-time">
              Game Time
            </label>
            <input
              id="ng-time"
              type="text"
              bind:value={gameTime}
              placeholder="e.g. 7:00 PM"
              class="w-full px-3 py-2 bg-bg border border-border rounded-lg text-sm text-text placeholder:text-text-muted focus:outline-none focus:border-secondary"
            />
          </div>

          <!-- Summary preview -->
          <div class="p-3 bg-bg rounded-lg border border-border text-sm">
            <span class="text-text-muted">Preview:</span>
            <span class="font-medium ml-1">{homeTeam} vs {awayTeam}</span>
            <span class="text-text-muted ml-1">| {date} | {level} {sport}</span>
            {#if tournament}
              <span class="text-text-muted ml-1">| {tournament}</span>
            {/if}
            {#if venue}
              <span class="text-text-muted ml-1">@ {venue}</span>
            {/if}
          </div>
        </div>

      <!-- STEP: AI & Plugins -->
      {:else if step === "ai"}
        <div class="space-y-3">
          <!-- Enabled plugins summary -->
          <div class="bg-bg rounded-lg border border-border p-3">
            <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">
              Enabled Plugins
              {#if selectedConfigProfile}
                <span class="font-normal normal-case ml-1">({selectedConfigProfile.name})</span>
              {/if}
            </h4>
            {#if enabledPluginNames.length === 0}
              <p class="text-sm text-text-muted">No plugins enabled in this profile.</p>
            {:else}
              <div class="flex flex-wrap gap-1.5">
                {#each enabledPluginNames as pluginName}
                  <span class="px-2 py-1 rounded bg-surface border border-border text-xs font-medium">{pluginName}</span>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Prompt templates — collapsed by default -->
          {#if promptsLoading}
            <p class="text-sm text-text-muted">Loading prompts...</p>
          {:else if prompts.length === 0}
            <div class="p-3 bg-bg rounded-lg border border-border text-sm text-text-muted">
              No prompt templates found.
            </div>
          {:else}
            <div class="space-y-1.5">
              <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
                Prompt Templates
              </h4>
              {#each prompts as prompt (prompt.name)}
                <div class="rounded-lg border overflow-hidden {prompt.overridden ? 'border-amber-600' : 'border-border'}">
                  <!-- Collapsed header — always visible -->
                  <button
                    class="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-bg/50 transition-colors"
                    onclick={() => togglePromptExpanded(prompt.name)}
                  >
                    <span class="text-xs flex-shrink-0 text-text-muted transition-transform {prompt.expanded ? 'rotate-90' : ''}">&#9654;</span>
                    <span class="text-xs font-semibold uppercase tracking-wider text-text-muted">
                      {prompt.name.replace(/_/g, " ")}
                    </span>
                    {#if prompt.overridden}
                      <span class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-900/40 text-amber-400 border border-amber-700/50">this run</span>
                    {/if}
                    <!-- First line preview when collapsed -->
                    {#if !prompt.expanded}
                      <span class="text-xs text-text-muted truncate ml-auto max-w-48 font-mono">{prompt.rendered.split("\n")[0]}</span>
                    {/if}
                  </button>

                  <!-- Expanded content -->
                  {#if prompt.expanded}
                    <div class="border-t border-border">
                      {#if prompt.editing}
                        <!-- Edit mode -->
                        <div class="p-3 space-y-2">
                          <textarea
                            class="w-full px-3 py-2 bg-bg border border-border rounded text-xs text-text font-mono focus:outline-none focus:border-secondary resize-y"
                            rows="6"
                            value={prompt.editText}
                            oninput={(e) => updatePromptEditText(prompt.name, (e.currentTarget as HTMLTextAreaElement).value)}
                          ></textarea>
                          <p class="text-[10px] text-text-muted">
                            Variables: <code class="bg-bg px-1 rounded">{"{{home_team}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{away_team}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{date}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{sport}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{level}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{tournament}}"}</code>
                            <code class="bg-bg px-1 rounded">{"{{venue}}"}</code>
                          </p>
                          <div class="flex items-center gap-2">
                            <button
                              class="px-3 py-1.5 text-xs rounded-lg border border-amber-700 text-amber-400 hover:bg-amber-900/30 transition-colors"
                              title="Use this modified prompt for this game only. The template file is not changed."
                              onclick={() => applyPromptThisRun(prompt.name)}
                            >
                              This run only
                            </button>
                            <button
                              class="px-3 py-1.5 text-xs rounded-lg border border-primary text-text hover:bg-primary/20 transition-colors"
                              title="Save this change permanently to the template file for all future games."
                              onclick={() => savePromptPermanently(prompt.name)}
                            >
                              Save permanently
                            </button>
                            <button
                              class="px-3 py-1.5 text-xs text-text-muted hover:text-text transition-colors"
                              onclick={() => cancelEditingPrompt(prompt.name)}
                            >
                              Cancel
                            </button>
                          </div>
                        </div>
                      {:else}
                        <!-- View mode -->
                        <pre class="px-3 py-2 text-xs text-text whitespace-pre-wrap max-h-32 overflow-y-auto font-mono bg-bg/30">{prompt.rendered}</pre>
                        <div class="px-3 py-2 border-t border-border flex items-center gap-2">
                          <button
                            class="px-3 py-1 text-xs border border-border text-text-muted hover:text-text hover:border-secondary rounded transition-colors"
                            onclick={() => startEditingPrompt(prompt.name)}
                          >
                            Edit template
                          </button>
                          {#if prompt.overridden}
                            <span class="text-[10px] text-amber-400">Modified for this run</span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>

      <!-- STEP: Hook Execution Pipeline -->
      {:else if step === "hooks"}
        <div class="space-y-2">
          {#each hookSteps as hook (hook.id)}
            <div class="rounded-lg border overflow-hidden transition-colors {hook.status === 'running' ? 'border-blue-500' : hook.status === 'failed' ? 'border-red-500' : 'border-border'}">
              <!-- Hook header row -->
              <button
                class="w-full flex items-center gap-3 px-3 py-2.5 text-left hover:bg-bg/50 transition-colors"
                onclick={() => toggleHookExpanded(hook.id)}
              >
                <!-- Status indicator -->
                <span class="w-2.5 h-2.5 rounded-full flex-shrink-0 {statusColor(hook.status)}"></span>

                <!-- Label and description -->
                <div class="flex-1 min-w-0">
                  <span class="text-sm font-medium">{hook.label}</span>
                  <span class="text-xs text-text-muted ml-2">{hook.description}</span>
                </div>

                <!-- Status icon -->
                <span class="text-xs flex-shrink-0"
                  class:text-green-400={hook.status === "done"}
                  class:text-red-400={hook.status === "failed"}
                  class:text-blue-400={hook.status === "running"}
                  class:text-text-muted={hook.status === "pending"}
                >
                  {statusIcon(hook.status)}
                </span>

                <!-- Expand chevron -->
                <span class="text-text-muted text-xs flex-shrink-0 transition-transform"
                  class:rotate-90={hook.expanded}
                >
                  &#9654;
                </span>
              </button>

              <!-- Expanded content -->
              {#if hook.expanded}
                <div class="px-3 pb-3 border-t border-border bg-bg/30">
                  {#if hook.error}
                    <div class="mt-2 px-3 py-2 bg-red-900/20 border border-red-800/50 rounded text-xs text-red-300 font-mono">
                      {hook.error}
                    </div>
                  {/if}

                  {#if hook.result}
                    <pre class="mt-2 px-3 py-2 bg-bg rounded border border-border text-xs text-text whitespace-pre-wrap font-mono max-h-32 overflow-y-auto">{hook.result}</pre>
                  {/if}

                  {#if hook.imageUrl}
                    <div class="mt-2">
                      <img
                        src={hook.imageUrl}
                        alt="Generated thumbnail"
                        class="rounded border border-border max-w-full max-h-48 object-contain"
                      />
                    </div>
                  {/if}

                  {#if hook.status === "running"}
                    <div class="mt-2 flex items-center gap-2">
                      <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
                      <span class="text-xs text-text-muted">Running...</span>
                    </div>
                  {/if}

                  <!-- Actions -->
                  {#if hook.status === "done" || hook.status === "failed"}
                    <div class="mt-2 flex gap-2">
                      <button
                        class="px-2 py-1 text-xs rounded border border-border text-text-muted hover:text-text hover:border-secondary transition-colors"
                        onclick={() => rerunHook(hook.id)}
                      >
                        Rerun
                      </button>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}

          <!-- Summary when done -->
          {#if hooksDone}
            <div class="mt-4 p-3 bg-bg rounded-lg border border-border text-sm">
              <div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
                <span class="text-text-muted">Game</span>
                <span class="font-medium">{homeTeam} vs {awayTeam}</span>
                <span class="text-text-muted">Level</span>
                <span>{level} {sport}</span>
                <span class="text-text-muted">Date</span>
                <span>{date}</span>
                {#if tournament}
                  <span class="text-text-muted">Tournament</span>
                  <span>{tournament}</span>
                {/if}
                {#if venue}
                  <span class="text-text-muted">Venue</span>
                  <span>{venue}</span>
                {/if}
                <span class="text-text-muted">Directory</span>
                <span class="text-xs font-mono truncate">{createdDirPath}</span>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Footer navigation -->
    <div class="px-6 py-4 border-t border-border flex justify-between">
      {#if step === "hooks"}
        <div></div>
        <button
          class="px-5 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          disabled={!hooksDone}
          onclick={handleDone}
        >
          Open Game
        </button>
      {:else}
        <button
          class="px-4 py-2 text-text-muted hover:text-text text-sm transition-colors"
          onclick={step === "level" ? onclose : prevStep}
        >
          {step === "level" ? "Cancel" : "Back"}
        </button>
        <button
          class="px-5 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
          disabled={!canAdvanceFrom(step)}
          onclick={nextStep}
        >
          {step === "ai" ? "Create Game" : "Next"}
        </button>
      {/if}
    </div>
  </div>
</div>
