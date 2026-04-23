import { invoke } from "@tauri-apps/api/core";
import type { RenderStageItem, CliQueueItem, QueueViewItem } from "$lib/types/queue";
import type { IterationItem, RenderOverrides } from "$lib/types/render";
import { renderShort, renderIteration } from "$lib/ipc/render";
import { queueList, queueListAll, queueEdit, queuePublish, queuePublishAll, queueRemove, queueTargets } from "$lib/ipc/queue";
import { listConfigProfiles } from "$lib/ipc/plugins";
import { log } from "$lib/stores/log.svelte";

// ---------------------------------------------------------------------------
// Render Staging (dock-local, persisted to render-stage.json)
// ---------------------------------------------------------------------------

let stageItems = $state<RenderStageItem[]>([]);
let stageInitialized = false;

function generateId(): string {
  return crypto.randomUUID();
}

/** Persist stage items to disk (fire-and-forget). */
function persistStage(): void {
  invoke("save_render_stage", { stageJson: JSON.stringify(stageItems) }).catch(
    (e) => log.error("RenderQueue", `Failed to persist stage: ${e}`),
  );
}

/** Load stage from disk and CLI queue on first access. */
export async function initQueue(): Promise<void> {
  if (stageInitialized) return;
  stageInitialized = true;
  try {
    const json = await invoke<string>("load_render_stage");
    const loaded: RenderStageItem[] = JSON.parse(json);
    // Reset any "rendering" items back to "pending" (interrupted by reload)
    stageItems = loaded.map((q) =>
      q.status === "rendering" ? { ...q, status: "pending" as const } : q,
    );
  } catch {
    stageItems = [];
  }
  // Load CLI queue eagerly so badge count is accurate from startup
  await refreshAllCliQueues();
}

export function getStageItems(): RenderStageItem[] {
  return stageItems;
}

export function getPendingStageCount(): number {
  return stageItems.filter((q) => q.status === "pending").length;
}

export function isClipInStage(clipPath: string): boolean {
  return stageItems.some((q) => q.clipPath === clipPath && q.status === "pending");
}

export function addToStage(item: {
  gameDir: string;
  gameName: string;
  eventId: string;
  clipPath: string;
  clipName: string;
  profiles: IterationItem[];
  concatOutput: boolean;
  overrides?: RenderOverrides;
  pluginProfile?: string;
  mode?: "short" | "apply";
  debug?: boolean;
  scorer?: string;
  assist1?: string;
  assist2?: string;
  playerNumbers?: string;
  noBranding?: boolean;
}): void {
  stageItems = [
    ...stageItems,
    {
      ...item,
      id: generateId(),
      status: "pending",
      addedAt: Date.now(),
    },
  ];
  persistStage();
}

export function removeFromStage(id: string): void {
  stageItems = stageItems.filter((q) => q.id !== id);
  persistStage();
}

export function clearStageErrors(): void {
  stageItems = stageItems.filter((q) => q.status !== "error");
  persistStage();
}

export function reorderStage(fromIdx: number, toIdx: number): void {
  const items = [...stageItems];
  const [moved] = items.splice(fromIdx, 1);
  items.splice(toIdx, 0, moved);
  stageItems = items;
  persistStage();
}

/** Resolve a plugin profile name to its config file path. */
async function resolveProfilePath(profileName?: string): Promise<string | undefined> {
  if (!profileName) return undefined;
  try {
    const profiles = await listConfigProfiles();
    const match = profiles.find((p) => p.name === profileName);
    return match?.path;
  } catch {
    log.error("RenderQueue", `Failed to resolve plugin profile '${profileName}'`);
    return undefined;
  }
}

