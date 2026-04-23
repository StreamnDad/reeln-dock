import type { IterationItem, RenderOverrides } from "./render";

// ---------------------------------------------------------------------------
// Dock-local render staging (pre-render items)
// ---------------------------------------------------------------------------

export type RenderStageStatus = "pending" | "rendering" | "error";

export interface RenderStageItem {
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
  mode?: "short" | "apply";
  debug?: boolean;
  scorer?: string;
  assist1?: string;
  assist2?: string;
  playerNumbers?: string;
  noBranding?: boolean;
  status: RenderStageStatus;
  jobId?: string;
  error?: string;
  addedAt: number;
}

// ---------------------------------------------------------------------------
// CLI queue types (from render_queue.json, managed by reeln-cli)
// ---------------------------------------------------------------------------

export type CliQueueStatus =
  | "rendered"
  | "publishing"
  | "published"
  | "partial"
  | "failed"
  | "removed";

export type PublishTargetStatus =
  | "pending"
  | "published"
  | "failed"
  | "skipped";

export interface PublishTargetResult {
  target: string;
  status: PublishTargetStatus;
  url: string;
  error: string;
  published_at: string;
}

export interface CliQueueItem {
  id: string;
  output: string;
  game_dir: string;
  status: CliQueueStatus;
  queued_at: string;

  // Render metadata (snapshotted at queue time)
  duration_seconds: number | null;
  file_size_bytes: number | null;
  format: string;
  crop_mode: string;
  render_profile: string;
  event_id: string;

  // Game context (snapshotted from GameInfo/GameEvent)
  home_team: string;
  away_team: string;
  date: string;
  sport: string;
  level: string;
  tournament: string;
  event_type: string;
  player: string;
  assists: string;

  // Editable publish metadata
  title: string;
  description: string;

  // Per-target publish tracking
  publish_targets: PublishTargetResult[];

  // Config profile used at queue time
  config_profile: string;

  // Plugin inputs passed through
  plugin_inputs: Record<string, unknown>;
}

export interface RenderQueue {
  version: number;
  items: CliQueueItem[];
}

// ---------------------------------------------------------------------------
// Unified view discriminated union
// ---------------------------------------------------------------------------

export type QueueViewItem =
  | { kind: "stage"; item: RenderStageItem }
  | { kind: "cli"; item: CliQueueItem };
