use tauri::{AppHandle, Manager};

use crate::domain::Settings;

pub const REMINDER_WINDOW_LABEL: &str = "reminder";
pub const MAIN_WINDOW_LABEL: &str = "main";

pub fn present_reminder(app_handle: &AppHandle, settings: &Settings, force: bool) {
    if !settings.popup_enabled && !force {
        return;
    }

    let Some(window) = app_handle.get_webview_window(REMINDER_WINDOW_LABEL) else {
        eprintln!("reminder window is not configured");
        return;
    };

    if let Err(error) = window.set_always_on_top(settings.always_on_top) {
        eprintln!("could not update reminder always-on-top state: {error}");
    }

    match window.is_visible() {
        Ok(true) => {}
        Ok(false) => {
            if let Err(error) = window.show() {
                eprintln!("could not show reminder window: {error}");
                return;
            }
            if let Err(error) = window.unminimize() {
                eprintln!("could not restore reminder window: {error}");
            }
            if let Err(error) = window.set_focus() {
                eprintln!("could not focus reminder window: {error}");
            }
        }
        Err(error) => eprintln!("could not read reminder window visibility: {error}"),
    }
}

pub fn show_main(app_handle: &AppHandle) {
    let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    if let Err(error) = window.show() {
        eprintln!("could not show main window: {error}");
        return;
    }
    if let Err(error) = window.unminimize() {
        eprintln!("could not restore main window: {error}");
    }
    if let Err(error) = window.set_focus() {
        eprintln!("could not focus main window: {error}");
    }
}

pub fn hide_reminder(app_handle: &AppHandle) {
    let Some(window) = app_handle.get_webview_window(REMINDER_WINDOW_LABEL) else {
        return;
    };
    if let Err(error) = window.hide() {
        eprintln!("could not hide reminder window: {error}");
    }
}
