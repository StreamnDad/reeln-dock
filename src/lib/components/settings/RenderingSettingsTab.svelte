<script lang="ts">
  import { getDockSettings, getConfig, setDockSettings } from "$lib/stores/config.svelte";
  import { saveDockSettings } from "$lib/ipc/config";
  import { getEventTypes } from "$lib/ipc/games";
  import { listRenderProfiles } from "$lib/ipc/render";
  import { listConfigProfiles } from "$lib/ipc/plugins";
  import type { RenderProfile, EventTypeEntry } from "$lib/types/config";
  import type { ConfigProfile } from "$lib/types/plugin";
  import type { RenderingDefaults } from "$lib/types/dock";

  let dockSettings = $derived(getDockSettings());
  let config = $derived(getConfig());

  let profiles = $state<RenderProfile[]>([]);
  let pluginProfiles = $state<ConfigProfile[]>([]);
  let eventTypes = $state<EventTypeEntry[]>([]);
  let saving = $state(false);
  let message = $state("");

  // Local editable copy of rendering defaults
  let defaultProfile = $state<string>("");
  let defaultPluginProfile = $state<string>("");
  let concatByDefault = $state(false);
  let mappings = $state<Record<string, string[]>>({});
  let newMappingType = $state("");

  // Override defaults
  let cropMode = $state<string>("");
  let scale = $state<number>(1.0);
  let speed = $state<number>(1.0);
  let smartZoom = $state(false);

  // Event types available to add as mappings (not already mapped)
  let unmappedEventTypes = $derived(
    ["default", ...eventTypes.map((e) => e.name)]
      .filter((v, i, a) => a.indexOf(v) === i)
      .filter((name) => !(name in mappings)),
  );

  // Load profiles + event types + plugin profiles
  $effect(() => {
    listRenderProfiles()
      .then((p) => { profiles = p; })
      .catch(() => {});
    getEventTypes()
      .then((types) => { eventTypes = types; })
      .catch(() => {});
    listConfigProfiles()
      .then((p) => { pluginProfiles = p; })
      .catch(() => {});
  });

  // Initialize local state from dock settings
  $effect(() => {
    const rendering = dockSettings.rendering;
    defaultProfile = rendering?.default_profile ?? "";
    defaultPluginProfile = rendering?.default_plugin_profile ?? "";
    concatByDefault = rendering?.concat_by_default ?? false;
    // Merge dock overrides with config defaults
    const configMappings = config?.iterations?.mappings ?? {};
    const dockMappings = rendering?.iteration_mappings ?? {};
    mappings = { ...configMappings, ...dockMappings };
    // Override defaults
    const ovr = rendering?.overrides;
    cropMode = ovr?.crop_mode ?? "";
    scale = ovr?.scale ?? 1.0;
    speed = ovr?.speed ?? 1.0;
    smartZoom = ovr?.smart ?? false;
  });

  function profileLabel(name: string): string {
    const p = profiles.find((rp) => rp.name === name);
    if (p?.width && p?.height) return `${name} (${p.width}x${p.height})`;
    return name;
  }

  function addProfileToMapping(eventType: string, profileName: string) {
    if (!profileName) return;
    const current = mappings[eventType] ?? [];
    if (current.includes(profileName)) return;
    mappings = { ...mappings, [eventType]: [...current, profileName] };
  }

  function removeProfileFromMapping(eventType: string, index: number) {
    const current = mappings[eventType] ?? [];
    mappings = {
      ...mappings,
      [eventType]: current.filter((_, i) => i !== index),
    };
  }

  function addEventTypeMapping() {
    if (!newMappingType || newMappingType in mappings) return;
    mappings = { ...mappings, [newMappingType]: [] };
    newMappingType = "";
  }

  let addProfileSelections = $state<Record<string, string>>({});

  async function save() {
    saving = true;
    message = "";
    try {
      const overrides: Record<string, unknown> = {};
      if (cropMode) overrides.crop_mode = cropMode;
      if (scale !== 1.0) overrides.scale = scale;
      if (speed !== 1.0) overrides.speed = speed;
      if (smartZoom) overrides.smart = true;
      const rendering: RenderingDefaults = {
        iteration_mappings: mappings,
        default_profile: defaultProfile || null,
        default_plugin_profile: defaultPluginProfile || null,
        concat_by_default: concatByDefault,
        overrides: Object.keys(overrides).length > 0 ? overrides as RenderingDefaults["overrides"] : undefined,
      };
      const updated = { ...dockSettings, rendering };
      await saveDockSettings(updated);
      setDockSettings(updated);
      message = "Saved.";
      setTimeout(() => { message = ""; }, 2000);
    } catch (e) {
      message = `Error: ${e}`;
    }
    saving = false;
  }

  function resetToDefaults() {
    const configMappings = config?.iterations?.mappings ?? {};
    mappings = { ...configMappings };
    defaultProfile = "";
    defaultPluginProfile = "";
    concatByDefault = false;
    cropMode = "";
    scale = 1.0;
    speed = 1.0;
    smartZoom = false;
  }
</script>

