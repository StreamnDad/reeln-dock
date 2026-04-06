import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  loadDockSettings,
  saveDockSettings,
  loadConfigFromPath,
  getConfigPath,
  saveEventTypes,
  saveRenderProfile,
  deleteRenderProfile,
  renameRenderProfile,
} from "./config";

const mockInvoke = invoke as Mock;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("loadDockSettings", () => {
  it("invokes load_dock_settings", async () => {
    const fake = { settings: {}, config: null };
    mockInvoke.mockResolvedValue(fake);
    const result = await loadDockSettings();
    expect(mockInvoke).toHaveBeenCalledWith("load_dock_settings");
    expect(result).toEqual(fake);
  });
});

describe("saveDockSettings", () => {
  it("passes settings to invoke", async () => {
    const settings = { reeln_config_path: "/config.json" };
    const fake = { settings, config: null };
    mockInvoke.mockResolvedValue(fake);
    const result = await saveDockSettings(settings as never);
    expect(mockInvoke).toHaveBeenCalledWith("save_dock_settings", { settings });
    expect(result).toEqual(fake);
  });
});

describe("loadConfigFromPath", () => {
  it("passes path to invoke", async () => {
    const fake = { config: { config_version: 1 }, path: "/config.json" };
    mockInvoke.mockResolvedValue(fake);
    const result = await loadConfigFromPath("/config.json");
    expect(mockInvoke).toHaveBeenCalledWith("load_config_from_path", {
      path: "/config.json",
    });
    expect(result).toEqual(fake);
  });
});

describe("getConfigPath", () => {
  it("passes null when no profile", async () => {
    mockInvoke.mockResolvedValue("/default/config.json");
    const result = await getConfigPath();
    expect(mockInvoke).toHaveBeenCalledWith("get_config_path", {
      profile: null,
    });
    expect(result).toBe("/default/config.json");
  });

  it("passes profile string", async () => {
    mockInvoke.mockResolvedValue("/config.production.json");
    const result = await getConfigPath("production");
    expect(mockInvoke).toHaveBeenCalledWith("get_config_path", {
      profile: "production",
    });
    expect(result).toBe("/config.production.json");
  });
});

describe("saveEventTypes", () => {
  it("passes event types to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const types = [{ name: "goal", team_specific: true }];
    await saveEventTypes(types);
    expect(mockInvoke).toHaveBeenCalledWith("save_event_types", {
      eventTypes: types,
    });
  });
});

describe("saveRenderProfile", () => {
  it("passes profile key and data to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const profile = { name: "tiktok", width: 1080, height: 1920 };
    await saveRenderProfile("tiktok", profile);
    expect(mockInvoke).toHaveBeenCalledWith("save_render_profile", {
      profileKey: "tiktok",
      profile,
    });
  });

  it("handles invoke error", async () => {
    mockInvoke.mockRejectedValue(new Error("No config path set"));
    await expect(saveRenderProfile("test", {})).rejects.toThrow(
      "No config path set",
    );
  });
});

describe("deleteRenderProfile", () => {
  it("passes profile key to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deleteRenderProfile("old-profile");
    expect(mockInvoke).toHaveBeenCalledWith("delete_render_profile", {
      profileKey: "old-profile",
    });
  });

  it("handles invoke error for missing profile", async () => {
    mockInvoke.mockRejectedValue(new Error("Profile 'nonexistent' not found"));
    await expect(deleteRenderProfile("nonexistent")).rejects.toThrow(
      "not found",
    );
  });
});

describe("renameRenderProfile", () => {
  it("passes old and new keys to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await renameRenderProfile("old-name", "new-name");
    expect(mockInvoke).toHaveBeenCalledWith("rename_render_profile", {
      oldKey: "old-name",
      newKey: "new-name",
    });
  });

  it("handles invoke error for duplicate name", async () => {
    mockInvoke.mockRejectedValue(
      new Error("Profile 'existing' already exists"),
    );
    await expect(
      renameRenderProfile("old", "existing"),
    ).rejects.toThrow("already exists");
  });
});
