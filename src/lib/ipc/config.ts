import { invoke } from "@tauri-apps/api/core";
import type { DockSettings, DockSettingsWithConfig, LoadedConfig } from "$lib/types/dock";

export async function loadDockSettings(): Promise<DockSettingsWithConfig> {
  return invoke<DockSettingsWithConfig>("load_dock_settings");
}

export async function saveDockSettings(
  settings: DockSettings,
): Promise<DockSettingsWithConfig> {
  return invoke<DockSettingsWithConfig>("save_dock_settings", { settings });
}

export async function loadConfigFromPath(
  path: string,
): Promise<LoadedConfig> {
  return invoke<LoadedConfig>("load_config_from_path", { path });
}

export async function getConfigPath(
  profile?: string,
): Promise<string> {
  return invoke<string>("get_config_path", { profile: profile ?? null });
}

export async function saveEventTypes(
  eventTypes: { name: string; team_specific: boolean }[],
): Promise<void> {
  return invoke<void>("save_event_types", { eventTypes });
}

export async function saveRenderProfile(
  profileKey: string,
  profile: Record<string, unknown>,
): Promise<void> {
  return invoke<void>("save_render_profile", { profileKey, profile });
}

export async function deleteRenderProfile(
  profileKey: string,
): Promise<void> {
  return invoke<void>("delete_render_profile", { profileKey });
}

export async function renameRenderProfile(
  oldKey: string,
  newKey: string,
): Promise<void> {
  return invoke<void>("rename_render_profile", { oldKey, newKey });
}
