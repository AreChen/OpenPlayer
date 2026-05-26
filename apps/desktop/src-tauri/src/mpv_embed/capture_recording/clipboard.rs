use super::super::*;

pub(in crate::mpv_embed) fn copy_image_file_to_clipboard(path: &Path) -> Result<(), String> {
    let image = image::ImageReader::open(path)
        .map_err(|error| format!("failed to open screenshot for clipboard: {error}"))?
        .decode()
        .map_err(|error| format!("failed to decode screenshot for clipboard: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| format!("failed to access clipboard: {error}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|error| format!("failed to copy screenshot to clipboard: {error}"))
}
