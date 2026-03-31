import type { AppConfig } from "./config";

export interface PluginProfile {
  enabled: string[];
  settings: Record<string, unknown>;
}

export interface SectionsExpanded {
  games: boolean;
  teams: boolean;
  tournaments: boolean;
}

export interface DisplayPreferences {
  show_logos: boolean;
  sections_expanded: SectionsExpanded;
}

export interface RenderOverrideDefaults {
  crop_mode?: string;
  scale?: number;
  speed?: number;
  smart?: boolean;
}

export interface RenderingDefaults {
  iteration_mappings: Record<string, string[]>;
  default_profile: string | null;
  default_plugin_profile: string | null;
  concat_by_default: boolean;
  overrides?: RenderOverrideDefaults;
}

export interface DockSettings {
  reeln_config_path: string | null;
  plugin_profiles: Record<string, PluginProfile>;
  display: DisplayPreferences;
  rendering?: RenderingDefaults;
}

export interface DockSettingsWithConfig {
  settings: DockSettings;
  config: AppConfig | null;
}

export interface LoadedConfig {
  config: AppConfig;
  path: string;
}
