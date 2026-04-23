<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { getConfig, getDockSettings, setConfig, setDockSettings } from "$lib/stores/config.svelte";
  import { saveDockSettings, loadConfigFromPath, saveEventTypes } from "$lib/ipc/config";
  import { getEventTypes } from "$lib/ipc/games";
  import { setGames } from "$lib/stores/games";
  import { listGames } from "$lib/ipc/games";
  import type { DockSettings } from "$lib/types/dock";
  import type { EventTypeEntry } from "$lib/types/config";
  import LogViewer from "./LogViewer.svelte";
  import { getShowHelpTips, setShowHelpTips } from "$lib/stores/uiPrefs.svelte";
  import TeamsSettingsTab from "./TeamsSettingsTab.svelte";
  import TournamentsSettingsTab from "./TournamentsSettingsTab.svelte";
  import RenderingSettingsTab from "./RenderingSettingsTab.svelte";
  import ProfilesSettingsTab from "./ProfilesSettingsTab.svelte";
  import { settingsTeamTarget, settingsTournamentTarget } from "$lib/stores/navigation";
  import { useStore } from "$lib/stores/bridge.svelte";
  import { help } from "$lib/help";
  import HelpLink from "$lib/components/HelpLink.svelte";
  import { getCliStatus, refreshCliStatus } from "$lib/stores/cli.svelte";
  import { getProxyCacheStats, clearProxyCache } from "$lib/ipc/media";
  import type { ProxyCacheStats } from "$lib/ipc/media";

  let config = $derived(getConfig());
  let dockSettings = $derived(getDockSettings());
  let activeTab = $state<"dock" | "teams" | "tournaments" | "event-types" | "rendering" | "profiles" | "config" | "logs">("dock");

  const getTeamTarget = useStore(settingsTeamTarget);
  const getTournamentTarget = useStore(settingsTournamentTarget);

  // Auto-switch to teams/tournaments tab when navigating from sidebar
  $effect(() => {
    if (getTeamTarget()) {
      activeTab = "teams";
    }
  });

  $effect(() => {
    if (getTournamentTarget()) {
      activeTab = "tournaments";
    }
  });

  // Event types management state
  let eventTypes = $state<EventTypeEntry[]>([]);
  let newEventType = $state("");
  let eventTypesSaving = $state(false);
  let eventTypesMessage = $state("");
  let dragIndex = $state<number | null>(null);

  async function loadEventTypes() {
    try {
      eventTypes = await getEventTypes();
    } catch {
      eventTypes = [];
    }
  }

  async function addEventType() {
    const trimmed = newEventType.trim().toLowerCase();
    if (!trimmed) return;
    if (eventTypes.some(et => et.name === trimmed)) {
      eventTypesMessage = `'${trimmed}' already exists.`;
      return;
    }
    eventTypes = [...eventTypes, { name: trimmed, team_specific: false }];
    newEventType = "";
    eventTypesMessage = "";
    await persistEventTypes();
  }

  async function removeEventType(index: number) {
    eventTypes = eventTypes.filter((_, i) => i !== index);
    await persistEventTypes();
  }

  async function toggleTeamSpecific(index: number) {
    eventTypes = eventTypes.map((et, i) =>
      i === index ? { ...et, team_specific: !et.team_specific } : et,
    );
    await persistEventTypes();
  }

  async function persistEventTypes() {
    eventTypesSaving = true;
    try {
      await saveEventTypes(eventTypes);
      if (dockSettings.reeln_config_path) {
        const loaded = await loadConfigFromPath(dockSettings.reeln_config_path);
        setConfig(loaded.config);
      }
      eventTypesMessage = "Saved.";
    } catch (e) {
      eventTypesMessage = `Error: ${e}`;
    }
    eventTypesSaving = false;
  }

  function handleDragStart(index: number) {
    dragIndex = index;
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === index) return;
    const updated = [...eventTypes];
    const [moved] = updated.splice(dragIndex, 1);
    updated.splice(index, 0, moved);
    eventTypes = updated;
    dragIndex = index;
  }

  async function handleDragEnd() {
    if (dragIndex !== null) {
      dragIndex = null;
      await persistEventTypes();
    }
  }
  let saving = $state(false);
  let message = $state("");
  let showLogos = $state(true);

  // Proxy cache
  let cacheStats = $state<ProxyCacheStats | null>(null);
  let cacheClearing = $state(false);

  async function loadCacheStats() {
    try {
      cacheStats = await getProxyCacheStats();
    } catch {
      cacheStats = null;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  async function handleClearCache() {
    cacheClearing = true;
    try {
      await clearProxyCache();
      await loadCacheStats();
    } finally {
      cacheClearing = false;
    }
  }

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
      class:bg-primary={activeTab === "teams"}
      class:text-text-muted={activeTab !== "teams"}
      class:hover:text-text={activeTab !== "teams"}
      onclick={() => (activeTab = "teams")}
    >
      Teams
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "tournaments"}
      class:text-text-muted={activeTab !== "tournaments"}
      class:hover:text-text={activeTab !== "tournaments"}
      onclick={() => (activeTab = "tournaments")}
    >
      Tournaments
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "event-types"}
      class:text-text-muted={activeTab !== "event-types"}
      class:hover:text-text={activeTab !== "event-types"}
      onclick={() => { activeTab = "event-types"; loadEventTypes(); }}
    >
      Event Types
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "rendering"}
      class:text-text-muted={activeTab !== "rendering"}
      class:hover:text-text={activeTab !== "rendering"}
      onclick={() => (activeTab = "rendering")}
    >
      Rendering
    </button>
    <button
      class="px-3 py-1.5 text-sm rounded transition-colors"
      class:bg-primary={activeTab === "profiles"}
      class:text-text-muted={activeTab !== "profiles"}
      class:hover:text-text={activeTab !== "profiles"}
      onclick={() => (activeTab = "profiles")}
    >
      Profiles
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

  {#if activeTab === "teams"}
    <TeamsSettingsTab />

  {:else if activeTab === "tournaments"}
    <TournamentsSettingsTab />

  {:else if activeTab === "rendering"}
    <RenderingSettingsTab />

  {:else if activeTab === "profiles"}
    <ProfilesSettingsTab />

  {:else if activeTab === "dock"}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
      <div>
        <label class="block text-sm text-text-muted mb-1" for="config-path">Config File Path <HelpLink text={help["config.file_locations"].text} url={help["config.file_locations"].url} /></label>
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
        <label class="flex items-center gap-3 cursor-pointer mt-3" for="show-help-tips">
          <input
            id="show-help-tips"
            type="checkbox"
            checked={getShowHelpTips()}
            onchange={(e) => setShowHelpTips((e.target as HTMLInputElement).checked)}
            class="rounded"
          />
          <div>
            <span class="text-sm text-text">Show help tooltips</span>
            <p class="text-xs text-text-muted">Display "?" help icons next to settings with links to documentation.</p>
          </div>
        </label>
      </div>

      <!-- CLI Status -->
      {#if true}
      {@const cliStatus = getCliStatus()}
      <div class="border-t border-border pt-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">reeln CLI</h3>
        <div class="space-y-2">
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full {cliStatus.available ? 'bg-green-500' : 'bg-red-500'}"></span>
            <span class="text-sm">{cliStatus.available ? `reeln ${cliStatus.version}` : "Not found"}</span>
            <button
              class="ml-auto px-2 py-0.5 text-xs text-text-muted hover:text-text bg-bg border border-border rounded transition-colors"
              onclick={() => refreshCliStatus()}
            >Detect</button>
          </div>
          {#if cliStatus.available}
            <div class="text-xs text-text-muted truncate" title={cliStatus.path ?? ""}>
              {cliStatus.path}
            </div>
            {#if cliStatus.plugins.length > 0}
              <div class="flex flex-wrap gap-1.5 mt-1">
                {#each cliStatus.plugins as plugin}
                  <span class="px-1.5 py-0.5 bg-bg rounded text-[10px] text-text-muted">{plugin.name} {plugin.version}</span>
                {/each}
              </div>
            {:else}
              <p class="text-xs text-text-muted">No plugins installed.</p>
            {/if}
          {:else}
            <p class="text-xs text-text-muted">
              Install with: <code class="bg-bg px-1 py-0.5 rounded">uv pip install reeln</code>
            </p>
            <p class="text-xs text-text-muted">Plugin features (overlays, smart zoom, uploads) require the CLI.</p>
          {/if}
        </div>
      </div>
      {/if}

      <!-- Preview Cache -->
      {#await loadCacheStats() then}
      <div class="border-t border-border pt-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-3">Preview Cache</h3>
        <p class="text-xs text-text-muted mb-2">
          Video proxies are generated for MKV and other non-web formats to enable in-app preview.
          Old proxies are automatically cleaned up after 7 days.
        </p>
        {#if cacheStats}
          <div class="flex items-center gap-4">
            <span class="text-sm">
              {cacheStats.file_count} {cacheStats.file_count === 1 ? "file" : "files"}
              <span class="text-text-muted">({formatBytes(cacheStats.total_bytes)})</span>
            </span>
            {#if cacheStats.file_count > 0}
              <button
                class="px-2.5 py-1 text-xs font-medium border border-border text-text-muted hover:text-accent hover:border-accent/50 rounded transition-colors disabled:opacity-50"
                disabled={cacheClearing}
                onclick={handleClearCache}
              >
                {cacheClearing ? "Clearing..." : "Clear Cache"}
              </button>
            {/if}
          </div>
        {:else}
          <span class="text-sm text-text-muted">No cache data</span>
        {/if}
      </div>
      {/await}
    </div>

  {:else if activeTab === "event-types"}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
      {#if config}
        <div class="text-sm text-text-muted">
          Sport: <span class="text-text font-medium">{config.sport}</span>
        </div>
      {/if}

      {#if eventTypesMessage}
        <p class="text-sm text-text-muted">{eventTypesMessage}</p>
      {/if}

      <!-- Add new type -->
      <div class="flex gap-2">
        <input
          type="text"
          bind:value={newEventType}
          placeholder="Add event type..."
          class="flex-1 px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
          onkeydown={(e) => { if (e.key === "Enter") addEventType(); }}
        />
        <button
          class="px-3 py-1.5 bg-primary hover:bg-primary-light text-text rounded text-sm transition-colors disabled:opacity-50"
          onclick={addEventType}
          disabled={eventTypesSaving || !newEventType.trim()}
        >
          Add
        </button>
      </div>

      <!-- Event types list -->
      {#if eventTypes.length === 0}
        <p class="text-text-muted text-sm">No event types configured. Add types above or they'll be inferred from the sport defaults.</p>
      {:else}
        <div class="space-y-1">
          {#each eventTypes as et, index (et.name)}
            <div
              class="flex items-center gap-2 px-3 py-1.5 bg-bg rounded border transition-colors"
              class:border-secondary={dragIndex === index}
              class:border-border={dragIndex !== index}
              draggable="true"
              ondragstart={() => handleDragStart(index)}
              ondragover={(e) => handleDragOver(e, index)}
              ondragend={handleDragEnd}
              role="listitem"
            >
              <span class="text-text-muted cursor-grab select-none" title="Drag to reorder">&#x2630;</span>
              <span class="flex-1 text-sm text-text">{et.name}</span>
              <label class="flex items-center gap-1 text-xs text-text-muted cursor-pointer" title="Show Home/Away variants">
                <input
                  type="checkbox"
                  checked={et.team_specific}
                  onchange={() => toggleTeamSpecific(index)}
                  class="accent-secondary"
                />
                Team
              </label>
              <button
                class="text-text-muted hover:text-red-400 text-sm transition-colors"
                onclick={() => removeEventType(index)}
                title="Remove"
              >
                &times;
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else if activeTab === "config" && config}
    <div class="bg-surface rounded-lg border border-border p-4 space-y-4 text-sm">
      <div class="flex items-center gap-2 mb-2">
        <span class="text-xs font-semibold uppercase tracking-wider text-text-muted">reeln Config</span>
        <HelpLink text={help["config.doctor"].text} url={help["config.doctor"].url} />
      </div>
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
        <span>{(config.plugins?.enabled ?? []).join(", ") || "none"}</span>
      </div>

      {#if Object.keys(config.render_profiles ?? {}).length > 0}
        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted pt-2">Render Profiles</h3>
        <div>
          {#each Object.keys(config.render_profiles ?? {}) as name}
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