/** Render a single stage item with --queue flag so it lands in CLI queue. */
async function renderStageItem(item: RenderStageItem): Promise<void> {
  stageItems = stageItems.map((q) =>
    q.id === item.id ? { ...q, status: "rendering" as const } : q,
  );
  persistStage();

  try {
    const outputDir = item.gameDir + "/renders";
    const configPath = await resolveProfilePath(item.pluginProfile);

    if (item.profiles.length === 1) {
      const profile = item.profiles[0];
      const mergedOverrides = item.overrides
        ? { ...item.overrides, ...profile.overrides }
        : profile.overrides;
      await renderShort(
        item.clipPath,
        outputDir,
        profile.profile_name,
        item.eventId,
        item.gameDir,
        mergedOverrides,
        item.mode,
        item.scorer,
        item.assist1,
        item.assist2,
        item.playerNumbers,
        item.debug,
        configPath,
        item.noBranding,
        true, // queue: true — add to CLI queue
      );
    } else {
      const items: IterationItem[] = item.profiles.map((p) => ({
        profile_name: p.profile_name,
        overrides: item.overrides
          ? { ...item.overrides, ...p.overrides }
          : p.overrides,
      }));
      await renderIteration(
        item.clipPath,
        outputDir,
        items,
        item.eventId,
        item.gameDir,
        item.concatOutput,
        item.mode,
        item.scorer,
        item.assist1,
        item.assist2,
        item.playerNumbers,
        item.debug,
        configPath,
        item.noBranding,
        true, // queue: true — add to CLI queue
      );
    }

    // Success: remove from staging (item is now in CLI queue)
    stageItems = stageItems.filter((q) => q.id !== item.id);
    log.info("RenderQueue", `Rendered and queued: ${item.clipName}`);

    // Refresh CLI queue to pick up the new item
    await refreshCliQueue(item.gameDir);
  } catch (err) {
    stageItems = stageItems.map((q) =>
      q.id === item.id
        ? { ...q, status: "error" as const, error: String(err) }
        : q,
    );
    log.error("RenderQueue", `Failed: ${item.clipName}: ${err}`);
  }
  persistStage();
}

export async function renderSingle(id: string): Promise<void> {
  const item = stageItems.find((q) => q.id === id);
  if (!item || item.status !== "pending") return;
  await renderStageItem(item);
}

export async function renderAll(): Promise<void> {
  const pending = stageItems.filter((q) => q.status === "pending");
  for (const item of pending) {
    await renderStageItem(item);
  }
}

// ---------------------------------------------------------------------------
// CLI Queue (read from per-game render_queue.json via IPC)
// ---------------------------------------------------------------------------

let cliItems = $state<CliQueueItem[]>([]);
let cliLoading = $state(false);
let publishTargets = $state<string[]>([]);

export function getCliQueueItems(): CliQueueItem[] {
  return cliItems;
}

export function isCliQueueLoading(): boolean {
  return cliLoading;
}

export function getPublishTargets(): string[] {
  return publishTargets;
}

/** Refresh CLI queue items for a specific game directory. */
export async function refreshCliQueue(gameDir: string): Promise<void> {
  cliLoading = true;
  try {
    const items = await queueList(gameDir);
    // Merge: replace items for this gameDir, keep others
    const otherItems = cliItems.filter((i) => i.game_dir !== gameDir);
    cliItems = [...otherItems, ...items];
  } catch (err) {
    log.error("RenderQueue", `Failed to load CLI queue for ${gameDir}: ${err}`);
  }
  cliLoading = false;
}

/** Refresh CLI queue items across all games. */
export async function refreshAllCliQueues(): Promise<void> {
  cliLoading = true;
  try {
    cliItems = await queueListAll();
  } catch (err) {
    log.error("RenderQueue", `Failed to load all CLI queues: ${err}`);
  }
  cliLoading = false;
}

/** Refresh available publish targets from plugins. */
export async function refreshPublishTargets(configPath?: string, profile?: string): Promise<void> {
  try {
    publishTargets = await queueTargets(configPath, profile);
  } catch (err) {
    log.error("RenderQueue", `Failed to load publish targets: ${err}`);
    publishTargets = [];
  }
}

/** Edit title/description of a CLI queue item. */
export async function editCliItem(
  gameDir: string,
  itemId: string,
  title?: string,
  description?: string,
): Promise<void> {
  try {
    const updated = await queueEdit(gameDir, itemId, title, description);
    cliItems = cliItems.map((i) => (i.id === updated.id ? updated : i));
  } catch (err) {
    log.error("RenderQueue", `Failed to edit queue item: ${err}`);
    throw err;
  }
}

/** Publish a CLI queue item to target(s).
 *
 * When ``configPath`` is omitted (typical dock usage), this auto-resolves
 * the item's stored ``config_profile`` to its file path so the CLI loads
 * the same plugin config used at render time. Without this, the CLI
 * would fall back to the default config — where plugin feature flags
 * (like cloudflare ``upload_video``) are off — and every target would be
 * skipped or fail. */
