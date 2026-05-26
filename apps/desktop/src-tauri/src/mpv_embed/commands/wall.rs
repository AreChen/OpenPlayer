use super::super::*;

#[tauri::command]
pub async fn mpv_wall_open(
    app: AppHandle,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_open_for_app(&app, state.inner(), tiles)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_layout(
    app: AppHandle,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_layout_for_app(&app, state.inner(), tiles)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_snapshot(app: AppHandle) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_snapshot(state.inner())
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_close(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_close(&app, state.inner())
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_set_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_set_visible(&app, state.inner(), visible)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}
