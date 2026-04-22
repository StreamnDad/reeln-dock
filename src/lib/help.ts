/** Help text and documentation links for dock settings. */

const DOCS_BASE = "https://reeln-cli.readthedocs.io/en/latest";

export interface HelpEntry {
  text: string;
  url?: string;
}

export const help: Record<string, HelpEntry> = {
  // ── Render Profile Fields ─────────────────────────────────
  "profile.width": {
    text: "Output video width in pixels. Common values: 1080 (vertical short), 1920 (landscape).",
    url: `${DOCS_BASE}/cli/render/`,
  },
  "profile.height": {
    text: "Output video height in pixels. Common values: 1920 (vertical short), 1080 (landscape).",
    url: `${DOCS_BASE}/cli/render/`,
  },
  "profile.crop_mode": {
    text: "How to fit source video into target dimensions. 'crop' cuts edges to fill, 'pad' adds bars to fit without cutting.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.pad_color": {
    text: "Background color for padding bars when crop_mode is 'pad'. Hex format: #000000 for black.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.anchor_x": {
    text: "Horizontal crop anchor (0.0 = left, 0.5 = center, 1.0 = right). Controls which part of the frame is kept when cropping.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.anchor_y": {
    text: "Vertical crop anchor (0.0 = top, 0.5 = center, 1.0 = bottom). Controls which part of the frame is kept when cropping.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.scale": {
    text: "Scale factor applied after cropping. 1.0 = no change, 0.5 = half size, 2.0 = double.",
    url: `${DOCS_BASE}/cli/render/`,
  },
  "profile.speed": {
    text: "Playback speed multiplier. 1.0 = normal, 0.5 = half speed (slow-mo), 2.0 = double speed.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.speed_segments": {
    text: "Variable speed across the clip. Format: time ranges with speed values. Overrides the base speed setting.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.lut": {
    text: "Path to a LUT (Look-Up Table) file for color grading. Supports .cube format.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "profile.codec": {
    text: "Video codec for encoding. libx264 (H.264) is widely compatible, libx265 (H.265/HEVC) is smaller but slower to encode.",
    url: `${DOCS_BASE}/guide/configuration/#video`,
  },
  "profile.preset": {
    text: "Encoding speed preset. Faster presets = larger files, slower presets = better compression. 'medium' is a good balance.",
    url: `${DOCS_BASE}/guide/configuration/#video`,
  },
  "profile.crf": {
    text: "Constant Rate Factor controls quality. Lower = higher quality, larger file. 18 is near-lossless, 23 is default, 28+ is low quality.",
    url: `${DOCS_BASE}/guide/configuration/#video`,
  },
  "profile.audio_codec": {
    text: "Audio codec for the output. 'aac' is the most compatible choice.",
    url: `${DOCS_BASE}/guide/configuration/#video`,
  },
  "profile.audio_bitrate": {
    text: "Audio bitrate in kbps. 128k is standard, 192k is high quality, 256k+ is near-transparent.",
    url: `${DOCS_BASE}/guide/configuration/#video`,
  },
  "profile.overlay_template": {
    text: "Path to a JSON overlay template. Composites graphics (score, team names, logos) onto the rendered video.",
    url: `${DOCS_BASE}/guide/overlay-templates/`,
  },
  "profile.smart_zoom": {
    text: "AI-powered smart crop tracking using the OpenAI plugin. Dynamically reframes the video to follow the action.",
    url: `${DOCS_BASE}/examples/10-smart-zoom/`,
  },

  // ── Rendering Settings ─────────────────────────────────────
  "render.mode": {
    text: "'Short' crops/scales to profile dimensions (vertical shorts). 'Apply' keeps the original frame, only applying speed, LUT, and overlay.",
    url: `${DOCS_BASE}/cli/render/`,
  },
  "render.default_profile": {
    text: "The render profile used by default when no specific profile is selected.",
    url: `${DOCS_BASE}/guide/configuration/#render-profiles`,
  },
  "render.iteration_mappings": {
    text: "Maps event types to render profiles. When iterating, each event is rendered with the profile assigned to its type.",
    url: `${DOCS_BASE}/examples/07-profiles-and-iterations/`,
  },
  "render.concat": {
    text: "When rendering multiple profiles, concatenate results into a single output file instead of separate files.",
    url: `${DOCS_BASE}/cli/render/`,
  },

  // ── Plugin Settings ────────────────────────────────────────
  "plugins.profile": {
    text: "Plugin config profile determines which plugins are active and their settings for rendering and publishing.",
    url: `${DOCS_BASE}/cli/plugins/`,
  },
  "plugins.enforce_hooks": {
    text: "When enabled, plugin hooks must complete successfully for game operations to proceed. Disable to skip plugin processing.",
    url: `${DOCS_BASE}/cli/plugins/`,
  },

  // ── Event Types ────────────────────────────────────────────
  "events.types": {
    text: "Define the event categories for tagging clips (e.g., goal, save, penalty). Event types drive iteration mappings and metadata.",
    url: `${DOCS_BASE}/guide/configuration/#event-types`,
  },
  "events.team_specific": {
    text: "When enabled, creates Home/Away variants of this event type for team-specific tagging.",
    url: `${DOCS_BASE}/guide/configuration/#event-types`,
  },

  // ── Queue ──────────────────────────────────────────────────
  "queue.publish": {
    text: "Publish rendered clips to configured platform targets (YouTube, Instagram, TikTok, etc.) via plugins.",
    url: `${DOCS_BASE}/cli/queue/`,
  },

  // ── Config ─────────────────────────────────────────────────
  "config.source_dir": {
    text: "Directory where replay/recording files are captured by your streaming software (OBS, etc.).",
    url: `${DOCS_BASE}/guide/configuration/#paths`,
  },
  "config.output_dir": {
    text: "Base directory for game directories. Each game gets a subdirectory with its state, clips, and renders.",
    url: `${DOCS_BASE}/guide/configuration/#paths`,
  },
};
