//! Shared test utilities — compiled only during `cargo test`.

use std::path::Path;
use std::sync::Arc;

use reeln_media::{ConcatOptions, MediaBackend, MediaError, MediaInfo, RenderPlan, RenderResult};

/// A mock `MediaBackend` that creates empty output files instead of encoding video.
pub struct MockMediaBackend;

impl MediaBackend for MockMediaBackend {
    fn probe(&self, _path: &Path) -> Result<MediaInfo, MediaError> {
        Ok(MediaInfo {
            duration_secs: Some(30.0),
            fps: Some(60.0),
            width: Some(1920),
            height: Some(1080),
            codec: Some("h264".to_string()),
        })
    }

    fn concat(
        &self,
        _segments: &[&Path],
        output: &Path,
        _opts: &ConcatOptions,
    ) -> Result<(), MediaError> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(MediaError::Io)?;
        }
        std::fs::write(output, b"mock_concat_output").map_err(MediaError::Io)?;
        Ok(())
    }

    fn render(&self, plan: &RenderPlan) -> Result<RenderResult, MediaError> {
        if let Some(parent) = plan.output.parent() {
            std::fs::create_dir_all(parent).map_err(MediaError::Io)?;
        }
        std::fs::write(&plan.output, b"mock_render_output").map_err(MediaError::Io)?;
        Ok(RenderResult {
            output: plan.output.clone(),
            duration_secs: 30.0,
        })
    }
}

/// Construct a mock `Arc<dyn MediaBackend>` for tests.
pub fn mock_backend() -> Arc<dyn MediaBackend> {
    Arc::new(MockMediaBackend)
}

/// Create a minimal game.json in the given directory and return the GameState.
pub fn create_test_game(game_dir: &Path) -> reeln_state::GameState {
    std::fs::create_dir_all(game_dir).unwrap();
    let state = reeln_state::GameState {
        game_info: reeln_state::GameInfo {
            date: "2026-04-03".to_string(),
            home_team: "Team A".to_string(),
            away_team: "Team B".to_string(),
            sport: "soccer".to_string(),
            game_number: 1,
            venue: String::new(),
            game_time: String::new(),
            period_length: 45,
            description: String::new(),
            thumbnail: String::new(),
            level: String::new(),
            home_slug: "team-a".to_string(),
            away_slug: "team-b".to_string(),
            tournament: "Test League".to_string(),
        },
        segments_processed: vec![],
        highlighted: false,
        finished: false,
        created_at: chrono::Utc::now().to_rfc3339(),
        finished_at: String::new(),
        renders: vec![],
        events: vec![],
        livestreams: std::collections::HashMap::new(),
        segment_outputs: vec![],
        highlights_output: String::new(),
    };
    reeln_state::save_game_state(&state, game_dir).unwrap();
    state
}

/// Create a minimal reeln config JSON file and return AppConfig.
pub fn create_test_config(path: &Path) -> reeln_config::AppConfig {
    let config = reeln_config::AppConfig::default();
    let json = serde_json::to_string_pretty(&config).unwrap();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, json).unwrap();
    config
}
