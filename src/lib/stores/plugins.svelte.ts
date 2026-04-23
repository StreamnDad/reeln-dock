/** Plugin view state — tracks selected config profile, plugins, and registry. */
import type {
  AuthCheckResult,
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
  checkPluginAuth,
  refreshPluginAuth,
  cancelPluginAuth,
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
/** Auth results keyed by "profilePath::pluginName" for per-profile caching. */
let authResults = $state<Map<string, AuthCheckResult[]>>(new Map());
let authLoading = $state(false);

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
  // Reload auth for the new profile's config
  await loadAuthStatus();
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

// ── Auth ──────────────────────────────────────────────────────────

function authKey(pluginName: string): string {
  return `${selectedProfilePath}::${pluginName}`;
}

export function getAuthResults(pluginName: string): AuthCheckResult[] {
  return authResults.get(authKey(pluginName)) ?? [];
}

/** Expose the full auth map so components can derive from it reactively. */
export function getAuthMap(): Map<string, AuthCheckResult[]> {
  return authResults;
}

export function isAuthLoading(): boolean {
  return authLoading;
}

/** Load auth status for all plugins (or a single plugin). Skips if already cached for this profile. */
export async function loadAuthStatus(pluginName?: string): Promise<void> {
  // Capture profile path at call time — prevents drift if user switches tabs during fetch
  const profilePath = selectedProfilePath;
  if (!profilePath) return;

  const makeKey = (name: string) => `${profilePath}::${name}`;

  // Skip if we already have cached results for this profile
  if (!pluginName) {
    const prefix = `${profilePath}::`;
    const hasCached = [...authResults.keys()].some((k) => k.startsWith(prefix));
    if (hasCached) return;
  } else if (authResults.has(makeKey(pluginName))) {
    return;
  }

  authLoading = true;
  try {
    const reports = await checkPluginAuth(pluginName, profilePath);
    const next = new Map(authResults);
    for (const report of reports) {
      next.set(makeKey(report.plugin_name), report.results);
    }
    authResults = next;
  } catch (err) {
    log.error("Plugins", `Failed to check auth: ${err}`);
  } finally {
    authLoading = false;
  }
}

/** Force reauthentication for a specific plugin, returns updated results. */
export async function refreshAuth(
  pluginName: string,
): Promise<AuthCheckResult[]> {
  try {
    const reports = await refreshPluginAuth(
      pluginName,
      selectedProfilePath || undefined,
    );
    const next = new Map(authResults);
    for (const report of reports) {
      next.set(authKey(report.plugin_name), report.results);
    }
    authResults = next;
    log.info("Plugins", `Refreshed auth for ${pluginName}`);
    return next.get(authKey(pluginName)) ?? [];
  } catch (err) {
    log.error("Plugins", `Failed to refresh auth for ${pluginName}: ${err}`);
    return [];
  }
}

/** Cancel an in-progress auth subprocess. */
export async function cancelAuth(): Promise<void> {
  try {
    await cancelPluginAuth();
    authLoading = false;
    log.info("Plugins", "Auth cancelled");
  } catch (err) {
    log.error("Plugins", `Failed to cancel auth: ${err}`);
  }
}
