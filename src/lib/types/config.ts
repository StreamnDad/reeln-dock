export interface VideoConfig {
  ffmpeg_path: string;
  codec: string;
  preset: string;
  crf: number;
  audio_codec: string;
  audio_bitrate: string;
}

export interface PathConfig {
  source_dir: string | null;
  source_glob: string;
  output_dir: string | null;
  temp_dir: string | null;
}

export interface PluginsConfig {
  enabled: string[];
  disabled: string[];
  settings: Record<string, unknown>;
  registry_url: string;
  enforce_hooks: boolean;
}

export interface BrandingConfig {
  enabled: boolean;
  template: string;
  duration: number;
}

export interface OrchestrationConfig {
  upload_bitrate_kbps: number;
  sequential: boolean;
}

export interface SpeedSegment {
  speed: number;
  until: number | null;
}

export interface RenderProfile {
  name: string;
  width?: number;
  height?: number;
  crop_mode?: string;
  anchor_x?: number;
  anchor_y?: number;
  pad_color?: string;
  scale?: number;
  smart?: boolean;
  speed?: number;
  speed_segments?: SpeedSegment[];
  lut?: string;
  subtitle_template?: string;
  codec?: string;
  preset?: string;
  crf?: number;
  audio_codec?: string;
  audio_bitrate?: string;
}

export interface ProfilePreset {
  label: string;
  profile: Partial<RenderProfile>;
}

export const PROFILE_PRESETS: ProfilePreset[] = [
  {
    label: "TikTok 9:16",
    profile: { width: 1080, height: 1920, crop_mode: "crop", anchor_x: 0.5, anchor_y: 0.5 },
  },
  {
    label: "YouTube 16:9",
    profile: { width: 1920, height: 1080, crop_mode: "pad" },
  },
  {
    label: "Instagram Square",
    profile: { width: 1080, height: 1080, crop_mode: "crop", anchor_x: 0.5, anchor_y: 0.5 },
  },
  {
    label: "Instagram Reels",
    profile: { width: 1080, height: 1920, crop_mode: "crop", anchor_x: 0.5, anchor_y: 0.5 },
  },
  {
    label: "Twitter/X",
    profile: { width: 1280, height: 720, crop_mode: "pad" },
  },
  {
    label: "Full Frame",
    profile: {},
  },
];

export interface IterationConfig {
  mappings: Record<string, string[]>;
}

export interface EventTypeEntry {
  name: string;
  team_specific: boolean;
}

export interface AppConfig {
  config_version: number;
  sport: string;
  event_types: EventTypeEntry[];
  video: VideoConfig;
  paths: PathConfig;
  render_profiles: Record<string, RenderProfile>;
  iterations: IterationConfig;
  branding: BrandingConfig;
  orchestration: OrchestrationConfig;
  plugins: PluginsConfig;
}
