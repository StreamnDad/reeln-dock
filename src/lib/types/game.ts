export interface GameInfo {
  date: string;
  home_team: string;
  away_team: string;
  sport: string;
  game_number: number;
  venue: string;
  game_time: string;
  period_length: number;
  description: string;
  thumbnail: string;
  level: string;
  home_slug: string;
  away_slug: string;
  tournament: string;
}

export interface GameEvent {
  id: string;
  clip: string;
  segment_number: number;
  event_type: string;
  player: string;
  created_at: string;
  metadata: Record<string, unknown>;
}

export interface RenderEntry {
  input: string;
  output: string;
  segment_number: number;
  format: string;
  crop_mode: string;
  rendered_at: string;
  event_id: string;
}

export interface GameState {
  game_info: GameInfo;
  segments_processed: number[];
  highlighted: boolean;
  finished: boolean;
  created_at: string;
  finished_at: string;
  renders: RenderEntry[];
  events: GameEvent[];
  livestreams: Record<string, string>;
  segment_outputs: string[];
  highlights_output: string;
}

export interface GameSummary {
  dir_path: string;
  state: GameState;
}

export interface TournamentGroup {
  tournament: string;
  games: GameSummary[];
}
