import { invoke } from "@tauri-apps/api/core";
import type { RenderEntry } from "$lib/types/game";
import type { RenderProfile } from "$lib/types/config";
import type { RenderReelResult } from "$lib/types/render";

export async function renderShort(
  inputClip: string,
  outputDir: string,
  profileName: string,
  eventId?: string,
  gameDir?: string,
): Promise<RenderEntry> {
  return invoke<RenderEntry>("render_short", {
    inputClip,
    outputDir,
    profileName,
    eventId,
    gameDir,
  });
}

export async function renderPreview(
  inputClip: string,
  outputDir: string,
): Promise<string> {
  return invoke<string>("render_preview", { inputClip, outputDir });
}

export async function renderReel(
  shorts: string[],
  output: string,
): Promise<RenderReelResult> {
  return invoke<RenderReelResult>("render_reel", { shorts, output });
}

export async function listRenderProfiles(): Promise<RenderProfile[]> {
  return invoke<RenderProfile[]>("list_render_profiles");
}
