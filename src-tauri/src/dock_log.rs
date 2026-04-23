use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct DockLogEvent {
    pub level: String, // "debug", "info", "warn", "error"
    pub source: String,
    pub message: String,
}

/// Emit a log event to the frontend.
pub fn emit(app: &AppHandle, level: &str, source: &str, message: &str) {
    let _ = app.emit(
        "dock:log",
        DockLogEvent {
            level: level.to_string(),
            source: source.to_string(),
            message: message.to_string(),
        },
    );
}

/// Log a CLI command about to be executed.
pub fn log_cli_command(app: &AppHandle, source: &str, program: &str, args: &[&str]) {
    let cmd_str = format!("{} {}", program, args.join(" "));
    emit(app, "info", source, &format!("$ {cmd_str}"));
}

/// Log CLI output (stdout/stderr) line by line.
pub fn log_cli_output(app: &AppHandle, source: &str, output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    for line in stdout.lines() {
        if !line.trim().is_empty() {
            emit(app, "debug", source, line);
        }
    }
    for line in stderr.lines() {
        if !line.trim().is_empty() {
            emit(app, "warn", source, line);
        }
    }

    let code = output.status.code().unwrap_or(-1);
    if output.status.success() {
        emit(app, "info", source, &format!("exit code {code}"));
    } else {
        emit(app, "error", source, &format!("exit code {code}"));
    }
}
