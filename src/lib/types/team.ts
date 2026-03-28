export interface TeamProfile {
  team_name: string;
  short_name: string;
  level: string;
  logo_path: string;
  roster_path: string;
  colors: string[];
  jersey_colors: string[];
  metadata: Record<string, unknown>;
}
