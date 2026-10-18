// The Overlay tab's commands. The overlay is Windows only: elsewhere get_overlay answers null (the tab stays hidden)
// and the rest refuse.

#[cfg(windows)]
mod imp {
    use tauri::{AppHandle, Manager, State};

    use crate::overlay::{KeySlot, Overlay, OverlayView, SettingsPatch};

    #[tauri::command]
    pub fn get_overlay(overlay: State<'_, Overlay>) -> Option<OverlayView> {
        Some(overlay.view())
    }

    // Async so stopping (which waits for the radar's threads) runs off the main thread.
    #[tauri::command]
    pub async fn set_overlay_enabled(enabled: bool, app: AppHandle) -> Result<OverlayView, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let overlay = app.state::<Overlay>();
            overlay.set_enabled(enabled);
            overlay.view()
        })
        .await
        .map_err(|err| err.to_string())
    }

    // Async: switching the radar widget off waits for its threads.
    #[tauri::command]
    pub async fn set_overlay_settings(
        patch: SettingsPatch,
        app: AppHandle,
    ) -> Result<OverlayView, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let overlay = app.state::<Overlay>();
            overlay.update(patch);
            overlay.view()
        })
        .await
        .map_err(|err| err.to_string())
    }

    #[tauri::command]
    pub fn edit_overlay_layout(overlay: State<'_, Overlay>) -> OverlayView {
        overlay.request_edit();
        overlay.view()
    }

    #[tauri::command]
    pub fn reset_overlay_layout(overlay: State<'_, Overlay>) -> OverlayView {
        overlay.reset_layout();
        overlay.view()
    }

    // Async: it waits up to 10 s for the key press.
    #[tauri::command]
    pub async fn capture_overlay_key(
        which: KeySlot,
        app: AppHandle,
    ) -> Result<OverlayView, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let overlay = app.state::<Overlay>();
            overlay.capture_key(which)?;
            Ok(overlay.view())
        })
        .await
        .map_err(|err| err.to_string())?
    }

    #[tauri::command]
    pub fn reset_overlay_key(which: KeySlot, overlay: State<'_, Overlay>) -> OverlayView {
        overlay.reset_key(which);
        overlay.view()
    }
}

#[cfg(not(windows))]
mod imp {
    const WINDOWS_ONLY: &str = "The overlay is Windows only.";

    #[tauri::command]
    pub fn get_overlay() -> Option<()> {
        None
    }

    #[tauri::command]
    pub fn set_overlay_enabled(_enabled: bool) -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }

    #[tauri::command]
    pub fn set_overlay_settings(_patch: serde_json::Value) -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }

    #[tauri::command]
    pub fn edit_overlay_layout() -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }

    #[tauri::command]
    pub fn reset_overlay_layout() -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }

    #[tauri::command]
    pub fn capture_overlay_key(_which: String) -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }

    #[tauri::command]
    pub fn reset_overlay_key(_which: String) -> Result<(), String> {
        Err(WINDOWS_ONLY.to_string())
    }
}

pub use imp::*;
