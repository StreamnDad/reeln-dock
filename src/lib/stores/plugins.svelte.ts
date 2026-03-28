/** Plugin view state — tracks selected config profile, plugins, and registry. */
import type {
  ConfigProfile,
  PluginDetail,
  RegistryPlugin,
  VersionInfo,
} from "$lib/types/plugin";
import {
  listConfigProfiles,
  listPluginsForProfile,
  togglePluginInConfig,
  updatePluginInConfig,
  fetchPluginRegistry,
  addPluginToConfig,
  removePluginFromConfig,
  createConfigProfile,
  getVersionInfo,
} from "$lib/ipc/plugins";
import { log } from "$lib/stores/log.svelte";

let profiles = $state<ConfigProfile[]>([]);
let selectedProfilePath = $state<string>("");
let plugins = $state<PluginDetail[]>([]);
let registry = $state<RegistryPlugin[]>([]);
let versionInfo = $state<VersionInfo | null>(null);
let loading = $state(false);
let registryLoading = $state(false);
let registryError = $state<string | null>(null);

export function getProfiles(): ConfigProfile[] {
  return profiles;
}

export function getSelectedProfilePath(): string {
  return selectedProfilePath;
}

export function getSelectedProfileName(): string {
  const found = profiles.find((p) => p.path === selectedProfilePath);
  return found?.name ?? "";
}

export function getPlugins(): PluginDetail[] {
  return plugins;
}

export function getRegistry(): RegistryPlugin[] {
  return registry;
}

export function isLoading(): boolean {
  return loading;
}

export function isRegistryLoading(): boolean {
  return registryLoading;
}

export function getRegistryError(): string | null {
  return registryError;
}

/** Registry plugins not yet in the current profile's plugin list. */
export function getAvailablePlugins(): RegistryPlugin[] {
  const configuredNames = new Set(plugins.map((p) => p.name));
  return registry.filter((rp) => !configuredNames.has(rp.name));
}

/** Registry info keyed by plugin name for enriching configured plugins. */
export function getRegistryMap(): Map<string, RegistryPlugin> {
  return new Map(registry.map((rp) => [rp.name, rp]));
}

export async function loadProfiles(): Promise<void> {
  try {
    profiles = await listConfigProfiles();
    // Auto-select the active profile
    const active = profiles.find((p) => p.active);
    if (active && !selectedProfilePath) {
      await selectProfile(active.path);
    } else if (profiles.length > 0 && !selectedProfilePath) {
      await selectProfile(profiles[0].path);
    }
  } catch (err) {
    log.error("Plugins", `Failed to load config profiles: ${err}`);
  }
}

export async function selectProfile(path: string): Promise<void> {
  selectedProfilePath = path;
  loading = true;
  try {
    plugins = await listPluginsForProfile(path);
  } catch (err) {
    log.error("Plugins", `Failed to load plugins for profile: ${err}`);
    plugins = [];
  } finally {
    loading = false;
  }
}

export async function loadRegistry(): Promise<void> {
  registryLoading = true;
  registryError = null;
  try {
    registry = await fetchPluginRegistry();
  } catch (err) {
    registryError = String(err);
    log.error("Plugins", `Failed to load plugin registry: ${err}`);
    registry = [];
  } finally {
    registryLoading = false;
  }
}

export async function togglePlugin(pluginName: string): Promise<void> {
  if (!selectedProfilePath) return;
  try {
    plugins = await togglePluginInConfig(selectedProfilePath, pluginName);
    log.info("Plugins", `Toggled ${pluginName}`);
  } catch (err) {
    log.error("Plugins", `Failed to toggle ${pluginName}: ${err}`);
  }
}

export async function addPlugin(pluginName: string): Promise<void> {
  if (!selectedProfilePath) return;
  try {
    plugins = await addPluginToConfig(selectedProfilePath, pluginName);
    log.info("Plugins", `Added ${pluginName} to profile`);
  } catch (err) {
    log.error("Plugins", `Failed to add ${pluginName}: ${err}`);
  }
}

export async function removePlugin(pluginName: string): Promise<void> {
  if (!selectedProfilePath) return;
  try {
    plugins = await removePluginFromConfig(selectedProfilePath, pluginName);
    log.info("Plugins", `Removed ${pluginName} from profile`);
  } catch (err) {
    log.error("Plugins", `Failed to remove ${pluginName}: ${err}`);
  }
}

export async function updatePluginSettings(
  pluginName: string,
  settings: Record<string, unknown>,
): Promise<void> {
  if (!selectedProfilePath) return;
  try {
    plugins = await updatePluginInConfig(
      selectedProfilePath,
      pluginName,
      settings,
    );
    log.info("Plugins", `Updated settings for ${pluginName}`);
  } catch (err) {
    log.error("Plugins", `Failed to update ${pluginName} settings: ${err}`);
  }
}

export async function createProfile(name: string): Promise<void> {
  try {
    profiles = await createConfigProfile(name);
    // Select the new profile
    const created = profiles.find((p) => p.name === name);
    if (created) {
      await selectProfile(created.path);
    }
    log.info("Plugins", `Created profile '${name}'`);
  } catch (err) {
    log.error("Plugins", `Failed to create profile: ${err}`);
    throw err;
  }
}

export function getVersionInfo_(): VersionInfo | null {
  return versionInfo;
}

export async function loadVersionInfo(): Promise<void> {
  try {
    versionInfo = await getVersionInfo();
  } catch (err) {
    log.error("Plugins", `Failed to load version info: ${err}`);
  }
}
