<script lang="ts">
  import type { RegistryPlugin, PluginDetail } from "$lib/types/plugin";
  import {
    getProfiles,
    getSelectedProfilePath,
    getPlugins,
    getRegistry,
    isLoading,
    isRegistryLoading,
    getRegistryError,
    loadProfiles,
    loadRegistry,
    loadVersionInfo,
    getVersionInfo_,
    selectProfile,
    addPlugin,
    removePlugin,
    createProfile,
  } from "$lib/stores/plugins.svelte";
  import { getEnforceHooks, setEnforceHooks } from "$lib/ipc/plugins";
  import { isPluginInstalled, isCliAvailable } from "$lib/stores/cli.svelte";
  import PluginCard from "./PluginCard.svelte";

  let profiles = $derived(getProfiles());
  let selectedPath = $derived(getSelectedProfilePath());
  let configuredPlugins = $derived(getPlugins());
  let registryPlugins = $derived(getRegistry());
  let loading = $derived(isLoading());
  let regLoading = $derived(isRegistryLoading());
  let regError = $derived(getRegistryError());
  let version = $derived(getVersionInfo_());

  // Enforce hooks toggle
  let enforceHooks = $state(true);
  let enforceLoading = $state(false);

  $effect(() => {
    if (selectedPath) {
      getEnforceHooks(selectedPath)
        .then((v) => { enforceHooks = v; })
        .catch(() => { enforceHooks = true; });
    }
  });

  async function toggleEnforceHooks() {
    if (!selectedPath) return;
    enforceLoading = true;
    try {
      enforceHooks = await setEnforceHooks(selectedPath, !enforceHooks);
    } catch {
      // revert on failure
    } finally {
      enforceLoading = false;
    }
  }

  // New profile form
  let showNewProfile = $state(false);
  let newProfileName = $state("");
  let creatingProfile = $state(false);
  let createError = $state<string | null>(null);

  interface UnifiedPlugin {
    name: string;
    status: "enabled" | "enabled_not_installed" | "disabled" | "available";
    detail?: PluginDetail;
    registryInfo?: RegistryPlugin;
  }

  function resolveEnabledStatus(name: string): "enabled" | "enabled_not_installed" {
    // When CLI is unavailable, degrade gracefully — show as enabled
    if (!isCliAvailable()) return "enabled";
    return isPluginInstalled(name) ? "enabled" : "enabled_not_installed";
  }

  let unifiedPlugins = $derived.by(() => {
    const configuredMap = new Map(configuredPlugins.map((p) => [p.name, p]));
    const seen = new Set<string>();
    const result: UnifiedPlugin[] = [];

    const cliUp = isCliAvailable();

    for (const rp of registryPlugins) {
      seen.add(rp.name);
      const detail = configuredMap.get(rp.name);
      if (!detail) continue; // "available" plugins belong on the Registry page
      const installed = !cliUp || isPluginInstalled(rp.name);
      // Hide disabled plugins that aren't installed — Registry page handles those
      if (!detail.enabled && !installed) continue;
      result.push({
        name: rp.name,
        status: detail.enabled ? resolveEnabledStatus(rp.name) : "disabled",
        detail,
        registryInfo: rp,
      });
    }

    for (const p of configuredPlugins) {
      if (seen.has(p.name)) continue;
      const installed = !cliUp || isPluginInstalled(p.name);
      if (!p.enabled && !installed) continue;
      result.push({
        name: p.name,
        status: p.enabled ? resolveEnabledStatus(p.name) : "disabled",
        detail: p,
      });
    }

    return result;
  });

  $effect(() => {
    loadProfiles();
    loadRegistry();
    loadVersionInfo();
  });

  async function handleCreateProfile() {
    const name = newProfileName.trim().toLowerCase().replace(/\s+/g, "-");
    if (!name) return;
    creatingProfile = true;
    createError = null;
    try {
      await createProfile(name);
      showNewProfile = false;
      newProfileName = "";
    } catch (err) {
      createError = String(err);
    } finally {
      creatingProfile = false;
    }
  }

  async function handleAddPlugin(name: string) {
    await addPlugin(name);
  }

  async function handleRemovePlugin(name: string) {
    await removePlugin(name);
  }
</script>

