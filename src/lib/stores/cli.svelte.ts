/**
 * Tracks reeln-cli availability, version, and installed plugins.
 * Used to gate features that require the CLI or specific plugins.
 */

import { invoke } from "@tauri-apps/api/core";
import { log } from "$lib/stores/log.svelte";

interface CliPluginInfo {
  name: string;
  version: string;
}

interface CliVersionInfo {
  cli_version: string;
  cli_path: string;
  plugins: CliPluginInfo[];
}

interface CliStatus {
  available: boolean;
  path: string | null;
  version: string | null;
  plugins: CliPluginInfo[];
  checkedAt: number;
}

let status = $state<CliStatus>({
  available: false,
  path: null,
  version: null,
  plugins: [],
  checkedAt: 0,
});

let initialized = false;

export async function initCliStatus(): Promise<void> {
  if (initialized) return;
  initialized = true;
  await refreshCliStatus();
}

export async function refreshCliStatus(): Promise<void> {
  try {
    const info = await invoke<CliVersionInfo>("get_cli_version");
    status = {
      available: true,
      path: info.cli_path,
      version: info.cli_version,
      plugins: info.plugins,
      checkedAt: Date.now(),
    };
    log.info("CLI", `Found reeln ${info.cli_version} at ${info.cli_path} with ${info.plugins.length} plugins`);
  } catch {
    status = {
      available: false,
      path: null,
      version: null,
      plugins: [],
      checkedAt: Date.now(),
    };
    log.info("CLI", "reeln CLI not found — plugin features disabled");
  }
}

export function getCliStatus(): CliStatus {
  return status;
}

export function isCliAvailable(): boolean {
  return status.available;
}

export function isPluginInstalled(name: string): boolean {
  return status.available && status.plugins.some((p) => p.name === name);
}

export function getCliVersion(): string | null {
  return status.version;
}

export function getInstalledPlugins(): CliPluginInfo[] {
  return status.plugins;
}
