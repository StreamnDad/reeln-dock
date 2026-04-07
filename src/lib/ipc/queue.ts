import { invoke } from "@tauri-apps/api/core";
import type { CliQueueItem } from "$lib/types/queue";

/** List queue items for a game directory, optionally filtered by status. */
export async function queueList(
  gameDir: string,
  statusFilter?: string,
): Promise<CliQueueItem[]> {
  return invoke<CliQueueItem[]>("queue_list", {
    gameDir,
    statusFilter: statusFilter ?? null,
  });
}

/** List queue items across all games via the central queue index. */
export async function queueListAll(
  statusFilter?: string,
): Promise<CliQueueItem[]> {
  return invoke<CliQueueItem[]>("queue_list_all", {
    statusFilter: statusFilter ?? null,
  });
}

/** Get a single queue item by ID or prefix. */
export async function queueShow(
  gameDir: string,
  itemId: string,
): Promise<CliQueueItem> {
  return invoke<CliQueueItem>("queue_show", { gameDir, itemId });
}

/** List available publish targets from loaded plugins. */
export async function queueTargets(
  configPath?: string,
  profile?: string,
): Promise<string[]> {
  return invoke<string[]>("queue_targets", {
    configPath: configPath ?? null,
    profile: profile ?? null,
  });
}

/** Edit title/description of a queue item. */
export async function queueEdit(
  gameDir: string,
  itemId: string,
  title?: string,
  description?: string,
): Promise<CliQueueItem> {
  return invoke<CliQueueItem>("queue_edit", {
    gameDir,
    itemId,
    title: title ?? null,
    description: description ?? null,
  });
}

/** Publish a queue item to target(s). */
export async function queuePublish(
  gameDir: string,
  itemId: string,
  target?: string,
  configPath?: string,
): Promise<CliQueueItem> {
  return invoke<CliQueueItem>("queue_publish", {
    gameDir,
    itemId,
    target: target ?? null,
    configPath: configPath ?? null,
  });
}

/** Publish all rendered items in a game's queue. */
export async function queuePublishAll(
  gameDir: string,
  configPath?: string,
): Promise<CliQueueItem[]> {
  return invoke<CliQueueItem[]>("queue_publish_all", {
    gameDir,
    configPath: configPath ?? null,
  });
}

/** Soft-delete a queue item. */
export async function queueRemove(
  gameDir: string,
  itemId: string,
): Promise<CliQueueItem> {
  return invoke<CliQueueItem>("queue_remove", { gameDir, itemId });
}
