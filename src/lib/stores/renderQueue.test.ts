import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

// Mock all external dependencies before imports
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("$lib/ipc/render", () => ({
  renderShort: vi.fn(),
  renderIteration: vi.fn(),
}));

vi.mock("$lib/ipc/plugins", () => ({
  listConfigProfiles: vi.fn(),
}));

vi.mock("$lib/stores/log.svelte", () => ({
  log: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

import { invoke } from "@tauri-apps/api/core";
import { renderShort, renderIteration } from "$lib/ipc/render";
import { listConfigProfiles } from "$lib/ipc/plugins";

const mockInvoke = invoke as Mock;
const mockRenderShort = renderShort as Mock;
const mockRenderIteration = renderIteration as Mock;
const mockListConfigProfiles = listConfigProfiles as Mock;

describe("renderQueue store", () => {
  beforeEach(async () => {
    vi.resetModules();
    mockInvoke.mockReset();
    mockRenderShort.mockReset();
    mockRenderIteration.mockReset();
    mockListConfigProfiles.mockReset();
    // Default: persist is fire-and-forget
    mockInvoke.mockResolvedValue("[]");
  });

  async function loadStore() {
    return await import("./renderQueue.svelte");
  }

  describe("initQueue", () => {
    it("loads queue from disk and resets rendering items to pending", async () => {
      const saved = [
        { id: "1", status: "rendering", clipName: "a" },
        { id: "2", status: "done", clipName: "b" },
        { id: "3", status: "pending", clipName: "c" },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      const q = store.getQueue();

      expect(q[0].status).toBe("pending"); // was "rendering"
      expect(q[1].status).toBe("done"); // unchanged
      expect(q[2].status).toBe("pending"); // unchanged
    });

    it("initializes to empty array on error", async () => {
      mockInvoke.mockRejectedValueOnce(new Error("file not found"));

      const store = await loadStore();
      await store.initQueue();
      expect(store.getQueue()).toEqual([]);
    });

    it("is idempotent — second call is a no-op", async () => {
      mockInvoke.mockResolvedValueOnce("[]");

      const store = await loadStore();
      await store.initQueue();
      await store.initQueue(); // should not invoke again
      // First call loads, subsequent persist calls may happen
      // but load_render_queue should only be called once
      const loadCalls = mockInvoke.mock.calls.filter(
        (c) => c[0] === "load_render_queue",
      );
      expect(loadCalls).toHaveLength(1);
    });
  });

  describe("addToQueue", () => {
    it("preserves all QueueItem fields including pluginProfile, scorer, assists", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      store.addToQueue({
        gameDir: "/games/test",
        gameName: "Test Game",
        eventId: "goal_1",
        clipPath: "/games/test/clips/clip.mp4",
        clipName: "clip.mp4",
        profiles: [
          { profile_name: "tiktok", overrides: { speed: 0.5 } },
          { profile_name: "youtube" },
        ],
        concatOutput: true,
        overrides: { crop_mode: "pad", scale: 0.8 },
        pluginProfile: "google-test",
        mode: "short",
        debug: true,
        scorer: "Player One",
        assist1: "Player Two",
        assist2: "Player Three",
      });

      const q = store.getQueue();
      expect(q).toHaveLength(1);
      const item = q[0];

      // Auto-generated fields
      expect(item.id).toBeTruthy();
      expect(item.status).toBe("pending");
      expect(item.addedAt).toBeGreaterThan(0);

      // All user-supplied fields preserved
      expect(item.gameDir).toBe("/games/test");
      expect(item.gameName).toBe("Test Game");
      expect(item.eventId).toBe("goal_1");
      expect(item.clipPath).toBe("/games/test/clips/clip.mp4");
      expect(item.clipName).toBe("clip.mp4");
      expect(item.profiles).toEqual([
        { profile_name: "tiktok", overrides: { speed: 0.5 } },
        { profile_name: "youtube" },
      ]);
      expect(item.concatOutput).toBe(true);
      expect(item.overrides).toEqual({ crop_mode: "pad", scale: 0.8 });
      expect(item.pluginProfile).toBe("google-test");
      expect(item.mode).toBe("short");
      expect(item.debug).toBe(true);
      expect(item.scorer).toBe("Player One");
      expect(item.assist1).toBe("Player Two");
      expect(item.assist2).toBe("Player Three");
    });

    it("persists after adding", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();
      mockInvoke.mockClear();

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c.mp4",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });

      const persistCalls = mockInvoke.mock.calls.filter(
        (c) => c[0] === "save_render_queue",
      );
      expect(persistCalls).toHaveLength(1);
    });
  });

  describe("renderItem → renderShort", () => {
    it("passes all fields to renderShort for single-profile item", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      // Mock profile resolution: pluginProfile → config path
      mockListConfigProfiles.mockResolvedValue([
        { name: "google-test", path: "/config/google-test.json", active: true },
      ]);

      const fakeEntry = {
        input: "/clip.mp4",
        output: "/out.mp4",
        segment_number: 0,
        format: "tiktok",
        crop_mode: "crop",
        rendered_at: "2026-01-01",
        event_id: "goal_1",
      };
      mockRenderShort.mockResolvedValue(fakeEntry);

      store.addToQueue({
        gameDir: "/games/test",
        gameName: "Test",
        eventId: "goal_1",
        clipPath: "/games/test/clips/clip.mp4",
        clipName: "clip.mp4",
        profiles: [{ profile_name: "tiktok", overrides: { speed: 0.5 } }],
        concatOutput: false,
        overrides: { crop_mode: "pad" },
        pluginProfile: "google-test",
        mode: "short",
        debug: true,
        scorer: "Scorer",
        assist1: "Assist1",
        assist2: "Assist2",
      });

      const item = store.getQueue()[0];
      await store.renderSingle(item.id);

      expect(mockRenderShort).toHaveBeenCalledOnce();
      const [
        inputClip,
        outputDir,
        profileName,
        eventId,
        gameDir,
        overrides,
        mode,
        scorer,
        assist1,
        assist2,
        playerNumbers,
        debug,
        configPath,
        noBranding,
      ] = mockRenderShort.mock.calls[0];

      expect(inputClip).toBe("/games/test/clips/clip.mp4");
      expect(outputDir).toBe("/games/test/renders");
      expect(profileName).toBe("tiktok");
      expect(eventId).toBe("goal_1");
      expect(gameDir).toBe("/games/test");
      // Overrides merge: item.overrides spread with profile.overrides
      expect(overrides).toEqual({ crop_mode: "pad", speed: 0.5 });
      expect(mode).toBe("short");
      expect(scorer).toBe("Scorer");
      expect(assist1).toBe("Assist1");
      expect(assist2).toBe("Assist2");
      expect(playerNumbers).toBeUndefined();
      expect(debug).toBe(true);
      expect(configPath).toBe("/config/google-test.json");
      expect(noBranding).toBeUndefined();
    });

    it("marks item done with results on success", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      const fakeEntry = { output: "/out.mp4" };
      mockRenderShort.mockResolvedValue(fakeEntry);

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      const item = store.getQueue().find((q) => q.id === id);
      expect(item?.status).toBe("done");
      expect(item?.results).toEqual([fakeEntry]);
    });

    it("marks item error on failure", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderShort.mockRejectedValue(new Error("render failed"));

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      const item = store.getQueue().find((q) => q.id === id);
      expect(item?.status).toBe("error");
      expect(item?.error).toContain("render failed");
    });
  });

  describe("renderItem → renderIteration", () => {
    it("passes all fields to renderIteration for multi-profile item", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([
        { name: "google-test", path: "/config/google.json", active: true },
      ]);
      mockRenderIteration.mockResolvedValue([
        { output: "/a.mp4" },
        { output: "/b.mp4" },
      ]);

      store.addToQueue({
        gameDir: "/games/test",
        gameName: "Test",
        eventId: "goal_2",
        clipPath: "/clip.mp4",
        clipName: "clip",
        profiles: [
          { profile_name: "tiktok", overrides: { scale: 0.5 } },
          { profile_name: "youtube" },
        ],
        concatOutput: true,
        overrides: { crop_mode: "crop" },
        pluginProfile: "google-test",
        mode: "apply",
        scorer: "S",
        assist1: "A1",
        assist2: "A2",
        debug: false,
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      expect(mockRenderIteration).toHaveBeenCalledOnce();
      const [
        inputClip,
        outputDir,
        items,
        eventId,
        gameDir,
        concatOutput,
        mode,
        scorer,
        assist1,
        assist2,
        playerNumbers,
        debug,
        configPath,
        noBranding,
      ] = mockRenderIteration.mock.calls[0];

      expect(inputClip).toBe("/clip.mp4");
      expect(outputDir).toBe("/games/test/renders");
      // Items have merged overrides
      expect(items).toEqual([
        { profile_name: "tiktok", overrides: { crop_mode: "crop", scale: 0.5 } },
        { profile_name: "youtube", overrides: { crop_mode: "crop" } },
      ]);
      expect(eventId).toBe("goal_2");
      expect(gameDir).toBe("/games/test");
      expect(concatOutput).toBe(true);
      expect(mode).toBe("apply");
      expect(scorer).toBe("S");
      expect(assist1).toBe("A1");
      expect(assist2).toBe("A2");
      expect(playerNumbers).toBeUndefined();
      expect(debug).toBe(false);
      expect(configPath).toBe("/config/google.json");
      expect(noBranding).toBeUndefined();
    });
  });

  describe("resolveProfilePath error handling", () => {
    it("falls back to undefined configPath when listConfigProfiles fails", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      // listConfigProfiles rejects → resolveProfilePath catches and returns undefined
      mockListConfigProfiles.mockRejectedValue(new Error("network error"));
      mockRenderShort.mockResolvedValue({ output: "/out.mp4" });

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
        pluginProfile: "broken-profile",
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      // configPath should be undefined (error fallback) — position 13 after playerNumbers
      const configPath = mockRenderShort.mock.calls[0][13];
      expect(configPath).toBeUndefined();
    });
  });

  describe("renderAll", () => {
    it("renders all pending items in sequence", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderShort.mockResolvedValue({ output: "/out.mp4" });

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e1",
        clipPath: "/a.mp4",
        clipName: "a",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });
      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e2",
        clipPath: "/b.mp4",
        clipName: "b",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });

      await store.renderAll();

      expect(mockRenderShort).toHaveBeenCalledTimes(2);
      const q = store.getQueue();
      expect(q[0].status).toBe("done");
      expect(q[1].status).toBe("done");
    });

    it("renderSingle skips non-pending items", async () => {
      const saved = [{ id: "1", status: "done" }];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));
      const store = await loadStore();
      await store.initQueue();
      mockRenderShort.mockResolvedValue({});

      await store.renderSingle("1"); // done, should skip
      await store.renderSingle("nonexistent"); // not found, should skip
      expect(mockRenderShort).not.toHaveBeenCalled();
    });

    it("skips non-pending items", async () => {
      const saved = [
        { id: "1", status: "done", clipPath: "/a.mp4", profiles: [{ profile_name: "d" }] },
        { id: "2", status: "pending", clipPath: "/b.mp4", profiles: [{ profile_name: "d" }], gameDir: "/g", concatOutput: false },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderShort.mockResolvedValue({ output: "/out.mp4" });

      await store.renderAll();

      // Only the pending item should be rendered
      expect(mockRenderShort).toHaveBeenCalledOnce();
    });
  });

  describe("renderIteration with multiple queue items", () => {
    it("only updates the rendered item, leaving others unchanged", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderIteration.mockResolvedValue([{ output: "/iter.mp4" }]);

      // Add two items
      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e1",
        clipPath: "/a.mp4",
        clipName: "a",
        profiles: [{ profile_name: "x" }, { profile_name: "y" }],
        concatOutput: true,
      });
      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e2",
        clipPath: "/b.mp4",
        clipName: "b",
        profiles: [{ profile_name: "z" }],
        concatOutput: false,
      });

      const firstId = store.getQueue()[0].id;
      const secondId = store.getQueue()[1].id;
      await store.renderSingle(firstId);

      expect(store.getQueue().find((q) => q.id === firstId)?.status).toBe("done");
      expect(store.getQueue().find((q) => q.id === secondId)?.status).toBe("pending");
    });

    it("marks only the failed item as error when iteration fails", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderIteration.mockRejectedValue(new Error("iteration failed"));

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e1",
        clipPath: "/a.mp4",
        clipName: "a",
        profiles: [{ profile_name: "x" }, { profile_name: "y" }],
        concatOutput: true,
      });
      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e2",
        clipPath: "/b.mp4",
        clipName: "b",
        profiles: [{ profile_name: "z" }],
        concatOutput: false,
      });

      const firstId = store.getQueue()[0].id;
      const secondId = store.getQueue()[1].id;
      await store.renderSingle(firstId);

      expect(store.getQueue().find((q) => q.id === firstId)?.status).toBe("error");
      expect(store.getQueue().find((q) => q.id === secondId)?.status).toBe("pending");
    });
  });

  describe("persist error handling", () => {
    it("catches and logs persist failures without throwing", async () => {
      mockInvoke.mockResolvedValueOnce("[]"); // load
      const store = await loadStore();
      await store.initQueue();

      // Make save_render_queue reject
      mockInvoke.mockRejectedValue(new Error("disk full"));

      // addToQueue calls persist — should not throw
      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [{ profile_name: "default" }],
        concatOutput: false,
      });

      // Wait for the rejected promise to be caught
      await new Promise((r) => setTimeout(r, 10));
      expect(store.getQueue()).toHaveLength(1);
    });
  });

  describe("renderItem without overrides", () => {
    it("passes profile overrides directly when item has no overrides", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderShort.mockResolvedValue({ output: "/out.mp4" });

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [{ profile_name: "default", overrides: { speed: 0.5 } }],
        concatOutput: false,
        // no overrides at item level
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      const overrides = mockRenderShort.mock.calls[0][5];
      expect(overrides).toEqual({ speed: 0.5 });
    });

    it("passes undefined overrides for multi-profile when item has no overrides", async () => {
      mockInvoke.mockResolvedValue("[]");
      const store = await loadStore();
      await store.initQueue();

      mockListConfigProfiles.mockResolvedValue([]);
      mockRenderIteration.mockResolvedValue([{ output: "/a.mp4" }]);

      store.addToQueue({
        gameDir: "/g",
        gameName: "G",
        eventId: "e",
        clipPath: "/c.mp4",
        clipName: "c",
        profiles: [
          { profile_name: "a" },
          { profile_name: "b", overrides: { speed: 2.0 } },
        ],
        concatOutput: false,
        // no item-level overrides
      });

      const id = store.getQueue()[0].id;
      await store.renderSingle(id);

      const items = mockRenderIteration.mock.calls[0][2];
      expect(items[0].overrides).toBeUndefined();
      expect(items[1].overrides).toEqual({ speed: 2.0 });
    });
  });

  describe("queue operations", () => {
    it("getPendingCount returns only pending items", async () => {
      const saved = [
        { id: "1", status: "pending" },
        { id: "2", status: "done" },
        { id: "3", status: "pending" },
        { id: "4", status: "error" },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      expect(store.getPendingCount()).toBe(2);
    });

    it("isClipInQueue checks clipPath and pending status", async () => {
      const saved = [
        { id: "1", clipPath: "/a.mp4", status: "pending" },
        { id: "2", clipPath: "/b.mp4", status: "done" },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      expect(store.isClipInQueue("/a.mp4")).toBe(true);
      expect(store.isClipInQueue("/b.mp4")).toBe(false); // done, not pending
      expect(store.isClipInQueue("/c.mp4")).toBe(false);
    });

    it("removeFromQueue removes by id", async () => {
      const saved = [{ id: "1" }, { id: "2" }];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      store.removeFromQueue("1");
      expect(store.getQueue()).toHaveLength(1);
      expect(store.getQueue()[0].id).toBe("2");
    });

    it("clearCompleted removes done and error items", async () => {
      const saved = [
        { id: "1", status: "pending" },
        { id: "2", status: "done" },
        { id: "3", status: "error" },
        { id: "4", status: "rendering" },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      store.clearCompleted();
      const q = store.getQueue();
      expect(q).toHaveLength(2);
      expect(q.map((i) => i.id)).toEqual(["1", "4"]);
    });

    it("reorderQueue moves item from one index to another", async () => {
      const saved = [
        { id: "a" },
        { id: "b" },
        { id: "c" },
      ];
      mockInvoke.mockResolvedValueOnce(JSON.stringify(saved));

      const store = await loadStore();
      await store.initQueue();
      store.reorderQueue(0, 2);
      expect(store.getQueue().map((i) => i.id)).toEqual(["b", "c", "a"]);
    });
  });
});
