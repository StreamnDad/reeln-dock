/**
 * Computes active plugin UI contributions by cross-referencing the
 * registry (what plugins declare) with installed/enabled plugins.
 */

import type { PluginUIField, PluginUIContributions, RegistryPlugin } from "$lib/types/plugin";
import { fetchPluginRegistry, listConfigProfiles, listPluginsForProfile } from "$lib/ipc/plugins";
import { log } from "$lib/stores/log.svelte";

interface PluginFieldGroup {
  pluginName: string;
  fields: PluginUIField[];
}

let registry = $state<RegistryPlugin[]>([]);
let enabledPlugins = $state<Set<string>>(new Set());
let initialized = false;

/** Load registry and enabled plugins. Call once on app startup. */
export async function initPluginUI(): Promise<void> {
  if (initialized) return;
  initialized = true;
  try {
    registry = await fetchPluginRegistry();
  } catch (e) {
    log.error("PluginUI", `Failed to load registry: ${e}`);
    registry = [];
  }
  await refreshEnabledPlugins();
}

/** Refresh which plugins are installed/enabled (call after toggling plugins). */
export async function refreshEnabledPlugins(): Promise<void> {
  try {
    const profiles = await listConfigProfiles();
    const active = profiles.find((p) => p.active);
    if (active) {
      const plugins = await listPluginsForProfile(active.path);
      enabledPlugins = new Set(plugins.filter((p) => p.enabled).map((p) => p.name));
    }
  } catch (e) {
    log.error("PluginUI", `Failed to load enabled plugins: ${e}`);
  }
}

/** Get active plugin field groups for a specific screen. */
export function getActiveFieldsForScreen(screen: "render_options" | "settings" | "clip_review"): PluginFieldGroup[] {
  const groups: PluginFieldGroup[] = [];
  for (const plugin of registry) {
    if (!enabledPlugins.has(plugin.name)) continue;
    const contributions = plugin.ui_contributions as PluginUIContributions | undefined;
    if (!contributions) continue;
    const screenData = contributions[screen];
    if (!screenData || screenData.fields.length === 0) continue;
    groups.push({
      pluginName: plugin.name,
      fields: screenData.fields,
    });
  }
  return groups;
}

/** Check if a specific plugin is enabled. */
export function isPluginEnabled(name: string): boolean {
  return enabledPlugins.has(name);
}
