use std::{
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

use tauri::{AppHandle, Emitter, Manager};

use crate::logger;

const SOUND_RESOURCE_PATH: &str = "sounds/workout-reminder.wav";
const FALLBACK_EVENT: &str = "gym://play-reminder-sound";

pub fn play_workout_reminder(app_handle: &AppHandle) -> Result<(), String> {
    let sound_path = app_handle
        .path()
        .resource_dir()
        .map_err(|error| format!("could not resolve resource directory: {error}"))?
        .join(SOUND_RESOURCE_PATH);

    if !sound_path.is_file() {
        return emit_webview_fallback(
            app_handle,
            format!("sound resource is missing: {}", sound_path.display()),
        );
    }

    let fallback_handle = app_handle.clone();
    thread::spawn(move || {
        if let Err(error) = play_with_system_player(&sound_path) {
            if let Err(fallback_error) = emit_webview_fallback(&fallback_handle, error) {
                logger::warn(fallback_error);
            }
        }
    });

    Ok(())
}

fn emit_webview_fallback(app_handle: &AppHandle, reason: String) -> Result<(), String> {
    logger::warn(format!(
        "native reminder sound unavailable; trying webview playback: {reason}"
    ));
    app_handle
        .emit(FALLBACK_EVENT, ())
        .map_err(|error| format!("could not emit sound fallback event: {error}"))
}

fn play_with_system_player(sound_path: &PathBuf) -> Result<(), String> {
    let mut last_error = String::from("no supported audio player is available");

    for player in player_candidates() {
        match Command::new(player)
            .arg(sound_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => last_error = format!("{player} exited with {status}"),
            Err(error) => last_error = format!("could not start {player}: {error}"),
        }
    }

    Err(last_error)
}

#[cfg(target_os = "macos")]
fn player_candidates() -> &'static [&'static str] {
    &["/usr/bin/afplay"]
}

#[cfg(target_os = "linux")]
fn player_candidates() -> &'static [&'static str] {
    &["paplay", "pw-play", "aplay"]
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn player_candidates() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_platform_has_an_audio_strategy() {
        assert!(!player_candidates().is_empty());
    }

    #[test]
    fn bundled_sound_has_the_expected_name() {
        assert!(SOUND_RESOURCE_PATH.ends_with("workout-reminder.wav"));
    }
}
