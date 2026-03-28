export interface ProgressEvent {
  job_id: string;
  phase: string;
  progress: number;
  message: string;
}

export interface RenderShortRequest {
  input_clip: string;
  output_dir: string;
  profile_name: string;
  event_id?: string;
  game_dir?: string;
}

export interface RenderReelRequest {
  shorts: string[];
  output: string;
}

export interface RenderReelResult {
  output: string;
  duration_secs: number;
}