<div>
  <!-- Header with version -->
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-bold">Plugins</h2>
    {#if version}
      <div class="flex items-center gap-3 text-xs text-text-muted">
        <span>dock v{version.app_version}</span>
        {#if version.config_version != null}
          <span>config v{version.config_version}</span>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Config Profile Selector -->
  <div class="mb-4">
    <label class="block text-xs font-semibold uppercase tracking-wider text-text-muted mb-1.5" for="profile-select">
      Config Profile
    </label>
    <div class="flex flex-wrap items-center gap-2">
      {#each profiles as profile (profile.path)}
        <button
          class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border"
          class:bg-primary={selectedPath === profile.path}
          class:border-primary={selectedPath === profile.path}
          class:text-text={selectedPath === profile.path}
          class:bg-bg={selectedPath !== profile.path}
          class:border-border={selectedPath !== profile.path}
          class:text-text-muted={selectedPath !== profile.path}
          class:hover:border-secondary={selectedPath !== profile.path}
          onclick={() => selectProfile(profile.path)}
        >
          {profile.name}
          {#if profile.active}
            <span class="ml-1 text-xs opacity-60">(active)</span>
          {/if}
        </button>
      {/each}

      <!-- New profile button / inline form -->
      {#if showNewProfile}
        <form
          class="flex items-center gap-1.5"
          onsubmit={(e) => { e.preventDefault(); handleCreateProfile(); }}
        >
          <input
            type="text"
            bind:value={newProfileName}
            placeholder="profile-name"
            class="px-2 py-1.5 bg-bg border border-border rounded text-sm text-text font-mono focus:outline-none focus:border-secondary w-40"
            autofocus
          />
          <button
            type="submit"
            class="px-2 py-1.5 text-xs bg-secondary text-text rounded transition-colors disabled:opacity-50"
            disabled={creatingProfile || !newProfileName.trim()}
          >
            {creatingProfile ? "..." : "Create"}
          </button>
          <button
            type="button"
            class="px-2 py-1.5 text-xs text-text-muted hover:text-text transition-colors"
            onclick={() => { showNewProfile = false; newProfileName = ""; createError = null; }}
          >
            Cancel
          </button>
        </form>
        {#if createError}
          <span class="text-xs text-accent">{createError}</span>
        {/if}
      {:else}
        <button
          class="px-3 py-2 rounded-lg text-sm font-medium transition-colors border border-dashed border-border text-text-muted hover:border-secondary hover:text-text"
          onclick={() => (showNewProfile = true)}
        >
          + New Profile
        </button>
      {/if}
    </div>
  </div>

  <!-- Hook Enforcement Toggle -->
  {#if selectedPath}
    <div class="mb-4 flex items-center justify-between px-3 py-2.5 bg-bg rounded-lg border border-border">
      <div>
        <span class="text-sm font-medium">Enforce Hook Registry</span>
        <p class="text-xs text-text-muted mt-0.5">
          When enabled, plugins can only register hooks declared in the registry.
          Disable to allow all hooks.
        </p>
      </div>
      <button
        class="relative w-10 h-5 rounded-full transition-colors"
        class:bg-primary={enforceHooks}
        class:bg-zinc-600={!enforceHooks}
        disabled={enforceLoading}
        onclick={toggleEnforceHooks}
        aria-label="Toggle hook registry enforcement"
      >
        <span
          class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full transition-transform"
          class:translate-x-5={enforceHooks}
        ></span>
      </button>
    </div>
  {/if}

  <!-- Plugin List -->
  {#if loading || regLoading}
    <div class="text-text-muted text-center py-8">
      <p class="text-sm">Loading plugins...</p>
    </div>
  {:else if profiles.length === 0}
    <div class="text-text-muted text-center py-12">
      <p class="text-lg mb-2">No config profiles found</p>
      <p class="text-sm">Set a reeln config path in Settings, or create a new profile above.</p>
    </div>
  {:else if regError}
    <div class="text-center py-8">
      <p class="text-sm text-accent mb-2">{regError}</p>
      <button
        class="px-3 py-1 text-xs border border-border text-text-muted hover:text-text rounded transition-colors"
        onclick={() => loadRegistry()}
      >
        Retry
      </button>
    </div>
  {:else}
    <div class="space-y-3">
      {#each unifiedPlugins as up (up.name)}
        <PluginCard
          name={up.name}
          status={up.status}
          detail={up.detail}
          registryInfo={up.registryInfo}
          onAdd={() => handleAddPlugin(up.name)}
          onRemove={() => handleRemovePlugin(up.name)}
        />
      {/each}
    </div>
  {/if}
</div>
