import { writable, derived } from "svelte/store";
import type { GameSummary, TournamentGroup } from "$lib/types/game";

export type GameStatus = "all" | "new" | "active" | "done";

export const games = writable<GameSummary[]>([]);
export const selectedLevel = writable<string | null>(null);
export const selectedGameDir = writable<string | null>(null);
export const selectedEventId = writable<string | null>(null);
export const statusFilter = writable<GameStatus>("all");

// --- Derived stores ---

export const levels = derived(games, ($games) => {
  const s = new Set<string>();
  for (const g of $games) {
    if (g.state.game_info.level) s.add(g.state.game_info.level);
  }
  return Array.from(s).sort();
});

export const selectedGame = derived(
  [games, selectedGameDir],
  ([$games, $dir]) => $games.find((g) => g.dir_path === $dir),
);

export const selectedEvent = derived(
  [selectedGame, selectedEventId],
  ([$game, $eventId]) => {
    if (!$game || !$eventId) return undefined;
    return $game.state.events.find((e) => e.id === $eventId);
  },
);

export const statusCounts = derived(games, ($games) => {
  const counts: Record<GameStatus, number> = { all: $games.length, new: 0, active: 0, done: 0 };
  for (const g of $games) {
    counts[gameStatus(g)]++;
  }
  return counts;
});

export const allTournamentNames = derived(games, ($games) => {
  const names = new Set<string>();
  for (const g of $games) {
    if (g.state.game_info.tournament) names.add(g.state.game_info.tournament);
  }
  return Array.from(names).sort();
});

// --- Setters with side effects ---

export function setGames(value: GameSummary[]): void {
  games.set(value);
}

export function setSelectedLevel(level: string | null): void {
  selectedLevel.set(level);
  selectedGameDir.set(null);
  selectedEventId.set(null);
}

export function setSelectedGameDir(dir: string | null): void {
  selectedGameDir.set(dir);
  selectedEventId.set(null);
}

export function setSelectedEventId(id: string | null): void {
  selectedEventId.set(id);
}

export function setStatusFilter(status: GameStatus): void {
  statusFilter.set(status);
}

export function updateGameState(
  dirPath: string,
  updater: (game: GameSummary) => GameSummary,
): void {
  games.update(($games) => $games.map((g) => (g.dir_path === dirPath ? updater(g) : g)));
}

// --- Pure helper functions ---

export function gameStatus(game: GameSummary): "new" | "active" | "done" {
  if (game.state.finished) return "done";
  if (game.state.segments_processed.length > 0) return "active";
  return "new";
}

export type GameSortOrder = "date-desc" | "date-asc";

function compareGamesByDate(a: GameSummary, b: GameSummary, order: GameSortOrder): number {
  const dateA = a.state.game_info.date || a.state.created_at || "";
  const dateB = b.state.game_info.date || b.state.created_at || "";
  const cmp = dateA.localeCompare(dateB);
  return order === "date-asc" ? cmp : -cmp;
}

export function getTournamentGroups(
  allGames: GameSummary[],
  level: string | null,
  status: GameStatus,
  archivedNames?: Set<string>,
  showArchived?: boolean,
  sortOrder: GameSortOrder = "date-desc",
): TournamentGroup[] {
  let filtered = level
    ? allGames.filter((g) => g.state.game_info.level === level)
    : allGames;

  if (status !== "all") {
    filtered = filtered.filter((g) => gameStatus(g) === status);
  }

  const grouped = new Map<string, GameSummary[]>();
  for (const game of filtered) {
    const tournament = game.state.game_info.tournament || "Ungrouped";
    if (archivedNames && !showArchived && archivedNames.has(tournament)) continue;
    const list = grouped.get(tournament) ?? [];
    list.push(game);
    grouped.set(tournament, list);
  }
  return Array.from(grouped.entries())
    .map(([tournament, gameList]) => ({
      tournament,
      games: [...gameList].sort((a, b) => compareGamesByDate(a, b, sortOrder)),
    }))
    .sort((a, b) => a.tournament.localeCompare(b.tournament));
}

export function getGamesForTournament(allGames: GameSummary[], tournament: string): GameSummary[] {
  return allGames.filter(
    (g) => (g.state.game_info.tournament || "Ungrouped") === tournament,
  );
}
