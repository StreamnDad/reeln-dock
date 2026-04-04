import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  listConfigProfiles,
  listPluginsForProfile,
  togglePluginInConfig,
  updatePluginInConfig,
  fetchPluginRegistry,
  addPluginToConfig,
  removePluginFromConfig,
  createConfigProfile,
  getVersionInfo,
  installPluginViaCli,
  getEnforceHooks,
  setEnforceHooks,
} from "./plugins";

const mockInvoke = invoke as Mock;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("listConfigProfiles", () => {
  it("invokes list_config_profiles", async () => {
    const profiles = [{ name: "default", path: "/config.json", active: true }];
    mockInvoke.mockResolvedValue(profiles);
    const result = await listConfigProfiles();
    expect(mockInvoke).toHaveBeenCalledWith("list_config_profiles");
    expect(result).toEqual(profiles);
  });
});

describe("listPluginsForProfile", () => {
  it("passes profilePath", async () => {
    mockInvoke.mockResolvedValue([]);
    await listPluginsForProfile("/config/google.json");
    expect(mockInvoke).toHaveBeenCalledWith("list_plugins_for_profile", {
      profilePath: "/config/google.json",
    });
  });
});

describe("togglePluginInConfig", () => {
  it("passes profilePath and pluginName", async () => {
    mockInvoke.mockResolvedValue([]);
    await togglePluginInConfig("/config.json", "openai");
    expect(mockInvoke).toHaveBeenCalledWith("toggle_plugin_in_config", {
      profilePath: "/config.json",
      pluginName: "openai",
    });
  });
});

describe("updatePluginInConfig", () => {
  it("passes profilePath, pluginName, and settings object", async () => {
    const settings = { api_key: "test", model: "gpt-4" };
    mockInvoke.mockResolvedValue([]);
    await updatePluginInConfig("/config.json", "openai", settings);
    expect(mockInvoke).toHaveBeenCalledWith("update_plugin_in_config", {
      profilePath: "/config.json",
      pluginName: "openai",
      settings,
    });
  });
});

describe("fetchPluginRegistry", () => {
  it("invokes fetch_plugin_registry", async () => {
    mockInvoke.mockResolvedValue([{ name: "youtube" }]);
    const result = await fetchPluginRegistry();
    expect(mockInvoke).toHaveBeenCalledWith("fetch_plugin_registry");
    expect(result).toEqual([{ name: "youtube" }]);
  });
});

describe("addPluginToConfig", () => {
  it("passes profilePath and pluginName", async () => {
    mockInvoke.mockResolvedValue([]);
    await addPluginToConfig("/config.json", "youtube");
    expect(mockInvoke).toHaveBeenCalledWith("add_plugin_to_config", {
      profilePath: "/config.json",
      pluginName: "youtube",
    });
  });
});

describe("removePluginFromConfig", () => {
  it("passes profilePath and pluginName", async () => {
    mockInvoke.mockResolvedValue([]);
    await removePluginFromConfig("/config.json", "youtube");
    expect(mockInvoke).toHaveBeenCalledWith("remove_plugin_from_config", {
      profilePath: "/config.json",
      pluginName: "youtube",
    });
  });
});

describe("createConfigProfile", () => {
  it("passes profileName", async () => {
    mockInvoke.mockResolvedValue([]);
    await createConfigProfile("tournament-2026");
    expect(mockInvoke).toHaveBeenCalledWith("create_config_profile", {
      profileName: "tournament-2026",
    });
  });
});

describe("getVersionInfo", () => {
  it("invokes get_version_info", async () => {
    const info = { app_version: "0.1.0", config_version: 2 };
    mockInvoke.mockResolvedValue(info);
    const result = await getVersionInfo();
    expect(mockInvoke).toHaveBeenCalledWith("get_version_info");
    expect(result).toEqual(info);
  });
});

describe("installPluginViaCli", () => {
  it("passes pluginName to invoke", async () => {
    const installResult = { success: true, output: "installed" };
    mockInvoke.mockResolvedValue(installResult);
    const result = await installPluginViaCli("reeln-youtube");
    expect(mockInvoke).toHaveBeenCalledWith("install_plugin_via_cli", {
      pluginName: "reeln-youtube",
    });
    expect(result).toEqual(installResult);
  });
});

describe("getEnforceHooks", () => {
  it("passes profilePath", async () => {
    mockInvoke.mockResolvedValue(true);
    const result = await getEnforceHooks("/config.json");
    expect(mockInvoke).toHaveBeenCalledWith("get_enforce_hooks", {
      profilePath: "/config.json",
    });
    expect(result).toBe(true);
  });
});

describe("setEnforceHooks", () => {
  it("passes profilePath and enforce boolean", async () => {
    mockInvoke.mockResolvedValue(false);
    await setEnforceHooks("/config.json", false);
    expect(mockInvoke).toHaveBeenCalledWith("set_enforce_hooks", {
      profilePath: "/config.json",
      enforce: false,
    });
  });
});
