import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Mock } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  probeClip,
  openInFinder,
  openFile,
  fileExists,
  preparePreviewProxy,
  getPlatform,
  revealLabel,
} from "./media";

const mockInvoke = invoke as Mock;

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("probeClip", () => {
  it("passes path and returns MediaInfoResponse", async () => {
    const info = {
      duration_secs: 30.5,
      fps: 60,
      width: 1920,
      height: 1080,
      codec: "h264",
    };
    mockInvoke.mockResolvedValue(info);
    const result = await probeClip("/games/clip.mp4");
    expect(mockInvoke).toHaveBeenCalledWith("probe_clip", {
      path: "/games/clip.mp4",
    });
    expect(result).toEqual(info);
  });
});

describe("openInFinder", () => {
  it("passes path to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await openInFinder("/games/test");
    expect(mockInvoke).toHaveBeenCalledWith("open_in_finder", {
      path: "/games/test",
    });
  });
});

describe("openFile", () => {
  it("passes path to invoke", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await openFile("/games/clip.mp4");
    expect(mockInvoke).toHaveBeenCalledWith("open_file", {
      path: "/games/clip.mp4",
    });
  });
});

describe("fileExists", () => {
  it("passes path and returns boolean", async () => {
    mockInvoke.mockResolvedValue(true);
    const result = await fileExists("/games/clip.mp4");
    expect(mockInvoke).toHaveBeenCalledWith("file_exists", {
      path: "/games/clip.mp4",
    });
    expect(result).toBe(true);
  });

  it("returns false when file does not exist", async () => {
    mockInvoke.mockResolvedValue(false);
    const result = await fileExists("/nonexistent.mp4");
    expect(result).toBe(false);
  });
});

describe("getPlatform", () => {
  it("invokes get_platform and returns the host OS identifier", async () => {
    mockInvoke.mockResolvedValue("macos");
    const result = await getPlatform();
    expect(mockInvoke).toHaveBeenCalledWith("get_platform");
    expect(result).toBe("macos");
  });
});

describe("revealLabel", () => {
  it("returns Finder label on macOS", () => {
    expect(revealLabel("macos")).toBe("Show in Finder");
  });

  it("returns Explorer label on Windows", () => {
    expect(revealLabel("windows")).toBe("Show in Explorer");
  });

  it("returns File Manager label on Linux", () => {
    expect(revealLabel("linux")).toBe("Show in File Manager");
  });

  it("falls back to File Manager label on unknown platforms", () => {
    expect(revealLabel("other")).toBe("Show in File Manager");
  });
});

describe("preparePreviewProxy", () => {
  it("passes path and returns playable path", async () => {
    mockInvoke.mockResolvedValue("/app/proxies/abc123_clip.mp4");
    const result = await preparePreviewProxy("/games/clip.mkv");
    expect(mockInvoke).toHaveBeenCalledWith("prepare_preview_proxy", {
      path: "/games/clip.mkv",
    });
    expect(result).toBe("/app/proxies/abc123_clip.mp4");
  });

  it("returns original path for web-playable formats", async () => {
    mockInvoke.mockResolvedValue("/games/clip.mp4");
    const result = await preparePreviewProxy("/games/clip.mp4");
    expect(result).toBe("/games/clip.mp4");
  });
});
