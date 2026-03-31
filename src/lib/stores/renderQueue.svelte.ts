import type { QueueItem } from "$lib/types/queue";
import type { IterationItem, RenderOverrides } from "$lib/types/render";
import { renderShort, renderIteration } from "$lib/ipc/render";
import { log } from "$lib/stores/log.svelte";

let queue = $state<QueueItem[]>([]);

function generateId(): string {
  return crypto.randomUUID();
}

export function getQueue(): QueueItem[] {
  return queue;
}

export function getPendingCount(): number {
  return queue.filter((q) => q.status === "pending").length;
}

export function addToQueue(item: {
  gameDir: string;
  gameName: string;
  eventId: string;
  clipPath: string;
  clipName: string;
  profiles: IterationItem[];
  concatOutput: boolean;
  overrides?: RenderOverrides;
  pluginProfile?: string;
}): void {
  queue = [
    ...queue,
    {
      ...item,
      id: generateId(),
      status: "pending",
      addedAt: Date.now(),
    },
  ];
}

export function removeFromQueue(id: string): void {
  queue = queue.filter((q) => q.id !== id);
}

export function clearCompleted(): void {
  queue = queue.filter((q) => q.status !== "done" && q.status !== "error");
}

export function reorderQueue(fromIdx: number, toIdx: number): void {
  const items = [...queue];
  const [moved] = items.splice(fromIdx, 1);
  items.splice(toIdx, 0, moved);
  queue = items;
}

async function renderItem(item: QueueItem): Promise<void> {
  queue = queue.map((q) =>
    q.id === item.id ? { ...q, status: "rendering" as const } : q,
  );

  try {
    const outputDir = item.gameDir + "/renders";
    if (item.profiles.length === 1) {
      const profile = item.profiles[0];
      const mergedOverrides = item.overrides
        ? { ...item.overrides, ...profile.overrides }
        : profile.overrides;
      const entry = await renderShort(
        item.clipPath,
        outputDir,
        profile.profile_name,
        item.eventId,
        item.gameDir,
        mergedOverrides,
      );
      queue = queue.map((q) =>
        q.id === item.id
          ? { ...q, status: "done" as const, results: [entry] }
          : q,
      );
    } else {
      const items: IterationItem[] = item.profiles.map((p) => ({
        profile_name: p.profile_name,
        overrides: item.overrides
          ? { ...item.overrides, ...p.overrides }
          : p.overrides,
      }));
      const entries = await renderIteration(
        item.clipPath,
        outputDir,
        items,
        item.eventId,
        item.gameDir,
        item.concatOutput,
      );
      queue = queue.map((q) =>
        q.id === item.id
          ? { ...q, status: "done" as const, results: entries }
          : q,
      );
    }
    log.info("RenderQueue", `Completed: ${item.clipName}`);
  } catch (err) {
    queue = queue.map((q) =>
      q.id === item.id
        ? { ...q, status: "error" as const, error: String(err) }
        : q,
    );
    log.error("RenderQueue", `Failed: ${item.clipName}: ${err}`);
  }
}

export async function renderSingle(id: string): Promise<void> {
  const item = queue.find((q) => q.id === id);
  if (!item || item.status !== "pending") return;
  await renderItem(item);
}

export async function renderAll(): Promise<void> {
  const pending = queue.filter((q) => q.status === "pending");
  for (const item of pending) {
    await renderItem(item);
  }
}
