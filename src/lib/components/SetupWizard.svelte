<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { open as openUrl } from "@tauri-apps/plugin-shell";
  import {
    loadConfigFromPath,
    saveDockSettings,
    getConfigPath,
    listAvailableSportsInit,
    createInitialConfig,
  } from "$lib/ipc/config";
  import type { SportInfoInit } from "$lib/ipc/config";
  import { help } from "$lib/help";
  import HelpLink from "$lib/components/HelpLink.svelte";
  import type { AppConfig } from "$lib/types/config";
  import type { DockSettings } from "$lib/types/dock";

  interface Props {
    onDone: (config: AppConfig, settings: DockSettings) => void;
  }

  let { onDone }: Props = $props();

  type Step = "welcome" | "sport" | "source" | "output" | "summary" | "done" | "locate" | "loaded";

  let step = $state<Step>("welcome");
  let flow = $state<"create" | "locate">("create");
  let config = $state<AppConfig | null>(null);
  let error = $state("");
  let configPath = $state("");
  let defaultPath = $state("");
  let saving = $state(false);

  // Create flow state
  let selectedSport = $state<SportInfoInit | null>(null);
  let sourceDir = $state("");
  let outputDir = $state("");
  let createDirs = $state(true);
  let sports = $state<SportInfoInit[]>([]);
  let sportsLoading = $state(false);
  let createdSettings = $state<DockSettings | null>(null);

  $effect(() => {
    (async () => {
      defaultPath = await getConfigPath();
    })();
  });

  async function loadSports() {
    if (sports.length > 0) return;
    sportsLoading = true;
    try {
      sports = await listAvailableSportsInit();
    } catch (e) {
      error = String(e);
    } finally {
      sportsLoading = false;
    }
  }

  function startCreateFlow() {
    flow = "create";
    step = "sport";
    loadSports();
  }

  function startLocateFlow() {
    flow = "locate";
    step = "locate";
  }

  function capitalize(s: string): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function formatSegmentInfo(sport: SportInfoInit): string {
    const parts = [`${sport.segment_count} ${sport.segment_name}s`];
    if (sport.duration_minutes) {
      parts.push(`${sport.duration_minutes} min`);
    }
    return parts.join(", ");
  }

  function formatEventTypes(sport: SportInfoInit): string {
    if (sport.default_event_types.length === 0) return "no default events";
    return sport.default_event_types.map((e) => e.name).join(", ");
  }

  // ── Locate flow handlers ─────────────────────────────────────────

  async function browseForConfig() {
    error = "";
    const result = await openDialog({
      title: "Locate reeln config",
      filters: [{ name: "JSON", extensions: ["json"] }],
      directory: false,
      multiple: false,
    });
    if (!result) return;
    await loadFromPath(result as string);
  }

  async function browseForDirectory() {
    error = "";
    const result = await openDialog({
      title: "Select directory containing reeln config",
      directory: true,
    });
    if (!result) return;
    await loadFromPath(result as string);
  }

  async function loadFromPath(path: string) {
    error = "";
    try {
      const result = await loadConfigFromPath(path);
      config = result.config;
      configPath = result.path;
      step = "loaded";
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmAndSave() {
    if (!config) return;
    saving = true;
    error = "";
    try {
      const settings: DockSettings = {
        reeln_config_path: configPath,
        plugin_profiles: {},
        display: {
          show_logos: true,
          sections_expanded: { games: true, teams: true, tournaments: true },
        },
      };
      await saveDockSettings(settings);
      onDone(config, settings);
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }

  // ── Create flow handlers ──────────────────────────────────────────

  async function browseSourceDir() {
    error = "";
    const result = await openDialog({
      title: "Select replay source directory",
      directory: true,
    });
    if (!result) return;
    sourceDir = result as string;
  }

  async function browseOutputDir() {
    error = "";
    const result = await openDialog({
      title: "Select game output directory",
      directory: true,
    });
    if (!result) return;
    outputDir = result as string;
  }

  async function handleCreateConfig() {
    if (!selectedSport) return;
    saving = true;
    error = "";
    try {
      const result = await createInitialConfig({
        sport: selectedSport.name,
        sourceDir,
        outputDir,
        createDirs,
      });
      config = result.config;
      configPath = result.path;
      // Build the settings object for onDone
      createdSettings = {
        reeln_config_path: result.path,
        plugin_profiles: {},
        display: {
          show_logos: true,
          sections_expanded: { games: true, teams: true, tournaments: true },
        },
      };
      step = "done";
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleGetStarted() {
    if (!config || !createdSettings) return;
    onDone(config, createdSettings);
  }

  // ── Step dots ─────────────────────────────────────────────────────

  const createSteps: Step[] = ["welcome", "sport", "source", "output", "summary", "done"];
  const locateSteps: Step[] = ["welcome", "locate", "loaded"];

  let activeDots = $derived(flow === "create" ? createSteps : locateSteps);
  let dotIndex = $derived(activeDots.indexOf(step));
</script>

<div class="fixed inset-0 flex items-center justify-center bg-bg">
  <div class="w-full max-w-lg p-8 bg-surface rounded-xl border border-border">

    {#if step === "welcome"}
      <div class="flex flex-col items-center gap-6">
        <img src="/logo.png" alt="reeln" class="w-24 h-24" />
        <h2 class="text-2xl font-bold">Welcome to reeln dock</h2>
        <p class="text-text-muted text-center">
          Let's get you set up. We'll choose your sport, set up directories, and create a config file.
        </p>
        <button
          class="w-full px-6 py-3 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors text-lg font-medium"
          onclick={startCreateFlow}
        >
          Set up reeln
        </button>
        <button
          class="w-full px-4 py-2 bg-surface-hover border border-border hover:bg-border text-text-muted rounded-lg transition-colors text-sm"
          onclick={startLocateFlow}
        >
          I have an existing config
        </button>
        {#if help["docs.quickstart"]?.url}
          <button
            class="text-sm text-secondary hover:underline transition-colors"
            onclick={() => openUrl(help["docs.quickstart"].url!)}
          >Read the Quick Start Guide</button>
        {/if}
      </div>

    {:else if step === "sport"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Choose your sport</h2>
        <p class="text-text-muted text-sm">
          Select the sport you'll be recording. This determines segments, event types, and defaults.
        </p>

        {#if sportsLoading}
          <div class="flex justify-center py-8">
            <span class="text-text-muted">Loading sports...</span>
          </div>
        {:else}
          <div class="grid grid-cols-2 gap-3 max-h-72 overflow-y-auto pr-1">
            {#each sports as sport}
              <button
                class="p-3 rounded-lg border text-left transition-colors {selectedSport?.name === sport.name ? 'border-primary bg-primary-light' : 'border-border bg-surface hover:border-secondary'}"
                onclick={() => (selectedSport = sport)}
              >
                <div class="font-medium text-text">{capitalize(sport.name)}</div>
                <div class="text-xs text-text-muted mt-1">{formatSegmentInfo(sport)}</div>
                <div class="text-xs text-text-muted mt-0.5 truncate">{formatEventTypes(sport)}</div>
              </button>
            {/each}
          </div>
        {/if}

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <div class="flex justify-between mt-2">
          <button
            class="px-4 py-2 text-text-muted hover:text-text transition-colors"
            onclick={() => { step = "welcome"; error = ""; }}
          >Back</button>
          <button
            class="px-6 py-2 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50"
            disabled={!selectedSport}
            onclick={() => { step = "source"; error = ""; }}
          >Next</button>
        </div>
      </div>

    {:else if step === "source"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Where are your replays?</h2>
        <p class="text-text-muted text-sm">
          This is the directory where your streaming software (OBS, etc.) saves replay files.
        </p>

        <div class="flex gap-2">
          <input
            type="text"
            class="flex-1 px-3 py-2 bg-bg border border-border rounded-lg text-text text-sm focus:outline-none focus:border-secondary"
            placeholder="~/Videos/Replays"
            bind:value={sourceDir}
          />
          <button
            class="px-4 py-2 bg-surface-hover border border-border hover:bg-border text-text rounded-lg transition-colors text-sm whitespace-nowrap"
            onclick={browseSourceDir}
          >Browse...</button>
        </div>

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <div class="flex justify-between mt-2">
          <button
            class="px-4 py-2 text-text-muted hover:text-text transition-colors"
            onclick={() => { step = "sport"; error = ""; }}
          >Back</button>
          <button
            class="px-6 py-2 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50"
            disabled={!sourceDir.trim()}
            onclick={() => { step = "output"; error = ""; }}
          >Next</button>
        </div>
      </div>

    {:else if step === "output"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Where should games be saved?</h2>
        <p class="text-text-muted text-sm">
          Each game gets its own folder here with clips, renders, and metadata.
        </p>

        <div class="flex gap-2">
          <input
            type="text"
            class="flex-1 px-3 py-2 bg-bg border border-border rounded-lg text-text text-sm focus:outline-none focus:border-secondary"
            placeholder="~/Videos/Games"
            bind:value={outputDir}
          />
          <button
            class="px-4 py-2 bg-surface-hover border border-border hover:bg-border text-text rounded-lg transition-colors text-sm whitespace-nowrap"
            onclick={browseOutputDir}
          >Browse...</button>
        </div>

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <div class="flex justify-between mt-2">
          <button
            class="px-4 py-2 text-text-muted hover:text-text transition-colors"
            onclick={() => { step = "source"; error = ""; }}
          >Back</button>
          <button
            class="px-6 py-2 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50"
            disabled={!outputDir.trim()}
            onclick={() => { step = "summary"; error = ""; }}
          >Next</button>
        </div>
      </div>

    {:else if step === "summary"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Review your setup</h2>
        <p class="text-text-muted text-sm">
          Everything look right? We'll create your config and you'll be ready to go.
        </p>

        <div class="bg-bg rounded-lg p-4 border border-border text-sm space-y-2">
          <div>
            <span class="text-text-muted">Config path:</span>
            <code class="text-secondary break-all ml-1">{defaultPath}</code>
          </div>
          {#if selectedSport}
            <div>
              <span class="text-text-muted">Sport:</span>
              <span class="ml-1">{capitalize(selectedSport.name)}</span>
              <span class="text-text-muted ml-1">({formatSegmentInfo(selectedSport)})</span>
            </div>
          {/if}
          <div>
            <span class="text-text-muted">Source directory:</span>
            <code class="text-secondary break-all ml-1">{sourceDir}</code>
          </div>
          <div>
            <span class="text-text-muted">Output directory:</span>
            <code class="text-secondary break-all ml-1">{outputDir}</code>
          </div>
          {#if selectedSport && selectedSport.default_event_types.length > 0}
            <div>
              <span class="text-text-muted">Event types:</span>
              <span class="ml-1">{formatEventTypes(selectedSport)}</span>
            </div>
          {/if}
        </div>

        <label class="flex items-center gap-2 text-sm text-text-muted cursor-pointer">
          <input type="checkbox" bind:checked={createDirs} class="accent-primary" />
          Create directories if they don't exist
        </label>

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <button
          class="w-full px-6 py-3 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50 font-medium"
          onclick={handleCreateConfig}
          disabled={saving}
        >
          {saving ? "Creating..." : "Create Config"}
        </button>

        <div class="flex justify-start">
          <button
            class="px-4 py-2 text-text-muted hover:text-text transition-colors"
            onclick={() => { step = "output"; error = ""; }}
          >Back</button>
        </div>
      </div>

    {:else if step === "done"}
      <div class="flex flex-col items-center gap-6">
        <div class="w-16 h-16 rounded-full bg-primary/20 flex items-center justify-center">
          <svg class="w-8 h-8 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        </div>
        <h2 class="text-2xl font-bold">You're all set!</h2>
        <p class="text-text-muted text-center text-sm">
          Your config has been created at<br />
          <code class="text-secondary break-all">{configPath}</code>
        </p>
        <p class="text-text-muted text-center text-sm">
          Next, start a game, process your replay segments, and render your first highlights.
        </p>
        <button
          class="w-full px-6 py-3 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors text-lg font-medium"
          onclick={handleGetStarted}
        >
          Get Started
        </button>
      </div>

    {:else if step === "locate"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Locate your config <HelpLink text={help["setup.config_file"].text} url={help["setup.config_file"].url} /></h2>
        <p class="text-text-muted text-sm">
          Browse for your <code class="text-secondary">reeln.json</code> file, or select a directory that contains one.
        </p>

        {#if defaultPath}
          <p class="text-xs text-text-muted">
            Default location: <code class="text-secondary break-all">{defaultPath}</code>
          </p>
        {/if}

        <div class="flex flex-col gap-2">
          <button
            class="w-full px-4 py-3 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors text-left"
            onclick={browseForConfig}
          >
            Browse for config file...
          </button>
          <button
            class="w-full px-4 py-3 bg-surface-hover border border-border hover:bg-border text-text rounded-lg transition-colors text-left"
            onclick={browseForDirectory}
          >
            Browse for directory...
          </button>
        </div>

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <div class="flex justify-start mt-2">
          <button class="px-4 py-2 text-text-muted hover:text-text transition-colors" onclick={() => { step = "welcome"; error = ""; }}>Back</button>
        </div>
      </div>

    {:else if step === "loaded"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Config loaded</h2>
        <p class="text-text-muted text-sm">
          Loaded from <code class="text-secondary break-all">{configPath}</code>
        </p>

        {#if config}
          <div class="bg-bg rounded-lg p-4 border border-border text-sm space-y-2">
            <div><span class="text-text-muted">Sport:</span> {config.sport}</div>
            <div><span class="text-text-muted">Output:</span> {config.paths.output_dir ?? "(not set)"}</div>
            <div><span class="text-text-muted">Source:</span> {config.paths.source_dir ?? "(not set)"}</div>
            <div><span class="text-text-muted">Codec:</span> {config.video.codec}</div>
            <div><span class="text-text-muted">Enabled plugins:</span> {(config.plugins?.enabled ?? []).join(", ") || "none"}</div>
            <div><span class="text-text-muted">Render profiles:</span> {Object.keys(config.render_profiles ?? {}).join(", ") || "none"}</div>
          </div>
        {/if}

        {#if error}
          <p class="text-accent text-sm">{error}</p>
        {/if}

        <button
          class="w-full px-6 py-2 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50"
          onclick={confirmAndSave}
          disabled={saving}
        >
          {saving ? "Saving..." : "Continue with this config"}
        </button>

        <button
          class="text-text-muted hover:text-text text-sm transition-colors"
          onclick={() => (step = "locate")}
        >
          Pick a different file
        </button>
        {#if help["setup.config_file"]?.url}
          <button
            class="text-xs text-secondary hover:underline transition-colors mt-1"
            onclick={() => openUrl(help["setup.config_file"].url!)}
          >Learn more about configuration</button>
        {/if}
      </div>
    {/if}

    <div class="flex justify-center gap-2 mt-6">
      {#each activeDots as _, i}
        <div
          class="w-2 h-2 rounded-full transition-colors"
          class:bg-secondary={i === dotIndex}
          class:bg-border={i !== dotIndex}
        ></div>
      {/each}
    </div>
  </div>
</div>
