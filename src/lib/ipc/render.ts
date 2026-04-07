import { invoke } from "@tauri-apps/api/core";
import type { RenderEntry } from "$lib/types/game";
import type { RenderProfile } from "$lib/types/config";
import type { RenderReelResult, RenderOverrides, IterationItem } from "$lib/types/render";

export async function renderShort(
  inputClip: string,
  outputDir: string,
  profileName: string,
  eventId?: string,
  gameDir?: string,
  overrides?: RenderOverrides,
  mode?: string,
  scorer?: string,
  assist1?: string,
  assist2?: string,
  playerNumbers?: string,
  debug?: boolean,
  configPath?: string,
  noBranding?: boolean,
  queue?: boolean,
): Promise<RenderEntry> {
  return invoke<RenderEntry>("render_short", {
    inputClip,
    outputDir,
    profileName,
    eventId,
    gameDir,
    overrides: overrides ?? null,
    mode: mode ?? null,
    scorer: scorer ?? null,
    assist1: assist1 ?? null,
    assist2: assist2 ?? null,
    playerNumbers: playerNumbers ?? null,
    debug: debug ?? null,
    configPath: configPath ?? null,
    noBranding: noBranding ?? null,
    queue: queue ?? null,
  });
}

export async function renderIteration(
  inputClip: string,
  outputDir: string,
  items: IterationItem[],
  eventId?: string,
  gameDir?: string,
  concatOutput: boolean = true,
  mode?: string,
  scorer?: string,
  assist1?: string,
  assist2?: string,
  playerNumbers?: string,
  debug?: boolean,
  configPath?: string,
  noBranding?: boolean,
  queue?: boolean,
): Promise<RenderEntry[]> {
  return invoke<RenderEntry[]>("render_iteration", {
    inputClip,
    outputDir,
    items,
    eventId,
    gameDir,
    concatOutput,
    mode: mode ?? null,
    scorer: scorer ?? null,
    assist1: assist1 ?? null,
    assist2: assist2 ?? null,
    playerNumbers: playerNumbers ?? null,
    debug: debug ?? null,
    configPath: configPath ?? null,
    noBranding: noBranding ?? null,
    queue: queue ?? null,
  });
}

export async function getIterationProfiles(
  eventType: string,
): Promise<string[]> {
  return invoke<string[]>("get_iteration_profiles", { eventType });
}

export async function renderPreview(
  inputClip: string,
  outputDir: string,
  profileName?: string,
): Promise<string> {
  return invoke<string>("render_preview", {
    inputClip,
    outputDir,
    profileName: profileName ?? null,
  });
}

export async function deletePreview(path: string): Promise<void> {
  return invoke<void>("delete_preview", { path });
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

export async function renderProfilePreview(
  inputClip: string,
  outputDir: string,
  profile: Partial<RenderProfile>,
): Promise<string> {
  return invoke<string>("render_profile_preview", {
    inputClip,
    outputDir,
    profile,
  });
}

export async function suggestPreviewClip(): Promise<string | null> {
  return invoke<string | null>("suggest_preview_clip");
}
