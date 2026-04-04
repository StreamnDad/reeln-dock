import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  renderShort,
  renderIteration,
  getIterationProfiles,
  renderPreview,
  deletePreview,
  renderReel,
  listRenderProfiles,
} from "./render";

const mockInvoke = invoke as Mock;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("renderShort", () => {
  it("passes all parameters to invoke", async () => {
    const fakeEntry = {
      input: "/clip.mp4",
      output: "/out.mp4",
      segment_number: 0,
      format: "tiktok",
      crop_mode: "crop",
      rendered_at: "2026-01-01",
      event_id: "goal_1",
    };
    mockInvoke.mockResolvedValue(fakeEntry);

    const overrides = { crop_mode: "pad", scale: 0.5, smart: true, zoom_frames: 10 };
    const result = await renderShort(
      "/games/clip.mp4",
      "/games/renders",
      "tiktok",
      "goal_1",
      "/games/test",
      overrides,
      "short",
      "player1",
      "assist1",
      "assist2",
      true,
      "/config/google.json",
    );

    expect(mockInvoke).toHaveBeenCalledOnce();
    expect(mockInvoke).toHaveBeenCalledWith("render_short", {
      inputClip: "/games/clip.mp4",
      outputDir: "/games/renders",
      profileName: "tiktok",
      eventId: "goal_1",
      gameDir: "/games/test",
      overrides,
      mode: "short",
      scorer: "player1",
      assist1: "assist1",
      assist2: "assist2",
      debug: true,
      configPath: "/config/google.json",
    });
    expect(result).toEqual(fakeEntry);
  });

  it("normalizes undefined optional params to null", async () => {
    mockInvoke.mockResolvedValue({});

    await renderShort("/clip.mp4", "/out", "default");

    expect(mockInvoke).toHaveBeenCalledWith("render_short", {
      inputClip: "/clip.mp4",
      outputDir: "/out",
      profileName: "default",
      eventId: undefined,
      gameDir: undefined,
      overrides: null,
      mode: null,
      scorer: null,
      assist1: null,
      assist2: null,
      debug: null,
      configPath: null,
    });
  });
});

describe("renderIteration", () => {
  it("passes all parameters including items array and configPath", async () => {
    const fakeEntries = [{ input: "/clip.mp4", output: "/out.mp4" }];
    mockInvoke.mockResolvedValue(fakeEntries);

    const items = [
      { profile_name: "tiktok", overrides: { speed: 0.5 } },
      { profile_name: "youtube" },
    ];

    const result = await renderIteration(
      "/clip.mp4",
      "/renders",
      items,
      "goal_1",
      "/games/test",
      true,
      "apply",
      "scorer1",
      "a1",
      "a2",
      false,
      "/config/profile.json",
    );

    expect(mockInvoke).toHaveBeenCalledWith("render_iteration", {
      inputClip: "/clip.mp4",
      outputDir: "/renders",
      items,
      eventId: "goal_1",
      gameDir: "/games/test",
      concatOutput: true,
      mode: "apply",
      scorer: "scorer1",
      assist1: "a1",
      assist2: "a2",
      debug: false,
      configPath: "/config/profile.json",
    });
    expect(result).toEqual(fakeEntries);
  });

  it("normalizes undefined optional params to null", async () => {
    mockInvoke.mockResolvedValue([]);

    await renderIteration("/clip.mp4", "/out", [{ profile_name: "default" }]);

    expect(mockInvoke).toHaveBeenCalledWith("render_iteration", {
      inputClip: "/clip.mp4",
      outputDir: "/out",
      items: [{ profile_name: "default" }],
      eventId: undefined,
      gameDir: undefined,
      concatOutput: true,
      mode: null,
      scorer: null,
      assist1: null,
      assist2: null,
      debug: null,
      configPath: null,
    });
  });
});

describe("getIterationProfiles", () => {
  it("passes eventType to invoke", async () => {
    mockInvoke.mockResolvedValue(["tiktok", "youtube"]);
    const result = await getIterationProfiles("goal");
    expect(mockInvoke).toHaveBeenCalledWith("get_iteration_profiles", {
      eventType: "goal",
    });
    expect(result).toEqual(["tiktok", "youtube"]);
  });
});

describe("renderPreview", () => {
  it("passes profileName and normalizes undefined to null", async () => {
    mockInvoke.mockResolvedValue("/preview.mp4");

    await renderPreview("/clip.mp4", "/out", "tiktok");
    expect(mockInvoke).toHaveBeenCalledWith("render_preview", {
      inputClip: "/clip.mp4",
      outputDir: "/out",
      profileName: "tiktok",
    });

    mockInvoke.mockClear();
    await renderPreview("/clip.mp4", "/out");
    expect(mockInvoke).toHaveBeenCalledWith("render_preview", {
      inputClip: "/clip.mp4",
      outputDir: "/out",
      profileName: null,
    });
  });
});

describe("deletePreview", () => {
  it("invokes delete_preview with path", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await deletePreview("/preview.mp4");
    expect(mockInvoke).toHaveBeenCalledWith("delete_preview", {
      path: "/preview.mp4",
    });
  });
});

describe("renderReel", () => {
  it("passes shorts array and output path", async () => {
    const fakeResult = { output: "/reel.mp4", duration_secs: 120 };
    mockInvoke.mockResolvedValue(fakeResult);

    const result = await renderReel(["/a.mp4", "/b.mp4"], "/reel.mp4");
    expect(mockInvoke).toHaveBeenCalledWith("render_reel", {
      shorts: ["/a.mp4", "/b.mp4"],
      output: "/reel.mp4",
    });
    expect(result).toEqual(fakeResult);
  });
});

describe("listRenderProfiles", () => {
  it("invokes with no params", async () => {
    mockInvoke.mockResolvedValue([{ name: "tiktok" }]);
    const result = await listRenderProfiles();
    expect(mockInvoke).toHaveBeenCalledWith("list_render_profiles");
    expect(result).toEqual([{ name: "tiktok" }]);
  });
});
