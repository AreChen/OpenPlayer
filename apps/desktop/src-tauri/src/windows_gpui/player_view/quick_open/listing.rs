use std::{cmp::Reverse, fs::Metadata, path::PathBuf, time::SystemTime};

use gpui::SharedString;

use super::{QuickOpenMode, is_dot_named, is_supported_media_path, previews::is_video_file};
use crate::{
    media_paths::natural_name_key,
    windows_gpui::browser_options::{BrowserOptions, Category, Sort, SortBy},
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum EntryKind {
    Folder,
    Video,
    Audio,
    Other,
}

impl EntryKind {
    pub(super) fn is_media(self) -> bool {
        matches!(self, Self::Video | Self::Audio)
    }

    pub(super) fn glyph(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Video => "frames",
            Self::Audio => "headphones",
            Self::Other => "file",
        }
    }
}

pub(super) struct Entry {
    pub(super) path: PathBuf,
    pub(super) name: SharedString,
    pub(super) kind: EntryKind,
    modified: Option<SystemTime>,
    size: Option<u64>,
}

impl Entry {
    pub(super) fn new(path: PathBuf, kind: EntryKind, metadata: Option<Metadata>) -> Self {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned()
            .into();
        Self {
            path,
            name,
            kind,
            modified: metadata.as_ref().and_then(|data| data.modified().ok()),
            size: metadata
                .filter(|data| data.is_file())
                .map(|data| data.len()),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct Filter {
    pub(super) only_media: bool,
    pub(super) category: Category,
    pub(super) show_dot_folders: bool,
    pub(super) mode: QuickOpenMode,
}

impl Filter {
    pub(super) fn new(mode: QuickOpenMode, options: BrowserOptions) -> Self {
        Self {
            only_media: options.only_media,
            category: options.category,
            show_dot_folders: false,
            mode,
        }
    }

    pub(super) fn includes(self, kind: EntryKind) -> bool {
        if self.mode == QuickOpenMode::AppendFolder {
            return kind == EntryKind::Folder;
        }
        if kind == EntryKind::Folder {
            return !self.only_media;
        }
        match self.category {
            Category::All => !self.only_media || kind.is_media(),
            Category::Video => kind == EntryKind::Video,
            Category::Audio => kind == EntryKind::Audio,
            Category::Folders => false,
        }
    }
}

pub(super) fn file_kind(path: &std::path::Path) -> EntryKind {
    if !is_supported_media_path(path) {
        EntryKind::Other
    } else if is_video_file(path) {
        EntryKind::Video
    } else {
        EntryKind::Audio
    }
}

pub(super) struct Listing {
    entries: Vec<Entry>,
    visible: Vec<usize>,
    sort: Sort,
    filter: Filter,
}

impl Listing {
    pub(super) fn new(mut entries: Vec<Entry>, sort: Sort, filter: Filter) -> Self {
        sort_entries(&mut entries, sort);
        let mut listing = Self {
            entries,
            visible: Vec::new(),
            sort,
            filter,
        };
        listing.apply_filter(filter);
        listing
    }

    pub(super) fn apply(&mut self, sort: Sort, filter: Filter) {
        if self.sort == sort && self.filter == filter {
            return;
        }
        if self.sort != sort {
            sort_entries(&mut self.entries, sort);
            self.sort = sort;
        }
        self.filter = filter;
        self.apply_filter(filter);
    }

    fn apply_filter(&mut self, filter: Filter) {
        self.visible.clear();
        self.visible.extend(
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    filter.includes(entry.kind)
                        && (filter.show_dot_folders
                            || entry.kind != EntryKind::Folder
                            || !entry.path.file_name().is_some_and(is_dot_named))
                })
                .map(|(index, _)| index),
        );
    }

    pub(super) fn len(&self) -> usize {
        self.visible.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }

    pub(super) fn get(&self, index: usize) -> Option<&Entry> {
        self.visible.get(index).map(|&index| &self.entries[index])
    }

    pub(super) fn range(&self, start: usize, end: usize) -> impl Iterator<Item = &Entry> {
        self.visible[start.min(self.len())..end.min(self.len())]
            .iter()
            .map(|&index| &self.entries[index])
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum SortValue {
    Name,
    Type(EntryKind, String),
    Modified(Option<SystemTime>),
    Size(Option<u64>),
}

fn sort_entries(entries: &mut [Entry], sort: Sort) {
    let key = |entry: &Entry| {
        let value = match sort.by {
            SortBy::Name => SortValue::Name,
            SortBy::Type => SortValue::Type(
                entry.kind,
                entry
                    .path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .unwrap_or_default(),
            ),
            SortBy::Modified => SortValue::Modified(entry.modified),
            SortBy::Size => SortValue::Size(entry.size),
        };
        (
            entry.kind != EntryKind::Folder,
            matches!(value, SortValue::Modified(None) | SortValue::Size(None)),
            (value, natural_name_key(&entry.name)),
        )
    };
    if sort.descending {
        entries.sort_by_cached_key(|entry| {
            let (is_file, no_metadata, order) = key(entry);
            (is_file, no_metadata, Reverse(order))
        });
    } else {
        entries.sort_by_cached_key(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_numbered_names_naturally_and_keeps_folders_first_in_both_directions() {
        let filter = Filter::new(QuickOpenMode::Open, BrowserOptions::default());
        let sort = Sort::default();
        let mut listing = Listing::new(
            vec![
                Entry::new("Clip-10.mp4".into(), EntryKind::Video, None),
                Entry::new("Clip-2.mp4".into(), EntryKind::Video, None),
                Entry::new("Zebra".into(), EntryKind::Folder, None),
            ],
            sort,
            filter,
        );
        let names = |listing: &Listing| {
            listing
                .range(0, listing.len())
                .map(|entry| entry.name.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&listing), ["Zebra", "Clip-2.mp4", "Clip-10.mp4"]);
        listing.apply(
            Sort {
                descending: true,
                ..sort
            },
            filter,
        );
        assert_eq!(names(&listing), ["Zebra", "Clip-10.mp4", "Clip-2.mp4"]);
    }

    #[test]
    fn only_media_hides_folders_and_non_media_until_turned_off() {
        let filter = Filter::new(
            QuickOpenMode::Open,
            BrowserOptions {
                only_media: true,
                ..BrowserOptions::default()
            },
        );
        let sort = Sort::default();
        let mut listing = Listing::new(
            vec![
                Entry::new("Clips".into(), EntryKind::Folder, None),
                Entry::new("Film.mp4".into(), EntryKind::Video, None),
                Entry::new("Music.wav".into(), EntryKind::Audio, None),
                Entry::new("Notes.txt".into(), EntryKind::Other, None),
            ],
            sort,
            filter,
        );
        assert_eq!(listing.len(), 2);
        assert!(
            listing
                .range(0, listing.len())
                .all(|entry| entry.kind.is_media())
        );
        listing.apply(
            sort,
            Filter {
                only_media: false,
                ..filter
            },
        );
        assert_eq!(listing.len(), 4);
        assert!(
            listing
                .get(0)
                .is_some_and(|entry| entry.kind == EntryKind::Folder)
        );
    }
}
