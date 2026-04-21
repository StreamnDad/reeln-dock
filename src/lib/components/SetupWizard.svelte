<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { loadConfigFromPath, saveDockSettings, getConfigPath } from "$lib/ipc/config";
  import { listSports } from "$lib/ipc/games";
  import type { AppConfig } from "$lib/types/config";
  import type { DockSettings } from "$lib/types/dock";
  import type { SportAlias } from "$lib/types/sport";

  interface Props {
    onDone: (config: AppConfig, settings: DockSettings) => void;
  }

  let { onDone }: Props = $props();

  type Step = "welcome" | "locate" | "loaded";

  let step = $state<Step>("welcome");
  let config = $state<AppConfig | null>(null);
  let sports = $state<SportAlias[]>([]);
  let error = $state("");
  let configPath = $state("");
  let defaultPath = $state("");
  let saving = $state(false);

  $effect(() => {
    (async () => {
      sports = await listSports();
      defaultPath = await getConfigPath();
    })();
  });

  async function browseForConfig() {
    error = "";
    const result = await open({
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
    const result = await open({
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
      const settings: DockSettings = { reeln_config_path: configPath, plugin_profiles: {}, display: { show_logos: true, sections_expanded: { games: true, teams: true, tournaments: true } } };
      await saveDockSettings(settings);
      onDone(config, settings);
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }

  const stepDots: Step[] = ["welcome", "locate", "loaded"];
  let dotIndex = $derived(stepDots.indexOf(step));
</script>

<div class="fixed inset-0 flex items-center justify-center bg-bg">
  <div class="w-full max-w-lg p-8 bg-surface rounded-xl border border-border">

    {#if step === "welcome"}
      <div class="flex flex-col items-center gap-6">
        <img src="/logo.png" alt="reeln" class="w-24 h-24" />
        <h2 class="text-2xl font-bold">Welcome to reeln dock</h2>
        <p class="text-text-muted text-center">
          Point reeln dock at your existing config file to get started.
        </p>
        <button
          class="w-full px-6 py-3 bg-primary hover:bg-primary-light text-text rounded-lg transition-colors"
          onclick={() => (step = "locate")}
        >
          Locate config file
        </button>
      </div>

    {:else if step === "locate"}
      <div class="flex flex-col gap-4">
        <h2 class="text-xl font-bold">Locate your config</h2>
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
          <button class="px-4 py-2 text-text-muted hover:text-text transition-colors" onclick={() => (step = "welcome")}>Back</button>
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
      </div>
    {/if}

    <div class="flex justify-center gap-2 mt-6">
      {#each stepDots as _, i}
        <div
          class="w-2 h-2 rounded-full transition-colors"
          class:bg-secondary={i === dotIndex}
          class:bg-border={i !== dotIndex}
        ></div>
      {/each}
    </div>
  </div>
</div>
