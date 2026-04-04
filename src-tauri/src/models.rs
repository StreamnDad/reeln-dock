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
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub url: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_preferences_default() {
        let dp = DisplayPreferences::default();
        assert!(dp.show_logos);
        assert!(dp.sections_expanded.games);
        assert!(dp.sections_expanded.teams);
        assert!(dp.sections_expanded.tournaments);
    }

    #[test]
    fn sections_expanded_default_all_true() {
        let se = SectionsExpanded::default();
        assert!(se.games);
        assert!(se.teams);
        assert!(se.tournaments);
    }

    #[test]
    fn media_info_response_from_full() {
        let info = reeln_media::MediaInfo {
            duration_secs: Some(120.5),
            fps: Some(29.97),
            width: Some(1920),
            height: Some(1080),
            codec: Some("hevc".to_string()),
        };

        let response = MediaInfoResponse::from(info);
        assert_eq!(response.duration_secs, Some(120.5));
        assert_eq!(response.fps, Some(29.97));
        assert_eq!(response.width, Some(1920));
        assert_eq!(response.height, Some(1080));
        assert_eq!(response.codec.as_deref(), Some("hevc"));
    }

    #[test]
    fn media_info_response_from_all_none() {
        let info = reeln_media::MediaInfo {
            duration_secs: None,
            fps: None,
            width: None,
            height: None,
            codec: None,
        };

        let response = MediaInfoResponse::from(info);
        assert!(response.duration_secs.is_none());
        assert!(response.fps.is_none());
        assert!(response.width.is_none());
        assert!(response.height.is_none());
        assert!(response.codec.is_none());
    }

    #[test]
    fn default_true_returns_true() {
        assert!(default_true());
    }

    #[test]
    fn team_profile_serde_roundtrip() {
        let profile = TeamProfile {
            team_name: "Acme FC".to_string(),
            short_name: "ACM".to_string(),
            level: "varsity".to_string(),
            logo_path: "/logos/acme.png".to_string(),
            roster_path: "/rosters/acme.json".to_string(),
            colors: vec!["#FF0000".to_string(), "#0000FF".to_string()],
            jersey_colors: vec!["white".to_string()],
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("founded".to_string(), serde_json::json!(2020));
                m
            },
        };

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: TeamProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.team_name, "Acme FC");
        assert_eq!(deserialized.short_name, "ACM");
        assert_eq!(deserialized.level, "varsity");
        assert_eq!(deserialized.logo_path, "/logos/acme.png");
        assert_eq!(deserialized.roster_path, "/rosters/acme.json");
        assert_eq!(deserialized.colors, vec!["#FF0000", "#0000FF"]);
        assert_eq!(deserialized.jersey_colors, vec!["white"]);
        assert_eq!(deserialized.metadata.get("founded"), Some(&serde_json::json!(2020)));
    }

    #[test]
    fn tournament_meta_serde_roundtrip_with_defaults() {
        // Serialize only the required field, test that defaults populate
        let json = r#"{"name": "Spring Cup"}"#;
        let meta: TournamentMeta = serde_json::from_str(json).unwrap();

        assert_eq!(meta.name, "Spring Cup");
        assert!(!meta.archived);
        assert_eq!(meta.notes, "");
        assert_eq!(meta.start_date, "");
        assert_eq!(meta.end_date, "");
        assert_eq!(meta.url, "");

        // Full roundtrip
        let full = TournamentMeta {
            name: "Fall Classic".to_string(),
            archived: true,
            notes: "Great tournament".to_string(),
            start_date: "2026-09-01".to_string(),
            end_date: "2026-09-15".to_string(),
            url: "https://example.com".to_string(),
        };

        let serialized = serde_json::to_string(&full).unwrap();
        let deserialized: TournamentMeta = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, "Fall Classic");
        assert!(deserialized.archived);
        assert_eq!(deserialized.notes, "Great tournament");
        assert_eq!(deserialized.start_date, "2026-09-01");
        assert_eq!(deserialized.end_date, "2026-09-15");
        assert_eq!(deserialized.url, "https://example.com");
    }
}
