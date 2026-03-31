import type { IterationItem, RenderOverrides } from "./render";
import type { RenderEntry } from "./game";

export interface QueueItem {
  id: string;
  gameDir: string;
  gameName: string;
  eventId: string;
  clipPath: string;
  clipName: string;
  profiles: IterationItem[];
  concatOutput: boolean;
  overrides?: RenderOverrides;
  pluginProfile?: string;
  status: "pending" | "rendering" | "done" | "error";
  jobId?: string;
  error?: string;
  addedAt: number;
  results?: RenderEntry[];
}
