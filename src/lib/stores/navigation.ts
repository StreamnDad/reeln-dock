import { writable } from "svelte/store";

export type View = "games" | "plugins" | "registry" | "queue" | "settings";
export type SidebarMode = "games" | "teams" | "tournaments";

export const currentView = writable<View>("games");
export const sidebarMode = writable<SidebarMode>("games");
export const selectedTeamKey = writable<string | null>(null);
export const selectedTournamentName = writable<string | null>(null);

/** When set, the Settings > Teams tab should auto-select this team. */
export const settingsTeamTarget = writable<string | null>(null);

/** When set, the Settings > Tournaments tab should auto-select this tournament. */
export const settingsTournamentTarget = writable<string | null>(null);

export function setView(v: View): void {
  currentView.set(v);
}

export function setSidebarMode(mode: SidebarMode): void {
  sidebarMode.set(mode);
  selectedTeamKey.set(null);
  selectedTournamentName.set(null);
}
