<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { getConfig, getDockSettings, setConfig, setDockSettings } from "$lib/stores/config.svelte";
  import { saveDockSettings, loadConfigFromPath } from "$lib/ipc/config";
  import { setGames } from "$lib/stores/games";
  import { listGames } from "$lib/ipc/games";
  import type { DockSettings } from "$lib/types/dock";
  import LogViewer from "./LogViewer.svelte";

  let config = $derived(getConfig());
  let dockSettings = $derived(getDockSettings());
  let activeTab = $state<"dock" | "config" | "logs">("dock");
  let saving = $state(false);
  let message = $state("");
  let showLogos = $state(true);

  $effect(() => {
    showLogos = dockSettings.display?.show_logos ?? true;
  });

  async function changeConfigPath() {
    const result = await open({
      title: "Locate reeln config",
      filters: [{ name: "JSON", extensions: ["json"] }],
      directory: false,
      multiple: false,
    });
    if (!result) return;

    saving = true;
    message = "";
    try {
      const loaded = await loadConfigFromPath(result as string);
      const settings: DockSettings = { reeln_config_path: loaded.path, plugin_profiles: {}, display: { show_logos: true, sections_expanded: { games: true, teams: true, tournaments: true } } };
      await saveDockSettings(settings);
      setDockSettings(settings);
      setConfig(loaded.config);

      if (loaded.config.paths.output_dir) {
        const loadedGames = await listGames(loaded.config.paths.output_dir);
        setGames(loadedGames);
      }

      message = "Config updated.";
    } catch (e) {
      message = `Error: ${e}`;
    }
    saving = false;
  }
</script>

<div>
  <h2 class="text-lg font-bold mb-4">Settings</h2>

  <div class="flex gap-2 mb-4">
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "dock"}
      class:text-text-muted={activeTab !== "dock"}
      class:hover:text-text={activeTab !== "dock"}
      onclick={() => (activeTab = "dock")}
    >
      Dock
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "config"}
      class:text-text-muted={activeTab !== "config"}
      class:hover:text-text={activeTab !== "config"}
      onclick={() => (activeTab = "config")}
    >
      reeln Config (read-only)
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "logs"}
      class:text-text-muted={activeTab !== "logs"}
      class:hover:text-text={activeTab !== "logs"}
      onclick={() => (activeTab = "logs")}
    >
      Logs
    </button>
  </div>

  {#if message}
    <p class="text-sm text-text-muted mb-4">{message}</p>
  {/if}

  {#if activeTab === "dock"}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
      <div>
        <label class="block text-sm text-text-muted mb-1" for="config-path">Config File Path</label>
        <div class="flex gap-2">
          <input
            id="config-path"
            type="text"
            value={dockSettings.reeln_config_path ?? ""}
            readonly
            class="flex-1 px-3 py-1.5 bg-bg border border-border rounded text-sm text-text"
          />
          <button
            class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded text-sm transition-colors disabled:opacity-50"
            onclick={changeConfigPath}
            disabled={saving}
          >
            Change
          </button>
        </div>
      </div>

      <div class="border-t border-border pt-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">Display</h3>
        <label class="flex items-center gap-3 cursor-pointer" for="show-logos">
          <input
            id="show-logos"
            type="checkbox"
            bind:checked={showLogos}
            onchange={async () => {
              const updated = { ...dockSettings, display: { ...dockSettings.display, show_logos: showLogos } };
              await saveDockSettings(updated);
              setDockSettings(updated);
            }}
            class="rounded"
          />
          <div>
            <span class="text-sm text-text">Show team logos</span>
            <p class="text-xs text-text-muted">Display team logos on game tiles in sidebar and tournament views.</p>
          </div>
        </label>
      </div>
    </div>

  {:else if activeTab === "config" && config}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-4 text-sm">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <span class="text-text-muted block">Sport</span>
          <span>{config.sport}</span>
        </div>
        <div>
          <span class="text-text-muted block">Config Version</span>
          <span>{config.config_version}</span>
        </div>
      </div>

      <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted pt-2">Paths</h3>
      <div class="grid grid-cols-2 gap-4">
        <div>
          <span class="text-text-muted block">Output Directory</span>
          <span class="break-all">{config.paths.output_dir ?? "(not set)"}</span>
        </div>
        <div>
          <span class="text-text-muted block">Source Directory</span>
          <span class="break-all">{config.paths.source_dir ?? "(not set)"}</span>
        </div>
        <div>
          <span class="text-text-muted block">Source Glob</span>
          <span>{config.paths.source_glob}</span>
        </div>
        <div>
          <span class="text-text-muted block">Temp Directory</span>
          <span class="break-all">{config.paths.temp_dir ?? "(default)"}</span>
        </div>
      </div>

      <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted pt-2">Video</h3>
      <div class="grid grid-cols-3 gap-4">
        <div>
          <span class="text-text-muted block">Codec</span>
          <span>{config.video.codec}</span>
        </div>
        <div>
          <span class="text-text-muted block">Preset</span>
          <span>{config.video.preset}</span>
        </div>
        <div>
          <span class="text-text-muted block">CRF</span>
          <span>{config.video.crf}</span>
        </div>
        <div>
          <span class="text-text-muted block">Audio Codec</span>
          <span>{config.video.audio_codec}</span>
        </div>
        <div>
          <span class="text-text-muted block">Audio Bitrate</span>
          <span>{config.video.audio_bitrate}</span>
        </div>
      </div>

      <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted pt-2">Plugins</h3>
      <div>
        <span class="text-text-muted block">Enabled</span>
        <span>{config.plugins.enabled.join(", ") || "none"}</span>
      </div>

      {#if Object.keys(config.render_profiles).length > 0}
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted pt-2">Render Profiles</h3>
        <div>
          {#each Object.keys(config.render_profiles) as name}
            <span class="inline-block px-2 py-0.5 rounded bg-bg text-text-muted text-xs mr-1 mb-1">{name}</span>
          {/each}
        </div>
      {/if}
    </div>

  {:else if activeTab === "logs"}
    <div class="h-[calc(100vh-200px)]">
      <LogViewer />
    </div>

  {:else}
    <p class="text-text-muted text-sm">No config loaded.</p>
  {/if}
</div>
