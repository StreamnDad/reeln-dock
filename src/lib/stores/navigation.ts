import { writable } from "svelte/store";

export type View = "games" | "plugins" | "registry" | "settings";
export type SidebarMode = "games" | "teams" | "tournaments";

export const currentView = writable<View>("games");
export const sidebarMode = writable<SidebarMode>("games");
export const selectedTeamKey = writable<string | null>(null);
export const selectedTournamentName = writable<string | null>(null);

export function setView(v: View): void {
  currentView.set(v);
}

export function setSidebarMode(mode: SidebarMode): void {
  sidebarMode.set(mode);
  selectedTeamKey.set(null);
  selectedTournamentName.set(null);
}
