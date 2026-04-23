import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";
import type { RegistryPlugin } from "$lib/types/plugin";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("$lib/ipc/plugins", () => ({
  fetchPluginRegistry: vi.fn(),
  listConfigProfiles: vi.fn(),
  listPluginsForProfile: vi.fn(),
}));

vi.mock("$lib/stores/cli.svelte", () => ({
  isPluginInstalled: vi.fn(),
  isCliAvailable: vi.fn(),
}));

vi.mock("$lib/stores/log.svelte", () => ({
  log: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

import {
  fetchPluginRegistry,
  listConfigProfiles,
  listPluginsForProfile,
} from "$lib/ipc/plugins";
import { isPluginInstalled, isCliAvailable } from "$lib/stores/cli.svelte";

const mockFetchRegistry = fetchPluginRegistry as Mock;
const mockListProfiles = listConfigProfiles as Mock;
const mockListPlugins = listPluginsForProfile as Mock;
const mockIsInstalled = isPluginInstalled as Mock;
const mockIsCliAvailable = isCliAvailable as Mock;

// Registry fixtures
const openaiPlugin = {
  name: "openai",
  package: "reeln-openai",
  description: "AI features",
  capabilities: ["MetadataEnricher"],
  homepage: "https://example.com",
  min_reeln_version: "0.1.0",
  author: "test",
  license: "MIT",
  ui_contributions: {
    render_options: {
      fields: [
        { id: "smart", label: "Smart Zoom", type: "boolean" as const, default: false },
        { id: "zoom_frames", label: "Zoom Frames", type: "number" as const, min: 1, max: 30 },
      ],
    },
    clip_review: {
      fields: [
        { id: "auto_tag", label: "Auto Tag", type: "boolean" as const },
      ],
    },
  },
};

const youtubePlugin = {
  name: "youtube",
  package: "reeln-youtube",
  description: "YouTube upload",
  capabilities: ["Uploader"],
  homepage: "https://example.com",
  min_reeln_version: "0.1.0",
  author: "test",
  license: "MIT",
  ui_contributions: {
    settings: {
      fields: [
        { id: "channel", label: "Channel", type: "string" as const },
      ],
    },
  },
};

const noUiPlugin = {
  name: "discord",
  package: "reeln-discord",
  description: "Discord notifier",
  capabilities: ["Notifier"],
  homepage: "https://example.com",
  min_reeln_version: "0.1.0",
  author: "test",
  license: "MIT",
  // no ui_contributions
};

const emptyFieldsPlugin = {
  name: "empty",
  package: "reeln-empty",
  description: "Empty",
  capabilities: [],
  homepage: "",
  min_reeln_version: "0.1.0",
  author: "test",
  license: "MIT",
  ui_contributions: {
    render_options: { fields: [] }, // empty fields
  },
};

describe("pluginUI store", () => {
  beforeEach(() => {
    vi.resetModules();
    mockFetchRegistry.mockReset();
    mockListProfiles.mockReset();
    mockListPlugins.mockReset();
    mockIsInstalled.mockReset();
    mockIsCliAvailable.mockReset();
  });

  async function setupStore(opts: {
    registry?: RegistryPlugin[];
    enabledNames?: string[];
    cliAvailable?: boolean;
    installedNames?: string[];
  }) {
    const {
      registry = [],
      enabledNames = [],
      cliAvailable = false,
      installedNames = [],
    } = opts;

    mockFetchRegistry.mockResolvedValue(registry);
    mockListProfiles.mockResolvedValue([
      { name: "default", path: "/config.json", active: true },
    ]);
    mockListPlugins.mockResolvedValue(
      enabledNames.map((name) => ({ name, enabled: true, settings: {} })),
    );
    mockIsCliAvailable.mockReturnValue(cliAvailable);
    mockIsInstalled.mockImplementation((name: string) =>
      installedNames.includes(name),
    );

    const store = await import("./pluginUI.svelte");
    await store.initPluginUI();
    return store;
  }

  describe("getActiveFieldsForScreen", () => {
    it("returns fields for enabled + installed plugins", async () => {
      const store = await setupStore({
        registry: [openaiPlugin, youtubePlugin],
        enabledNames: ["openai", "youtube"],
        cliAvailable: true,
        installedNames: ["openai", "youtube"],
      });

      const renderFields = store.getActiveFieldsForScreen("render_options");
      expect(renderFields).toHaveLength(1);
      expect(renderFields[0].pluginName).toBe("openai");
      expect(renderFields[0].fields).toHaveLength(2);
      expect(renderFields[0].fields[0].id).toBe("smart");

      const settingsFields = store.getActiveFieldsForScreen("settings");
      expect(settingsFields).toHaveLength(1);
      expect(settingsFields[0].pluginName).toBe("youtube");
    });

    it("excludes plugins that are not enabled", async () => {
      const store = await setupStore({
        registry: [openaiPlugin, youtubePlugin],
        enabledNames: ["youtube"], // openai NOT enabled
        cliAvailable: true,
        installedNames: ["openai", "youtube"],
      });

      const renderFields = store.getActiveFieldsForScreen("render_options");
      expect(renderFields).toHaveLength(0); // openai excluded
    });

    it("excludes plugins not installed when CLI is available", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
        cliAvailable: true,
        installedNames: [], // NOT installed
      });

      const renderFields = store.getActiveFieldsForScreen("render_options");
      expect(renderFields).toHaveLength(0);
    });

    it("includes enabled plugins when CLI is NOT available (no install check)", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
        cliAvailable: false, // CLI not available — skip install check
        installedNames: [],
      });

      const renderFields = store.getActiveFieldsForScreen("render_options");
      expect(renderFields).toHaveLength(1);
      expect(renderFields[0].pluginName).toBe("openai");
    });

    it("excludes plugins with no ui_contributions", async () => {
      const store = await setupStore({
        registry: [noUiPlugin],
        enabledNames: ["discord"],
        cliAvailable: false,
      });

      const renderFields = store.getActiveFieldsForScreen("render_options");
      expect(renderFields).toHaveLength(0);

      const settingsFields = store.getActiveFieldsForScreen("settings");
      expect(settingsFields).toHaveLength(0);
    });

    it("excludes plugins with empty fields array for the screen", async () => {
      const store = await setupStore({
        registry: [emptyFieldsPlugin],
        enabledNames: ["empty"],
        cliAvailable: false,
      });

      const fields = store.getActiveFieldsForScreen("render_options");
      expect(fields).toHaveLength(0);
    });

    it("returns empty for screen with no contributions", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
        cliAvailable: false,
      });

      // openai has render_options and clip_review, but NOT settings
      const settingsFields = store.getActiveFieldsForScreen("settings");
      expect(settingsFields).toHaveLength(0);
    });

    it("returns clip_review fields correctly", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
        cliAvailable: false,
      });

      const clipFields = store.getActiveFieldsForScreen("clip_review");
      expect(clipFields).toHaveLength(1);
      expect(clipFields[0].fields[0].id).toBe("auto_tag");
    });
  });

  describe("isPluginEnabled", () => {
    it("returns true for enabled plugins", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
        cliAvailable: false,
      });

      expect(store.isPluginEnabled("openai")).toBe(true);
      expect(store.isPluginEnabled("youtube")).toBe(false);
    });
  });

  describe("initPluginUI", () => {
    it("is idempotent", async () => {
      const store = await setupStore({
        registry: [openaiPlugin],
        enabledNames: ["openai"],
      });

      // Call again — should not re-fetch
      mockFetchRegistry.mockClear();
      await store.initPluginUI();
      expect(mockFetchRegistry).not.toHaveBeenCalled();
    });

    it("handles refreshEnabledPlugins failure gracefully", async () => {
      mockFetchRegistry.mockResolvedValue([openaiPlugin]);
      // listConfigProfiles rejects → catch branch in refreshEnabledPlugins
      mockListProfiles.mockRejectedValue(new Error("network error"));

      const store = await import("./pluginUI.svelte");
      await store.initPluginUI();

      // Should not throw, but no plugins enabled
      expect(store.isPluginEnabled("openai")).toBe(false);
    });

    it("handles registry fetch failure gracefully", async () => {
      mockFetchRegistry.mockRejectedValue(new Error("network error"));
      mockListProfiles.mockResolvedValue([]);

      const store = await import("./pluginUI.svelte");
      await store.initPluginUI();

      // Should not throw, fields should be empty
      const fields = store.getActiveFieldsForScreen("render_options");
      expect(fields).toEqual([]);
    });
  });
});
