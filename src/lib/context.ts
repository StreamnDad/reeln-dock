import type { GameSummary } from "$lib/types/game";
import type { View, SidebarMode } from "$lib/stores/navigation";

export interface AppContext {
  readonly games: GameSummary[];
  readonly view: View;
  readonly sidebarMode: SidebarMode;
  readonly selectedTeamKey: string | null;
  readonly selectedTournamentName: string | null;
  setView: (v: View) => void;
  setSidebarMode: (m: SidebarMode) => void;
  setTeamKey: (k: string | null) => void;
  setTournamentName: (n: string | null) => void;
  setGames: (data: GameSummary[]) => void;
  updateGameState: (dirPath: string, updater: (g: GameSummary) => GameSummary) => void;
}
