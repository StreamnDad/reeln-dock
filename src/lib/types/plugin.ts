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

export interface RegistryPlugin {
  name: string;
  package: string;
  description: string;
  capabilities: string[];
  homepage: string;
  min_reeln_version: string;
  author: string;
  license: string;
}

export interface VersionInfo {
  app_version: string;
  config_version: number | null;
}
