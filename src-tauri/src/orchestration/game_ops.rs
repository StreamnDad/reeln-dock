use std::path::{Path, PathBuf};
use std::sync::Arc;

use reeln_config::AppConfig;
use reeln_media::{ConcatOptions, MediaBackend};
use reeln_sport::{SportRegistry, segment_dir_name, make_segments};
use reeln_state::{GameInfo, GameState};

use super::progress::ProgressReporter;

/// Parameters for creating a new game.
pub struct InitGameParams {
    pub sport: String,
    pub home_team: String,
    pub away_team: String,
    pub date: String,
    pub venue: Option<String>,
    pub game_time: Option<String>,
    pub level: Option<String>,
    pub tournament: Option<String>,
    pub period_length: Option<u32>,
}

/// Initialize a new game: create directory, segment subdirs, and game.json.
pub fn init_game(
    config: &AppConfig,
    sport_registry: &SportRegistry,
    params: InitGameParams,
) -> Result<(PathBuf, GameState), String> {
    let sport_alias = sport_registry
        .get_sport(&params.sport)
        .map_err(|e| e.to_string())?;

    let output_dir = config
        .paths
        .output_dir
        .as_ref()
        .ok_or_else(|| "output_dir not configured".to_string())?;

    let game_number = reeln_state::detect_next_game_number(
        output_dir,
        &params.date,
        &params.home_team,
        &params.away_team,
    );

    let period_length = params
        .period_length
        .or(sport_alias.duration_minutes)
        .unwrap_or(0);

    let game_info = GameInfo {
        date: params.date,
        home_team: params.home_team.clone(),
        away_team: params.away_team.clone(),
        sport: params.sport.clone(),
        game_number,
        venue: params.venue.unwrap_or_default(),
        game_time: params.game_time.unwrap_or_default(),
        period_length,
        description: String::new(),
        thumbnail: String::new(),
        level: params.level.unwrap_or_default(),
        home_slug: params.home_team.to_lowercase().replace(' ', "-"),
        away_slug: params.away_team.to_lowercase().replace(' ', "-"),
        tournament: params.tournament.unwrap_or_default(),
    };

    let game_dir =
        reeln_state::create_game_directory(output_dir, &game_info).map_err(|e| e.to_string())?;

    // Create segment subdirectories
    let segments = make_segments(sport_alias, None).map_err(|e| e.to_string())?;
    for seg in &segments {
        let seg_dir = game_dir.join(&seg.alias);
        std::fs::create_dir_all(&seg_dir).map_err(|e| e.to_string())?;
    }

    let state = GameState {
        game_info,
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

    reeln_state::save_game_state(&state, &game_dir).map_err(|e| e.to_string())?;

    Ok((game_dir, state))
}

/// Process a single segment: find videos, concat them, update state.
pub fn process_segment(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    game_dir: &Path,
    segment_number: u32,
    reporter: Option<&ProgressReporter>,
) -> Result<GameState, String> {
    let mut state = reeln_state::load_game_state(game_dir).map_err(|e| e.to_string())?;

    let sport_alias = reeln_sport::SportRegistry::default()
        .get_sport(&state.game_info.sport)
        .map_err(|e| e.to_string())?
        .clone();

    let seg_alias = segment_dir_name(&sport_alias, segment_number);
    let seg_dir = game_dir.join(&seg_alias);

    if let Some(r) = reporter {
        r.report("discover", 0.0, &format!("Finding videos in {seg_alias}"));
    }

    // Optionally collect replays from source_dir
    if let Some(ref source_dir) = config.paths.source_dir {
        if source_dir.exists() {
            let _ = reeln_state::collect_replays(
                source_dir,
                &config.paths.source_glob,
                &seg_dir,
            );
        }
    }

    let videos = reeln_state::find_segment_videos(&seg_dir, &seg_alias)
        .map_err(|e| e.to_string())?;

    if videos.is_empty() {
        return Err(format!("No videos found in {}", seg_dir.display()));
    }

    // Create events for discovered clips and persist immediately.
    // This ensures clips are visible in the UI even if concat is cancelled.
    let existing_clips: std::collections::HashSet<String> =
        state.events.iter().map(|e| e.clip.clone()).collect();
    let now = chrono::Utc::now().to_rfc3339();

    for video in &videos {
        let rel_path = video
            .strip_prefix(game_dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| video.display().to_string());

        if !existing_clips.contains(&rel_path) {
            state.events.push(reeln_state::GameEvent {
                id: uuid::Uuid::new_v4().to_string(),
                clip: rel_path,
                segment_number,
                event_type: String::new(),
                player: String::new(),
                created_at: now.clone(),
                metadata: std::collections::HashMap::new(),
            });
        }
    }

    if !state.segments_processed.contains(&segment_number) {
        state.segments_processed.push(segment_number);
        state.segments_processed.sort();
    }

    reeln_state::save_game_state(&state, game_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report(
            "events",
            0.1,
            &format!("Discovered {} clips, concatenating...", videos.len()),
        );
    }

    // Concat is the slow step — clips are already persisted above
    let output = seg_dir.join(format!("{seg_alias}_merged.mp4"));
    let segment_paths: Vec<&Path> = videos.iter().map(|p| p.as_path()).collect();

    let opts = ConcatOptions {
        copy: true,
        video_codec: config.video.codec.clone(),
        crf: config.video.crf,
        audio_codec: config.video.audio_codec.clone(),
        audio_rate: 48000,
    };

    backend
        .concat(&segment_paths, &output, &opts)
        .map_err(|e| e.to_string())?;

    state
        .segment_outputs
        .push(output.display().to_string());

    reeln_state::save_game_state(&state, game_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("done", 1.0, &format!("{seg_alias} processed"));
    }

    Ok(state)
}

/// Merge all segment outputs into a single highlights file.
pub fn merge_highlights(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    game_dir: &Path,
    reporter: Option<&ProgressReporter>,
) -> Result<GameState, String> {
    let mut state = reeln_state::load_game_state(game_dir).map_err(|e| e.to_string())?;

    if state.segment_outputs.is_empty() {
        return Err("No segment outputs to merge".to_string());
    }

    if let Some(r) = reporter {
        r.report(
            "concat",
            0.1,
            &format!(
                "Merging {} segments into highlights",
                state.segment_outputs.len()
            ),
        );
    }

    let highlights_output = game_dir.join("highlights.mp4");
    let segment_paths: Vec<PathBuf> = state
        .segment_outputs
        .iter()
        .map(PathBuf::from)
        .collect();
    let segment_refs: Vec<&Path> = segment_paths.iter().map(|p| p.as_path()).collect();

    let opts = ConcatOptions {
        copy: true,
        video_codec: config.video.codec.clone(),
        crf: config.video.crf,
        audio_codec: config.video.audio_codec.clone(),
        audio_rate: 48000,
    };

    backend
        .concat(&segment_refs, &highlights_output, &opts)
        .map_err(|e| e.to_string())?;

    state.highlighted = true;
    state.highlights_output = highlights_output.display().to_string();

    reeln_state::save_game_state(&state, game_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("done", 1.0, "Highlights merged");
    }

    Ok(state)
}

/// Mark a game as finished.
pub fn finish_game(game_dir: &Path) -> Result<GameState, String> {
    let mut state = reeln_state::load_game_state(game_dir).map_err(|e| e.to_string())?;
    state.finished = true;
    state.finished_at = chrono::Utc::now().to_rfc3339();
    reeln_state::save_game_state(&state, game_dir).map_err(|e| e.to_string())?;
    Ok(state)
}
