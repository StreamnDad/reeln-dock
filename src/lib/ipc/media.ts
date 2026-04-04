import { invoke } from "@tauri-apps/api/core";
import type { MediaInfoResponse } from "$lib/types/media";

export async function probeClip(
  path: string,
): Promise<MediaInfoResponse> {
  return invoke<MediaInfoResponse>("probe_clip", { path });
}

export async function openInFinder(path: string): Promise<void> {
  return invoke<void>("open_in_finder", { path });
}

export async function openFile(path: string): Promise<void> {
  return invoke<void>("open_file", { path });
}

export async function fileExists(path: string): Promise<boolean> {
  return invoke<boolean>("file_exists", { path });
}