export async function publishCliItem(
  gameDir: string,
  itemId: string,
  target?: string,
  configPath?: string,
): Promise<void> {
  try {
    const effectiveConfigPath =
      configPath ?? (await _autoResolveItemConfigPath(itemId));
    const updated = await queuePublish(
      gameDir,
      itemId,
      target,
      effectiveConfigPath,
    );
    cliItems = cliItems.map((i) => (i.id === updated.id ? updated : i));
  } catch (err) {
    log.error("RenderQueue", `Failed to publish queue item: ${err}`);
    throw err;
  }
}

/** Publish all rendered items in a game's queue.
 *
 * See :func:`publishCliItem` for the config-path auto-resolution logic.
 * For the bulk path we resolve using the first rendered item's profile
 * — all items in a game almost always share the same config profile. */
export async function publishAllCliItems(
  gameDir: string,
  configPath?: string,
): Promise<void> {
  try {
    const effectiveConfigPath =
      configPath ?? (await _autoResolveGameConfigPath(gameDir));
    const updated = await queuePublishAll(gameDir, effectiveConfigPath);
    // Replace all items for this game
    const otherItems = cliItems.filter((i) => i.game_dir !== gameDir);
    cliItems = [...otherItems, ...updated];
  } catch (err) {
    log.error("RenderQueue", `Failed to publish all: ${err}`);
    throw err;
  }
}

/** Look up a queue item by ID and resolve its stored config_profile
 * to a config file path. Returns undefined when the item can't be
 * found or the profile name doesn't resolve. */
async function _autoResolveItemConfigPath(
  itemId: string,
): Promise<string | undefined> {
  const item = cliItems.find((i) => i.id === itemId);
  if (!item || !item.config_profile) return undefined;
  return resolveProfilePath(item.config_profile);
}

/** Resolve a config path for a game's bulk publish by reading the
 * first rendered item's stored config_profile. */
async function _autoResolveGameConfigPath(
  gameDir: string,
): Promise<string | undefined> {
  const item = cliItems.find(
    (i) => i.game_dir === gameDir && i.config_profile,
  );
  if (!item) return undefined;
  return resolveProfilePath(item.config_profile);
}

/** Soft-delete a CLI queue item. */
export async function removeCliItem(
  gameDir: string,
  itemId: string,
): Promise<void> {
  try {
    await queueRemove(gameDir, itemId);
    // Remove from local state (it's now status "removed", hidden from list)
    cliItems = cliItems.filter((i) => i.id !== itemId);
  } catch (err) {
    log.error("RenderQueue", `Failed to remove queue item: ${err}`);
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Unified view
// ---------------------------------------------------------------------------

/** Get all queue items for display: staging items first, then CLI items. */
export function getQueueViewItems(): QueueViewItem[] {
  const stageView: QueueViewItem[] = stageItems.map((item) => ({
    kind: "stage" as const,
    item,
  }));
  const cliView: QueueViewItem[] = cliItems.map((item) => ({
    kind: "cli" as const,
    item,
  }));
  return [...stageView, ...cliView];
}

/** Count of items needing attention (pending renders + unpublished CLI items). */
export function getBadgeCount(): number {
  const pendingStage = stageItems.filter((q) => q.status === "pending").length;
  const unpublishedCli = cliItems.filter((q) => q.status === "rendered").length;
  return pendingStage + unpublishedCli;
}

// ---------------------------------------------------------------------------
// Backwards compatibility aliases
// ---------------------------------------------------------------------------

/** @deprecated Use getStageItems() instead */
export const getQueue = getStageItems;
/** @deprecated Use getPendingStageCount() instead */
export const getPendingCount = getPendingStageCount;
/** @deprecated Use isClipInStage() instead */
export const isClipInQueue = isClipInStage;
/** @deprecated Use addToStage() instead */
export const addToQueue = addToStage;
/** @deprecated Use removeFromStage() instead */
export const removeFromQueue = removeFromStage;
/** @deprecated Use clearStageErrors() instead */
export const clearCompleted = clearStageErrors;
/** @deprecated Use reorderStage() instead */
export const reorderQueue = reorderStage;