<div class="space-y-6">
  {#if message}
    <p class="text-sm text-text-muted">{message}</p>
  {/if}

  <!-- Default Profile -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-3">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Default Profile</h3>
    <select
      bind:value={defaultProfile}
      class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
    >
      <option value="">None (use iteration mappings)</option>
      {#each profiles as profile}
        <option value={profile.name}>{profileLabel(profile.name)}</option>
      {/each}
    </select>

    <div>
      <label class="block text-xs text-text-muted mb-1" for="default-plugin-profile">Default Plugin Profile</label>
      <select
        id="default-plugin-profile"
        bind:value={defaultPluginProfile}
        class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
      >
        <option value="">None (no plugins)</option>
        {#each pluginProfiles as pp}
          <option value={pp.name}>{pp.name}{pp.active ? " (active)" : ""}</option>
        {/each}
      </select>
      <p class="text-xs text-text-muted mt-1">Plugin profile to use when rendering. Determines which plugins process the output.</p>
    </div>

    <label class="flex items-center gap-2 text-sm text-text-muted cursor-pointer">
      <input type="checkbox" bind:checked={concatByDefault} class="accent-secondary" />
      Concatenate multi-format renders by default
    </label>
  </div>

  <!-- Render Override Defaults -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Render Overrides</h3>
    <p class="text-xs text-text-muted">Default overrides applied to all renders unless changed per-clip.</p>

    <div>
      <label class="block text-xs text-text-muted mb-1" for="settings-crop-mode">Crop Mode</label>
      <select
        id="settings-crop-mode"
        bind:value={cropMode}
        class="w-full px-2 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
      >
        <option value="">Profile Default</option>
        <option value="pad">Pad (letterbox)</option>
        <option value="crop">Crop (fill)</option>
      </select>
    </div>

    <div>
      <label class="block text-xs text-text-muted mb-1" for="settings-scale">Scale: {scale.toFixed(1)}</label>
      <div class="flex items-center gap-2">
        <input id="settings-scale" type="range" min="0.5" max="3.0" step="0.1" bind:value={scale} class="flex-1 accent-secondary" />
        <button class="text-[10px] text-text-muted hover:text-text" onclick={() => scale = 1.0}>reset</button>
      </div>
    </div>

    <div>
      <label class="block text-xs text-text-muted mb-1" for="settings-speed">Speed: {speed.toFixed(1)}x</label>
      <div class="flex items-center gap-2">
        <input id="settings-speed" type="range" min="0.5" max="2.0" step="0.1" bind:value={speed} class="flex-1 accent-secondary" />
        <button class="text-[10px] text-text-muted hover:text-text" onclick={() => speed = 1.0}>reset</button>
      </div>
    </div>

    <label class="flex items-center gap-2 text-sm text-text-muted cursor-pointer">
      <input type="checkbox" bind:checked={smartZoom} class="accent-secondary" />
      Smart zoom
    </label>
  </div>

  <!-- Iteration Mappings -->
  <div class="bg-surface rounded-lg border border-border p-4 space-y-4">
    <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted">Iteration Mappings</h3>
    <p class="text-xs text-text-muted">Configure which render profiles are used for each event type.</p>

    {#each Object.entries(mappings) as [eventType, profileNames]}
      <div class="space-y-1.5">
        <div class="text-sm font-medium">{eventType}</div>
        {#if profileNames.length > 0}
          <div class="flex flex-wrap gap-1">
            {#each profileNames as pname, i}
              <span class="inline-flex items-center gap-1 px-2 py-0.5 bg-bg rounded text-xs text-text">
                {pname}
                <button
                  class="text-text-muted hover:text-accent transition-colors"
                  onclick={() => removeProfileFromMapping(eventType, i)}
                >&times;</button>
              </span>
            {/each}
          </div>
        {:else}
          <p class="text-xs text-text-muted">No profiles assigned.</p>
        {/if}
        <div class="flex gap-1.5">
          <select
            bind:value={addProfileSelections[eventType]}
            class="flex-1 px-2 py-1 bg-bg border border-border rounded text-xs text-text focus:outline-none focus:border-secondary"
          >
            <option value="">Add profile...</option>
            {#each profiles as profile}
              <option value={profile.name}>{profileLabel(profile.name)}</option>
            {/each}
          </select>
          <button
            class="px-2 py-1 text-xs bg-bg border border-border rounded text-text-muted hover:text-text hover:border-secondary transition-colors"
            onclick={() => {
              addProfileToMapping(eventType, addProfileSelections[eventType] ?? "");
              addProfileSelections[eventType] = "";
            }}
          >+ Add</button>
        </div>
      </div>
    {/each}

    <!-- Add new event type mapping -->
    {#if unmappedEventTypes.length > 0}
      <div class="flex gap-2 pt-2 border-t border-border">
        <select
          bind:value={newMappingType}
          class="flex-1 px-3 py-1.5 bg-bg border border-border rounded text-sm text-text focus:outline-none focus:border-secondary"
        >
          <option value="">Add event type...</option>
          {#each unmappedEventTypes as et}
            <option value={et}>{et}</option>
          {/each}
        </select>
        <button
          class="px-3 py-1.5 bg-bg border border-border rounded text-sm text-text-muted hover:text-text hover:border-secondary transition-colors disabled:opacity-50"
          onclick={addEventTypeMapping}
          disabled={!newMappingType}
        >Add</button>
      </div>
    {/if}
  </div>

  <!-- Actions -->
  <div class="flex gap-2">
    <button
      class="px-4 py-1.5 bg-primary hover:bg-primary-light text-text rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
      onclick={save}
      disabled={saving}
    >
      {saving ? "Saving..." : "Save"}
    </button>
    <button
      class="px-3 py-1.5 text-sm text-text-muted hover:text-text transition-colors"
      onclick={resetToDefaults}
    >
      Reset to Config Defaults
    </button>
  </div>
</div>
