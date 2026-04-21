import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  listGames,
  getGameState,
  pruneGamePreview,
  pruneGameExecute,
  deleteGamePreview,
  deleteGame,
} from "./games";

const mockInvoke = invoke as Mock;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("listGames", () => {
  it("passes outputDir and returns game list", async () => {
    const games = [{ dir_path: "/games/g1", state: { finished: false } }];
    mockInvoke.mockResolvedValue(games);
    const result = await listGames("/games");
    expect(mockInvoke).toHaveBeenCalledWith("list_games", { outputDir: "/games" });
    expect(result).toEqual(games);
  });
});

describe("getGameState", () => {
  it("passes gameDir and returns state", async () => {
    const state = { finished: true };
    mockInvoke.mockResolvedValue(state);
    const result = await getGameState("/games/g1");
    expect(mockInvoke).toHaveBeenCalledWith("get_game_state", { gameDir: "/games/g1" });
    expect(result).toEqual(state);
  });
});

describe("pruneGamePreview", () => {
  it("passes gameDir, allFiles, and force=null by default", async () => {
    const preview = { files: [], total_bytes: 0, file_count: 0, eligible: true, reason: "" };
    mockInvoke.mockResolvedValue(preview);
    const result = await pruneGamePreview("/games/g1");
    expect(mockInvoke).toHaveBeenCalledWith("prune_game_preview", {
      gameDir: "/games/g1",
      allFiles: false,
      force: null,
    });
    expect(result).toEqual(preview);
  });

  it("passes force=true when specified", async () => {
    const preview = { files: [], total_bytes: 0, file_count: 0, eligible: true, reason: "" };
    mockInvoke.mockResolvedValue(preview);
    await pruneGamePreview("/games/g1", false, true);
    expect(mockInvoke).toHaveBeenCalledWith("prune_game_preview", {
      gameDir: "/games/g1",
      allFiles: false,
      force: true,
    });
  });

  it("passes allFiles=true", async () => {
    const preview = { files: [], total_bytes: 100, file_count: 1, eligible: true, reason: "" };
    mockInvoke.mockResolvedValue(preview);
    await pruneGamePreview("/games/g1", true);
    expect(mockInvoke).toHaveBeenCalledWith("prune_game_preview", {
      gameDir: "/games/g1",
      allFiles: true,
      force: null,
    });
  });
});

describe("pruneGameExecute", () => {
  it("passes gameDir, allFiles, and force", async () => {
    const result = { files: [], total_bytes: 500, file_count: 3, eligible: true, reason: "" };
    mockInvoke.mockResolvedValue(result);
    const res = await pruneGameExecute("/games/g1", true, true);
    expect(mockInvoke).toHaveBeenCalledWith("prune_game_execute", {
      gameDir: "/games/g1",
      allFiles: true,
      force: true,
    });
    expect(res).toEqual(result);
  });
});

describe("deleteGamePreview", () => {
  it("passes gameDir and returns preview", async () => {
    const preview = {
      dir_path: "/games/g1",
      home_team: "Team A",
      away_team: "Team B",
      date: "2026-04-12",
      total_bytes: 1024,
      file_count: 5,
    };
    mockInvoke.mockResolvedValue(preview);
    const result = await deleteGamePreview("/games/g1");
    expect(mockInvoke).toHaveBeenCalledWith("delete_game_preview", { gameDir: "/games/g1" });
    expect(result).toEqual(preview);
  });
});

describe("deleteGame", () => {
  it("passes gameDir and returns void", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteGame("/games/g1");
    expect(mockInvoke).toHaveBeenCalledWith("delete_game", { gameDir: "/games/g1" });
  });
});
