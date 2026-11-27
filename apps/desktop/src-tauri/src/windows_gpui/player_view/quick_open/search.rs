use std::{
    fs, io,
    os::windows::fs::MetadataExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui::{AppContext, Context, Entity, Subscription, Task, Window};
use ignore::{DirEntry, WalkBuilder};
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

use super::{
    AddressInput, DirectoryEntries, Entry, PlayerView, QuickOpenMode, is_supported_media_path,
};

const SEARCH_DELAY: Duration = Duration::from_millis(180);
pub(super) const MAX_SEARCH_RESULTS: usize = 1000;

pub(super) struct QuickSearch {
    pub(super) input: Entity<AddressInput>,
    pub(super) results: Option<DirectoryEntries>,
    pub(super) truncated: bool,
    query: Vec<String>,
    _observer: Subscription,
    work: Option<SearchWork>,
}

struct SearchWork {
    cancelled: Arc<AtomicBool>,
    _task: Task<()>,
}

impl Drop for SearchWork {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl PlayerView {
    pub(super) fn focus_quick_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        if picker.show_media_favorites || picker.navigation.is_some() {
            return;
        }
        let search = picker.search.get_or_insert_with(|| {
            let input = cx.new(|cx| AddressInput::new(String::new(), cx));
            let observer = cx.observe(&input, |view, input, cx| {
                if let Some(text) = input.read(cx).unmarked_text() {
                    view.update_quick_search(search_tokens(text), cx);
                }
            });
            QuickSearch {
                input,
                results: None,
                truncated: false,
                query: Vec::new(),
                _observer: observer,
                work: None,
            }
        });
        picker.address = None;
        picker.address_unavailable = false;
        picker.context = None;
        search.input.update(cx, |input, cx| input.focus(window, cx));
        self.restore_quick_focus = false;
        cx.notify();
    }

    pub(super) fn close_quick_search(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &mut self.quick_open
            && picker.search.take().is_some()
        {
            picker.reset_listing(cx);
            self.restore_quick_focus = true;
            cx.notify();
        }
    }

    fn update_quick_search(&mut self, query: Vec<String>, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        if picker
            .search
            .as_ref()
            .is_none_or(|search| search.query == query)
        {
            return;
        }
        picker.reset_listing(cx);
        let search = picker.search.as_mut().expect("search is open");
        search.work = None;
        search.query = query.clone();
        search.truncated = false;
        search.results = (!query.is_empty()).then_some(DirectoryEntries::Loading);
        cx.notify();
        if query.is_empty() {
            return;
        }
        let directory = picker.directory.clone();
        let mode = picker.mode;
        let executor = cx.background_executor().clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = cancelled.clone();
        let task = cx.spawn(async move |view, cx| {
            executor.timer(SEARCH_DELAY).await;
            let worker = request.clone();
            let results = executor
                .spawn(async move { scan(&directory, mode, &query, &worker) })
                .await;
            if request.load(Ordering::Relaxed) {
                return;
            }
            let _ = view.update(cx, |view, cx| {
                let Some(search) = view
                    .quick_open
                    .as_mut()
                    .and_then(|picker| picker.search.as_mut())
                else {
                    return;
                };
                search.results = Some(match results {
                    Ok((entries, truncated)) => {
                        search.truncated = truncated;
                        DirectoryEntries::Ready(entries)
                    }
                    Err(_) => DirectoryEntries::Unreadable,
                });
                cx.notify();
            });
        });
        search.work = Some(SearchWork {
            cancelled,
            _task: task,
        });
    }
}

fn search_tokens(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

fn scan(
    root: &Path,
    mode: QuickOpenMode,
    query: &[String],
    cancelled: &AtomicBool,
) -> io::Result<(Vec<Entry>, bool)> {
    fs::read_dir(root)?;
    let mut entries = Vec::new();
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .follow_links(false)
        .filter_entry(searchable_entry)
        .build();
    for result in walker {
        if cancelled.load(Ordering::Relaxed) {
            return Ok((Vec::new(), false));
        }
        let Ok(candidate) = result else { continue };
        // Keep the root event in the walker so its ignore stack stays balanced.
        if candidate.depth() == 0 {
            continue;
        }
        let Some(kind) = candidate.file_type() else {
            continue;
        };
        let is_directory = kind.is_dir();
        if !is_directory
            && (mode == QuickOpenMode::AppendFolder
                || !kind.is_file()
                || !is_supported_media_path(candidate.path()))
        {
            continue;
        }
        let name = candidate.file_name().to_string_lossy();
        let lowercase = name.to_lowercase();
        if query.iter().any(|token| !lowercase.contains(token)) {
            continue;
        }
        entries.push(Entry {
            name: name.into_owned().into(),
            path: candidate.into_path(),
            is_directory,
        });
        if entries.len() > MAX_SEARCH_RESULTS {
            break;
        }
    }
    let truncated = entries.len() > MAX_SEARCH_RESULTS;
    entries.truncate(MAX_SEARCH_RESULTS);
    entries.sort_by_cached_key(|entry| {
        (
            !entry.is_directory,
            entry.name.to_lowercase(),
            entry.path.clone(),
        )
    });
    Ok((entries, truncated))
}

fn searchable_entry(entry: &DirEntry) -> bool {
    entry.depth() == 0
        || (!entry.path_is_symlink()
            && (entry.file_type().is_none_or(|kind| !kind.is_dir())
                || entry
                    .metadata()
                    .is_ok_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0)))
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "openplayer-quick-search-{}-{nonce}",
                std::process::id()
            ));
            let nested = root.join("Nested");
            fs::create_dir_all(nested.join("Family Film")).unwrap();
            fs::write(nested.join("Family [Preview] Film.MP4"), []).unwrap();
            fs::write(nested.join("Family [Preview] Film.txt"), []).unwrap();
            fs::write(root.join("Family Notes.MP4"), []).unwrap();
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn searches_nested_supported_names_and_respects_picker_mode_and_cancellation() {
        let root = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let names = |mode, query: &str| {
            let (results, truncated) =
                scan(&root.0, mode, &search_tokens(query), &cancelled).unwrap();
            assert!(!truncated);
            results
                .into_iter()
                .map(|entry| entry.path)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            names(QuickOpenMode::Open, "FILM family"),
            [
                root.0.join("Nested").join("Family Film"),
                root.0.join("Nested").join("Family [Preview] Film.MP4"),
            ]
        );
        assert_eq!(
            names(QuickOpenMode::AppendFiles, "FiLM [preview]"),
            [root.0.join("Nested").join("Family [Preview] Film.MP4")]
        );
        assert_eq!(
            names(QuickOpenMode::AppendFolder, "FILM family"),
            [root.0.join("Nested").join("Family Film")]
        );
        assert!(names(QuickOpenMode::Open, "openplayer-quick-search").is_empty());
        cancelled.store(true, Ordering::Relaxed);
        assert!(names(QuickOpenMode::Open, "film").is_empty());
    }
}
