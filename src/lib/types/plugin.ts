export interface ConfigProfile {
  name: string;
  path: string;
  active: boolean;
}

export interface PluginDetail {
  name: string;
  enabled: boolean;
  settings: Record<string, unknown>;
}

export interface PluginUIField {
  id: string;
  label: string;
  type: "boolean" | "number" | "string" | "select";
  default?: unknown;
  description?: string;
  min?: number;
  max?: number;
  step?: number;
  options?: { value: string; label: string }[];
  maps_to?: string;
}

export interface PluginUIScreen {
  fields: PluginUIField[];
}

export interface PluginUIContributions {
  render_options?: PluginUIScreen;
  settings?: PluginUIScreen;
  clip_review?: PluginUIScreen;
}

export interface RegistryPlugin {
  name: string;
  package: string;
  description: string;
  capabilities: string[];
  homepage: string;
  min_reeln_version: string;
  author: string;
  license: string;
  ui_contributions?: PluginUIContributions;
}

export interface VersionInfo {
  app_version: string;
  config_version: number | null;
}
