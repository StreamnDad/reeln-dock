export interface ProgressEvent {
  job_id: string;
  phase: string;
  progress: number;
  message: string;
}

export interface RenderOverrides {
  crop_mode?: string;
  scale?: number;
  speed?: number;
  smart?: boolean;
  anchor_x?: number;
  anchor_y?: number;
  pad_color?: string;
  zoom_frames?: number;
  [key: string]: unknown;
}

export interface IterationItem {
  profile_name: string;
  overrides?: RenderOverrides;
}

export interface RenderShortRequest {
  input_clip: string;
  output_dir: string;
  profile_name: string;
  event_id?: string;
  game_dir?: string;
  overrides?: RenderOverrides;
}

export interface RenderReelRequest {
  shorts: string[];
  output: string;
}

export interface RenderReelResult {
  output: string;
  duration_secs: number;
}
