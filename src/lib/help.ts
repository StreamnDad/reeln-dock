/** Help text and documentation links for dock settings. */

const DOCS_BASE = "https://reeln-cli.readthedocs.io/en/latest";

export interface HelpEntry {
  text: string;
  url?: string;
}

export const help: Record<string, HelpEntry> = {
  // ── Getting Started / Setup ────────────────────────────────
  "setup.welcome": {
    text: "reeln dock is the desktop companion for reeln-cli. Point it at your config file to get started.",
    url: `${DOCS_BASE}/quickstart.html`,
  },
  "setup.config_file": {
    text: "Your reeln config file (reeln.json) defines paths, video settings, render profiles, and plugins. The dock reads this file to configure your workflow.",
    url: `${DOCS_BASE}/guide/configuration.html`,
  },
  "setup.install_cli": {
    text: "The reeln CLI powers rendering, publishing, and plugin features. Install it with: uv pip install reeln",
    url: `${DOCS_BASE}/install.html`,
  },

  // ── Config / Dock Settings ─────────────────────────────────
  "config.file_locations": {
    text: "reeln looks for config files in standard locations. Use named profiles to switch between different configurations.",
    url: `${DOCS_BASE}/guide/configuration.html#config-file-locations`,
  },
  "config.source_dir": {
    text: "Directory where replay/recording files are captured by your streaming software (OBS, etc.).",
    url: `${DOCS_BASE}/guide/configuration.html#paths-section`,
  },
  "config.output_dir": {
    text: "Base directory for game directories. Each game gets a subdirectory with its state, clips, and renders.",
    url: `${DOCS_BASE}/guide/configuration.html#paths-section`,
  },
  "config.named_profiles": {
    text: "Named config profiles let you switch between different setups (e.g., different sports, teams, or render settings).",
    url: `${DOCS_BASE}/guide/configuration.html#named-profiles`,
  },
  "config.env_overrides": {
    text: "Override any config value with REELN_ environment variables. Useful for CI/CD or per-machine settings.",
    url: `${DOCS_BASE}/guide/configuration.html#environment-variable-overrides`,
  },
  "config.doctor": {
    text: "Run 'reeln doctor' to validate your configuration and check for common issues.",
    url: `${DOCS_BASE}/cli/doctor.html`,
  },

  // ── New Game ───────────────────────────────────────────────
  "game.create": {
    text: "Create a new game directory with team names, date, and sport. This initializes the game state and runs plugin hooks.",
    url: `${DOCS_BASE}/examples/03-starting-a-game.html`,
  },
  "game.sport": {
    text: "The sport determines segment naming (periods, quarters, halves, innings) and validation rules.",
    url: `${DOCS_BASE}/guide/sports.html`,
  },

  // ── Game View ──────────────────────────────────────────────
  "game.segments": {
    text: "Segments are time divisions of a game (period, quarter, half). Process each segment to discover clips and events.",
    url: `${DOCS_BASE}/examples/04-segments-and-events.html`,
  },
  "game.events": {
    text: "Events are tagged moments within a segment — goals, saves, penalties, etc. Each event links to a video clip.",
    url: `${DOCS_BASE}/examples/04-segments-and-events.html`,
  },
  "game.highlights": {
    text: "Merge processed segments into a single highlight reel per game. Requires all segments to be processed first.",
    url: `${DOCS_BASE}/examples/06-highlights-and-reels.html`,
  },
  "game.finish": {
    text: "Mark a game as finished when all clips are reviewed and rendered. This locks the game state for archival.",
    url: `${DOCS_BASE}/examples/08-game-finish-and-cleanup.html`,
  },
  "game.prune": {
    text: "Remove intermediate files (concat outputs, debug artifacts) to reclaim disk space. Only available for finished games.",
    url: `${DOCS_BASE}/examples/08-game-finish-and-cleanup.html`,
  },
  "game.livestream": {
    text: "Associate a livestream URL with a game for reference and potential plugin-driven features like thumbnail extraction.",
    url: `${DOCS_BASE}/cli/game.html`,
  },
  "game.tournament": {
    text: "Assign games to tournaments for organization. Drag games between tournaments in the sidebar.",
    url: `${DOCS_BASE}/cli/game.html`,
  },

  // ── Clip Review / Rendering ────────────────────────────────
  "clip.render_short": {
    text: "Render a clip as a short — crops and scales to the profile dimensions for vertical social media formats.",
    url: `${DOCS_BASE}/examples/05-rendering-shorts.html`,
  },
  "clip.render_apply": {
    text: "Apply mode keeps the original frame dimensions. Only applies speed changes, LUT color grading, and overlay templates.",
    url: `${DOCS_BASE}/cli/render.html#reeln-render-apply`,
  },
  "clip.event_tagging": {
    text: "Tag events with a type (goal, save, etc.) and optionally assign a team. Tags drive iteration mappings for batch rendering.",
    url: `${DOCS_BASE}/examples/04-segments-and-events.html`,
  },
  "clip.player_lookup": {
    text: "Enter jersey numbers to auto-resolve player names from team roster CSV files. Names appear on overlay templates.",
    url: `${DOCS_BASE}/cli/render.html#player-number-roster-lookup`,
  },
  "clip.debug": {
    text: "Enable debug mode to save intermediate files (pre-overlay, filter chain visualization) for troubleshooting render issues.",
    url: `${DOCS_BASE}/guide/configuration.html#debug-artifacts`,
  },
  "clip.preview": {
    text: "Preview the render plan without encoding. Shows the filter chain, output dimensions, and estimated file size.",
    url: `${DOCS_BASE}/cli/render.html#reeln-render-preview`,
  },

  // ── Render Profile Fields ──────────────────────────────────
  "profile.width": {
    text: "Output video width in pixels. Common values: 1080 (vertical short), 1920 (landscape).",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "profile.height": {
    text: "Output video height in pixels. Common values: 1920 (vertical short), 1080 (landscape).",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "profile.crop_mode": {
    text: "How to fit source video into target dimensions. 'crop' cuts edges to fill, 'pad' adds bars to fit without cutting.",
    url: `${DOCS_BASE}/cli/render.html#crop-modes`,
  },
  "profile.pad_color": {
    text: "Background color for padding bars when crop_mode is 'pad'. Hex format: #000000 for black.",
    url: `${DOCS_BASE}/cli/render.html#crop-modes`,
  },
  "profile.anchor_x": {
    text: "Horizontal crop anchor (0.0 = left, 0.5 = center, 1.0 = right). Controls which part of the frame is kept when cropping.",
    url: `${DOCS_BASE}/cli/render.html#crop-modes`,
  },
  "profile.anchor_y": {
    text: "Vertical crop anchor (0.0 = top, 0.5 = center, 1.0 = bottom). Controls which part of the frame is kept when cropping.",
    url: `${DOCS_BASE}/cli/render.html#crop-modes`,
  },
  "profile.scale": {
    text: "Scale factor applied after cropping. 1.0 = no change, 0.5 = half size, 2.0 = double.",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "profile.speed": {
    text: "Playback speed multiplier. 1.0 = normal, 0.5 = half speed (slow-mo), 2.0 = double speed.",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "profile.speed_segments": {
    text: "Variable speed across the clip. Format: time ranges with speed values. Overrides the base speed setting.",
    url: `${DOCS_BASE}/cli/render.html#variable-speed-segments`,
  },
  "profile.lut": {
    text: "Path to a LUT (Look-Up Table) file for color grading. Supports .cube format.",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "profile.codec": {
    text: "Video codec for encoding. libx264 (H.264) is widely compatible, libx265 (H.265/HEVC) is smaller but slower to encode.",
    url: `${DOCS_BASE}/cli/render.html#encoding-settings`,
  },
  "profile.preset": {
    text: "Encoding speed preset. Faster presets = larger files, slower presets = better compression. 'medium' is a good balance.",
    url: `${DOCS_BASE}/cli/render.html#encoding-settings`,
  },
  "profile.crf": {
    text: "Constant Rate Factor controls quality. Lower = higher quality, larger file. 18 is near-lossless, 23 is default, 28+ is low quality.",
    url: `${DOCS_BASE}/cli/render.html#encoding-settings`,
  },
  "profile.audio_codec": {
    text: "Audio codec for the output. 'aac' is the most compatible choice.",
    url: `${DOCS_BASE}/cli/render.html#encoding-settings`,
  },
  "profile.audio_bitrate": {
    text: "Audio bitrate in kbps. 128k is standard, 192k is high quality, 256k+ is near-transparent.",
    url: `${DOCS_BASE}/cli/render.html#encoding-settings`,
  },
  "profile.overlay_template": {
    text: "Path to an overlay template. Composites graphics (score, team names, logos) onto the rendered video.",
    url: `${DOCS_BASE}/guide/overlay-templates.html`,
  },
  "profile.smart_zoom": {
    text: "AI-powered smart crop tracking using the OpenAI plugin. Dynamically reframes the video to follow the action.",
    url: `${DOCS_BASE}/examples/10-smart-zoom.html`,
  },

  // ── Rendering Settings ─────────────────────────────────────
  "render.mode": {
    text: "'Short' crops/scales to profile dimensions (vertical shorts). 'Apply' keeps the original frame, only applying speed, LUT, and overlay.",
    url: `${DOCS_BASE}/cli/render.html#commands`,
  },
  "render.default_profile": {
    text: "The render profile used by default when no specific profile is selected.",
    url: `${DOCS_BASE}/guide/configuration.html#render-profiles-section`,
  },
  "render.iteration_mappings": {
    text: "Maps event types to render profiles. When iterating, each event is rendered with the profile assigned to its type.",
    url: `${DOCS_BASE}/guide/configuration.html#iterations-section`,
  },
  "render.concat": {
    text: "When rendering multiple profiles, concatenate results into a single output file instead of separate files.",
    url: `${DOCS_BASE}/cli/render.html#render-workflows`,
  },
  "render.branding": {
    text: "Branding adds a small watermark to rendered output. Disable with --no-branding if your overlay already includes branding.",
    url: `${DOCS_BASE}/guide/configuration.html#branding-section`,
  },
  "render.filter_chain": {
    text: "Renders apply filters in order: scale/crop, speed, LUT, overlay composite. Understanding the chain helps debug output issues.",
    url: `${DOCS_BASE}/cli/render.html#filter-chain-order`,
  },

  // ── Queue / Publishing ─────────────────────────────────────
  "queue.overview": {
    text: "The render queue stages clips for batch rendering and publishing. Render first, then publish to platforms via plugins.",
    url: `${DOCS_BASE}/cli/queue.html#overview`,
  },
  "queue.publish": {
    text: "Publish rendered clips to configured platform targets (YouTube, Instagram, TikTok, etc.) via plugins.",
    url: `${DOCS_BASE}/cli/queue.html`,
  },
  "queue.targets": {
    text: "Publish targets are platforms configured through plugins (e.g., youtube, instagram, tiktok). Each plugin registers its targets.",
    url: `${DOCS_BASE}/cli/queue.html#reeln-queue-targets`,
  },
  "queue.edit": {
    text: "Edit queue item metadata (title, description) before publishing. Changes are saved to the render queue file.",
    url: `${DOCS_BASE}/cli/queue.html#reeln-queue-edit`,
  },
  "queue.reel": {
    text: "Assemble rendered shorts into a highlight reel. Combines clips from a game into a single video.",
    url: `${DOCS_BASE}/cli/render.html#reeln-render-reel`,
  },

  // ── Plugin Settings ────────────────────────────────────────
  "plugins.profile": {
    text: "Plugin config profile determines which plugins are active and their settings for rendering and publishing.",
    url: `${DOCS_BASE}/guide/configuration.html#named-profiles`,
  },
  "plugins.enforce_hooks": {
    text: "When enabled, plugin hooks must complete successfully for game operations to proceed. Disable to skip plugin processing.",
    url: `${DOCS_BASE}/cli/plugins.html#plugin-extension-points`,
  },
  "plugins.manage": {
    text: "Enable, disable, install, and configure plugins per config profile. Plugins add rendering features, upload targets, and metadata enrichment.",
    url: `${DOCS_BASE}/cli/plugins.html`,
  },
  "plugins.auth": {
    text: "Authenticate with plugin services (YouTube, Instagram, etc.). Each plugin manages its own OAuth flow and token storage.",
    url: `${DOCS_BASE}/cli/plugins.html#reeln-plugins-auth`,
  },
  "plugins.hooks": {
    text: "Plugins hook into the game lifecycle: on_game_init, on_segment_process, post_render, on_publish, and more.",
    url: `${DOCS_BASE}/cli/plugins.html#plugin-extension-points`,
  },
  "plugins.registry": {
    text: "Browse available plugins, check compatibility, and install with one click. The registry is fetched from GitHub.",
    url: `${DOCS_BASE}/cli/plugins.html#plugin-registry`,
  },
  "plugins.ui_contributions": {
    text: "Plugins can contribute UI fields to the dock — render options, clip review controls, and settings panels.",
    url: `${DOCS_BASE}/cli/plugins.html#desktop-ui-contributions`,
  },

  // ── Event Types ────────────────────────────────────────────
  "events.types": {
    text: "Define the event categories for tagging clips (e.g., goal, save, penalty). Event types drive iteration mappings and metadata.",
    url: `${DOCS_BASE}/examples/04-segments-and-events.html`,
  },
  "events.team_specific": {
    text: "When enabled, creates Home/Away variants of this event type for team-specific tagging.",
    url: `${DOCS_BASE}/examples/04-segments-and-events.html`,
  },

  // ── Teams ──────────────────────────────────────────────────
  "teams.profiles": {
    text: "Team profiles store team name, short name, colors, jersey colors, logo path, and roster file for player lookup.",
    url: `${DOCS_BASE}/guide/configuration.html#team-profiles-and-rosters`,
  },
  "teams.rosters": {
    text: "CSV roster files map jersey numbers to player names. Used for automatic player name resolution during rendering.",
    url: `${DOCS_BASE}/guide/configuration.html#setting-up-rosters-for-player-number-lookup`,
  },
  "teams.levels": {
    text: "Organize teams by level (e.g., varsity, JV, club). Each level is a directory containing team profile JSON files.",
    url: `${DOCS_BASE}/guide/configuration.html#team-profiles-and-rosters`,
  },

  // ── Overlay Templates ──────────────────────────────────────
  "overlay.templates": {
    text: "Overlay templates define graphics composited onto rendered video — scoreboards, team logos, player names, and more.",
    url: `${DOCS_BASE}/guide/overlay-templates.html`,
  },
  "overlay.builtin": {
    text: "reeln includes builtin ASS subtitle templates for common overlays. Custom templates can be created as JSON or ASS files.",
    url: `${DOCS_BASE}/guide/overlay-templates.html#builtin-ass-templates`,
  },
  "overlay.variables": {
    text: "Templates use variables like {home_team}, {away_team}, {scorer}, {date} that are auto-filled from game state and event metadata.",
    url: `${DOCS_BASE}/guide/overlay-templates.html#template-variables-ass`,
  },

  // ── General Docs ───────────────────────────────────────────
  "docs.home": {
    text: "Full documentation for reeln — configuration, rendering, plugins, and workflow guides.",
    url: `${DOCS_BASE}/index.html`,
  },
  "docs.quickstart": {
    text: "Get up and running with reeln in 5 minutes — install, configure, and render your first short.",
    url: `${DOCS_BASE}/quickstart.html`,
  },
  "docs.examples": {
    text: "Step-by-step examples covering the full reeln workflow from game setup to publishing.",
    url: `${DOCS_BASE}/examples/index.html`,
  },
  "docs.cli_reference": {
    text: "Complete CLI command reference — render, game, queue, plugins, config, media, and doctor commands.",
    url: `${DOCS_BASE}/cli/index.html`,
  },

  // ── Community ──────────────────────────────────────────────
  "community.issues": {
    text: "Report bugs, request features, or get help from the community.",
    url: "https://github.com/StreamnDad/reeln-dock/issues",
  },
  "community.discussions": {
    text: "Ask questions, share ideas, or show off your setup.",
    url: "https://github.com/StreamnDad/reeln-dock/discussions",
  },
};
