import type { AppConfig } from "$lib/types/config";
import type { DockSettings } from "$lib/types/dock";

let config = $state<AppConfig | null>(null);
let dockSettings = $state<DockSettings>({
  reeln_config_path: null,
  plugin_profiles: {},
  display: { show_logos: true, sections_expanded: { games: true, teams: true, tournaments: true } },
});

export function getConfig(): AppConfig | null {
  return config;
}

export function setConfig(value: AppConfig | null): void {
  config = value;
}

export function getDockSettings(): DockSettings {
  return dockSettings;
}

export function setDockSettings(value: DockSettings): void {
  dockSettings = value;
}
