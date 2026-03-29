import { invoke } from "@tauri-apps/api/core";
import type { GameSummary, GameState } from "$lib/types/game";
import type { HookExecutionResult } from "$lib/types/hooks";
import type { SportAlias } from "$lib/types/sport";

export async function listGames(
  outputDir: string,
): Promise<GameSummary[]> {
  return invoke<GameSummary[]>("list_games", { outputDir });
}

export async function getGameState(
  gameDir: string,
): Promise<GameState> {
  return invoke<GameState>("get_game_state", { gameDir });
}

export async function setGameTournament(
  gameDir: string,
  tournament: string,
): Promise<GameState> {
  return invoke<GameState>("set_game_tournament", { gameDir, tournament });
}

export async function updateGameEvent(
  gameDir: string,
  eventId: string,
  field: string,
  value: string,
): Promise<GameState> {
  return invoke<GameState>("update_game_event", { gameDir, eventId, field, value });
}

export async function listSports(): Promise<SportAlias[]> {
  return invoke<SportAlias[]>("list_sports");
}

export async function initGame(params: {
  sport: string;
  homeTeam: string;
  awayTeam: string;
  date: string;
  venue?: string;
  gameTime?: string;
  level?: string;
  tournament?: string;
  periodLength?: number;
}): Promise<GameSummary> {
  return invoke<GameSummary>("init_game", params);
}

export async function processSegment(
  gameDir: string,
  segmentNumber: number,
): Promise<GameState> {
  return invoke<GameState>("process_segment", { gameDir, segmentNumber });
}

export async function mergeHighlights(
  gameDir: string,
): Promise<GameState> {
  return invoke<GameState>("merge_highlights", { gameDir });
}

export async function finishGame(
  gameDir: string,
): Promise<GameState> {
  return invoke<GameState>("finish_game", { gameDir });
}

export async function detectReelnCli(): Promise<string> {
  return invoke<string>("detect_reeln_cli");
}

export async function executePluginHook(
  hook: string,
  contextData: Record<string, unknown>,
  shared: Record<string, unknown>,
  configPath?: string,
): Promise<HookExecutionResult> {
  return invoke<HookExecutionResult>("execute_plugin_hook", {
    hook,
    contextData,
    shared,
    configPath: configPath ?? null,
  });
}
