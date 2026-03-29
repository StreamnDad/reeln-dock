import { invoke } from "@tauri-apps/api/core";
import type {
  ConfigProfile,
  PluginDetail,
  RegistryPlugin,
  VersionInfo,
} from "$lib/types/plugin";

export async function listConfigProfiles(): Promise<ConfigProfile[]> {
  return invoke<ConfigProfile[]>("list_config_profiles");
}

export async function listPluginsForProfile(
  profilePath: string,
): Promise<PluginDetail[]> {
  return invoke<PluginDetail[]>("list_plugins_for_profile", {
    profilePath,
  });
}

export async function togglePluginInConfig(
  profilePath: string,
  pluginName: string,
): Promise<PluginDetail[]> {
  return invoke<PluginDetail[]>("toggle_plugin_in_config", {
    profilePath,
    pluginName,
  });
}

export async function updatePluginInConfig(
  profilePath: string,
  pluginName: string,
  settings: Record<string, unknown>,
): Promise<PluginDetail[]> {
  return invoke<PluginDetail[]>("update_plugin_in_config", {
    profilePath,
    pluginName,
    settings,
  });
}

export async function fetchPluginRegistry(): Promise<RegistryPlugin[]> {
  return invoke<RegistryPlugin[]>("fetch_plugin_registry");
}

export async function addPluginToConfig(
  profilePath: string,
  pluginName: string,
): Promise<PluginDetail[]> {
  return invoke<PluginDetail[]>("add_plugin_to_config", {
    profilePath,
    pluginName,
  });
}

export async function removePluginFromConfig(
  profilePath: string,
  pluginName: string,
): Promise<PluginDetail[]> {
  return invoke<PluginDetail[]>("remove_plugin_from_config", {
    profilePath,
    pluginName,
  });
}

export async function createConfigProfile(
  profileName: string,
): Promise<ConfigProfile[]> {
  return invoke<ConfigProfile[]>("create_config_profile", { profileName });
}

export async function getVersionInfo(): Promise<VersionInfo> {
  return invoke<VersionInfo>("get_version_info");
}

export async function getEnforceHooks(
  profilePath: string,
): Promise<boolean> {
  return invoke<boolean>("get_enforce_hooks", { profilePath });
}

export async function setEnforceHooks(
  profilePath: string,
  enforce: boolean,
): Promise<boolean> {
  return invoke<boolean>("set_enforce_hooks", { profilePath, enforce });
}
