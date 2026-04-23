/** Tracks available updates detected by the update checker. */

export interface UpdateInfo {
  name: string;
  current: string;
  latest: string;
  release_notes: string;
  release_url: string;
  published_at: string;
}

let updates_ = $state<UpdateInfo[]>([]);

export function setAvailableUpdates(updates: UpdateInfo[]): void {
  updates_ = updates;
}

export function getAvailableUpdates(): UpdateInfo[] {
  return updates_;
}

/** Get update info for a specific plugin by short name (e.g., "openai"). */
export function getPluginUpdate(pluginName: string): UpdateInfo | undefined {
  return updates_.find(
    (u) => u.name === `reeln-plugin-${pluginName}`,
  );
}

/** Check if any plugin updates are available. */
export function hasPluginUpdates(): boolean {
  return updates_.some((u) => u.name.startsWith("reeln-plugin-"));
}
