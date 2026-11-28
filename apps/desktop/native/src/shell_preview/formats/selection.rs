use std::collections::HashSet;

use super::{PREVIEW_FORMATS, ShellPreviewFormat};

pub(in crate::shell_preview) fn filter_preview_formats(
    selected_extensions: &[String],
) -> Result<Vec<ShellPreviewFormat>, String> {
    let selected: HashSet<String> = selected_extensions
        .iter()
        .map(|extension| normalize_extension(extension))
        .filter(|extension| !extension.is_empty())
        .collect();

    if selected.is_empty() {
        return Err("select at least one preview format".to_string());
    }

    let formats: Vec<ShellPreviewFormat> = PREVIEW_FORMATS
        .iter()
        .copied()
        .filter(|format| selected.contains(format.extension))
        .collect();

    if formats.len() != selected.len() {
        let known: HashSet<&str> = formats.iter().map(|format| format.extension).collect();
        let mut unknown: Vec<String> = selected
            .into_iter()
            .filter(|extension| !known.contains(extension.as_str()))
            .collect();
        unknown.sort();
        return Err(format!(
            "unsupported preview format: {}",
            unknown.join(", ")
        ));
    }

    Ok(formats)
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}
