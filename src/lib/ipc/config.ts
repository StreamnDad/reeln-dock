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
