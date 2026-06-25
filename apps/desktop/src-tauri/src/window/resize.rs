use tauri::{AppHandle, CursorIcon, PhysicalPosition, PhysicalSize, Position, Size};
use tauri_runtime::ResizeDirection;

use super::{MIN_MAIN_WINDOW_HEIGHT, MIN_MAIN_WINDOW_WIDTH, main_window, overlay, overlay_window};

pub(crate) fn window_start_resize(app: AppHandle, direction: String) -> Result<(), String> {
    let direction = resize_direction_from_str(&direction)?;

    main_window(&app)?
        .as_ref()
        .window()
        .start_resize_dragging(direction)
        .map_err(|error| error.to_string())
}

pub(crate) fn window_set_resize_cursor(
    app: AppHandle,
    direction: Option<String>,
) -> Result<(), String> {
    let icon = match direction.as_deref() {
        Some("East") => CursorIcon::EResize,
        Some("North") => CursorIcon::NResize,
        Some("NorthEast") => CursorIcon::NeResize,
        Some("NorthWest") => CursorIcon::NwResize,
        Some("South") => CursorIcon::SResize,
        Some("SouthEast") => CursorIcon::SeResize,
        Some("SouthWest") => CursorIcon::SwResize,
        Some("West") => CursorIcon::WResize,
        Some("Default") | None => CursorIcon::Default,
        Some(direction) => return Err(format!("invalid resize cursor direction: {direction}")),
    };

    main_window(&app)?
        .set_cursor_icon(icon)
        .map_err(|error| error.to_string())?;
    if let Some(overlay) = overlay_window(&app) {
        overlay
            .set_cursor_icon(icon)
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn resize_direction_from_str(direction: &str) -> Result<ResizeDirection, String> {
    Ok(match direction {
        "East" => ResizeDirection::East,
        "North" => ResizeDirection::North,
        "NorthEast" => ResizeDirection::NorthEast,
        "NorthWest" => ResizeDirection::NorthWest,
        "South" => ResizeDirection::South,
        "SouthEast" => ResizeDirection::SouthEast,
        "SouthWest" => ResizeDirection::SouthWest,
        "West" => ResizeDirection::West,
        _ => return Err(format!("invalid resize direction: {direction}")),
    })
}

pub(crate) fn window_apply_resize_delta(
    app: AppHandle,
    direction: String,
    delta_x: f64,
    delta_y: f64,
) -> Result<(), String> {
    if !delta_x.is_finite() || !delta_y.is_finite() {
        return Err("invalid resize delta".to_string());
    }

    let direction = resize_direction_from_str(&direction)?;
    let main = main_window(&app)?;
    if main.is_fullscreen().map_err(|error| error.to_string())?
        || main.is_maximized().map_err(|error| error.to_string())?
    {
        return Ok(());
    }

    let position = main.outer_position().map_err(|error| error.to_string())?;
    let size = main.outer_size().map_err(|error| error.to_string())?;
    let old_width = size.width as i32;
    let old_height = size.height as i32;
    let dx = delta_x.round() as i32;
    let dy = delta_y.round() as i32;
    let mut x = position.x;
    let mut y = position.y;
    let mut width = old_width;
    let mut height = old_height;

    if resize_direction_has_west_edge(direction) {
        x += dx;
        width -= dx;
    }
    if resize_direction_has_east_edge(direction) {
        width += dx;
    }
    if resize_direction_has_north_edge(direction) {
        y += dy;
        height -= dy;
    }
    if resize_direction_has_south_edge(direction) {
        height += dy;
    }

    if width < MIN_MAIN_WINDOW_WIDTH {
        if resize_direction_has_west_edge(direction) {
            x -= MIN_MAIN_WINDOW_WIDTH - width;
        }
        width = MIN_MAIN_WINDOW_WIDTH;
    }
    if height < MIN_MAIN_WINDOW_HEIGHT {
        if resize_direction_has_north_edge(direction) {
            y -= MIN_MAIN_WINDOW_HEIGHT - height;
        }
        height = MIN_MAIN_WINDOW_HEIGHT;
    }

    main.set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|error| error.to_string())?;
    main.set_size(Size::Physical(PhysicalSize {
        width: width as u32,
        height: height as u32,
    }))
    .map_err(|error| error.to_string())?;
    overlay::sync_overlay_to_main_after_resize(&app);
    Ok(())
}

fn resize_direction_has_west_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::West | ResizeDirection::NorthWest | ResizeDirection::SouthWest
    )
}

fn resize_direction_has_east_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::East | ResizeDirection::NorthEast | ResizeDirection::SouthEast
    )
}

fn resize_direction_has_north_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::North | ResizeDirection::NorthEast | ResizeDirection::NorthWest
    )
}

fn resize_direction_has_south_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::South | ResizeDirection::SouthEast | ResizeDirection::SouthWest
    )
}
