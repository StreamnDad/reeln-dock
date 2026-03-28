use serde::{Deserialize, Serialize};

use reeln_state::GameState;

#[derive(Debug, Clone, Serialize)]
pub struct GameSummary {
    pub dir_path: String,
    pub state: GameState,
}

/// Team profile — matches the Python CLI format at `$CONFIG_DIR/teams/{level}/{slug}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamProfile {
    pub team_name: String,
    pub short_name: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub logo_path: String,
    #[serde(default)]
    pub roster_path: String,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub jersey_colors: Vec<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// A prompt template with its metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PromptTemplateInfo {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaInfoResponse {
    pub duration_secs: Option<f64>,
    pub fps: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
}

/// Metadata for a tournament — stored in `$OUTPUT_DIR/tournaments.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentMeta {
    pub name: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub notes: String,
}

/// Per-sidebar-mode expand/collapse default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionsExpanded {
    #[serde(default = "default_true")]
    pub games: bool,
    #[serde(default = "default_true")]
    pub teams: bool,
    #[serde(default = "default_true")]
    pub tournaments: bool,
}

impl Default for SectionsExpanded {
    fn default() -> Self {
        Self {
            games: true,
            teams: true,
            tournaments: true,
        }
    }
}

/// Display preferences persisted per-user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayPreferences {
    #[serde(default = "default_true")]
    pub show_logos: bool,
    #[serde(default)]
    pub sections_expanded: SectionsExpanded,
}

fn default_true() -> bool {
    true
}

impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            show_logos: true,
            sections_expanded: SectionsExpanded::default(),
        }
    }
}

impl From<reeln_media::MediaInfo> for MediaInfoResponse {
    fn from(info: reeln_media::MediaInfo) -> Self {
        Self {
            duration_secs: info.duration_secs,
            fps: info.fps,
            width: info.width,
            height: info.height,
            codec: info.codec,
        }
    }
}
