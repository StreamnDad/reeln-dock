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

/** Generate an MP4 proxy for non-web video formats. Returns playable path. */
export async function preparePreviewProxy(path: string): Promise<string> {
  return invoke<string>("prepare_preview_proxy", { path });
}

export interface ProxyCacheStats {
  file_count: number;
  total_bytes: number;
}

/** Get proxy cache stats (file count and total size). */
export async function getProxyCacheStats(): Promise<ProxyCacheStats> {
  return invoke<ProxyCacheStats>("get_proxy_cache_stats");
}

/** Clear all proxy cache files. Returns number of files removed. */
export async function clearProxyCache(): Promise<number> {
  return invoke<number>("clear_proxy_cache");
}
