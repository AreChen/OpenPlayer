use std::{
    fs, io,
    os::windows::fs::MetadataExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use gpui::{AppContext, Context, Entity, Subscription, Task, Window};
use ignore::{DirEntry, WalkBuilder};
use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

use super::{
    AddressInput, DirectoryEntries, Entry, EntryKind, Filter, Listing, PlayerView, Sort, file_kind,
    is_dot_named,
};

const SEARCH_DELAY: Duration = Duration::from_millis(180);
const SEARCH_PROGRESS_INTERVAL: Duration = Duration::from_millis(140);
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
    progress: Arc<SearchProgress>,
    task: Option<Task<()>>,
    _progress_task: Task<()>,
}

impl Drop for SearchWork {
    fn drop(&mut self) {
        self.progress.cancelled.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct SearchProgress {
    cancelled: AtomicBool,
    checked: AtomicUsize,
    matches: AtomicUsize,
}

impl SearchProgress {
    fn counts(&self) -> (usize, usize) {
        let matches = self.matches.load(Ordering::Acquire);
        (self.checked.load(Ordering::Relaxed), matches)
    }
}

impl QuickSearch {
    pub(super) fn progress(&self) -> Option<(usize, usize)> {
        self.work.as_ref().map(|work| work.progress.counts())
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
        picker.toolbar_menu = None;
        search.input.update(cx, |input, cx| input.focus(window, cx));
        self.restore_quick_focus = false;
        cx.notify();
    }

    pub(super) fn close_quick_search(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &mut self.quick_open
            && let Some(search) = picker.search.take()
        {
            picker.reset_listing(cx);
            if search.results.is_some() {
                picker.restart_listing_motion();
            }
            self.restore_quick_focus = true;
            cx.notify();
        }
    }

    fn update_quick_search(&mut self, query: Vec<String>, cx: &mut Context<Self>) {
        if let Some(search) = self
            .quick_open
            .as_mut()
            .and_then(|picker| picker.search.as_mut())
            && search.query != query
        {
            search.query = query;
            self.reload_quick_search(cx);
        }
    }

    pub(super) fn reload_quick_search(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        let Some(query) = picker.search.as_ref().map(|search| search.query.clone()) else {
            return;
        };
        picker.reset_listing(cx);
        if query.is_empty() {
            picker.restart_listing_motion();
        }
        let search = picker.search.as_mut().expect("search is open");
        search.work = None;
        search.truncated = false;
        search.results = (!query.is_empty()).then_some(DirectoryEntries::Loading);
        cx.notify();
        if query.is_empty() {
            return;
        }
        let directory = picker.directory.clone();
        let sort = picker.sort;
        let filter = picker.filter;
        let executor = cx.background_executor().clone();
        let progress = Arc::new(SearchProgress::default());
        let request = progress.clone();
        let monitor = progress.clone();
        let timer = executor.clone();
        let progress_task = cx.spawn(async move |view, cx| {
            let mut previous = (0, 0);
            loop {
                timer.timer(SEARCH_PROGRESS_INTERVAL).await;
                if monitor.cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let current = monitor.counts();
                if current != previous {
                    previous = current;
                    if view.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            }
        });
        let task = cx.spawn(async move |view, cx| {
            executor.timer(SEARCH_DELAY).await;
            let worker = request.clone();
            let results = executor
                .spawn(async move { scan(&directory, sort, filter, &query, &worker) })
                .await;
            if request.cancelled.load(Ordering::Relaxed) {
                return;
            }
            let _ = view.update(cx, |view, cx| {
                let Some(picker) = view.quick_open.as_mut() else {
                    return;
                };
                let Some(search) = picker.search.as_mut() else {
                    return;
                };
                if search
                    .work
                    .as_ref()
                    .is_none_or(|work| !Arc::ptr_eq(&work.progress, &request))
                {
                    return;
                }
                if let Some(task) = search.work.take().and_then(|mut work| work.task.take()) {
                    task.detach();
                }
                search.results = Some(match results {
                    Ok((entries, truncated)) => {
                        search.truncated = truncated;
                        DirectoryEntries::Ready(entries)
                    }
                    Err(_) => DirectoryEntries::Unreadable,
                });
                picker.apply_listing_options();
                picker.restart_listing_motion();
                cx.notify();
            });
        });
        search.work = Some(SearchWork {
            progress,
            task: Some(task),
            _progress_task: progress_task,
        });
    }
}

fn search_tokens(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

fn scan(
    root: &Path,
    sort: Sort,
    filter: Filter,
    query: &[String],
    progress: &SearchProgress,
) -> io::Result<(Listing, bool)> {
    if progress.cancelled.load(Ordering::Relaxed) {
        return Ok((Listing::new(Vec::new(), sort, filter), false));
    }
    fs::read_dir(root)?;
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut walker = WalkBuilder::new(root)
        .standard_filters(false)
        .follow_links(false)
        .filter_entry(move |entry| searchable_entry(entry, filter.show_dot_folders))
        .build();
    while !progress.cancelled.load(Ordering::Relaxed) {
        let Some(result) = walker.next() else { break };
        let Ok(candidate) = result else { continue };
        // Keep the root event in the walker so its ignore stack stays balanced.
        if candidate.depth() == 0 {
            continue;
        }
        progress.checked.fetch_add(1, Ordering::Relaxed);
        let Some(kind) = candidate.file_type() else {
            continue;
        };
        let kind = if kind.is_dir() {
            EntryKind::Folder
        } else if kind.is_file() {
            file_kind(candidate.path())
        } else {
            continue;
        };
        if !filter.includes(kind) {
            continue;
        }
        let name = candidate.file_name().to_string_lossy();
        let lowercase = name.to_lowercase();
        if query.iter().any(|token| !lowercase.contains(token)) {
            continue;
        }
        if entries.len() == MAX_SEARCH_RESULTS {
            truncated = true;
            break;
        }
        let metadata = candidate.metadata().ok();
        entries.push(Entry::new(candidate.into_path(), kind, metadata));
        progress.matches.fetch_add(1, Ordering::Release);
    }
    if progress.cancelled.load(Ordering::Relaxed) {
        return Ok((Listing::new(Vec::new(), sort, filter), false));
    }
    entries.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok((Listing::new(entries, sort, filter), truncated))
}

fn searchable_entry(entry: &DirEntry, show_hidden_folders: bool) -> bool {
    entry.depth() == 0
        || (!entry.path_is_symlink()
            && (entry.file_type().is_none_or(|kind| !kind.is_dir())
                || ((show_hidden_folders || !is_dot_named(entry.file_name()))
                    && entry.metadata().is_ok_and(|meta| {
                        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
                    }))))
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::windows_gpui::player_view::quick_open::{QuickOpenMode, listing::Category};

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
        let names = |mode, query: &str| {
            let progress = SearchProgress::default();
            let (results, truncated) = scan(
                &root.0,
                Sort::default(),
                Filter::new(mode),
                &search_tokens(query),
                &progress,
            )
            .unwrap();
            assert_eq!(progress.counts(), (5, results.len()));
            assert!(!truncated);
            results
                .range(0, results.len())
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            names(QuickOpenMode::Open, "FILM family"),
            [root.0.join("Nested").join("Family [Preview] Film.MP4")]
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
        let progress = SearchProgress::default();
        progress.cancelled.store(true, Ordering::Relaxed);
        let (results, truncated) = scan(
            &root.0,
            Sort::default(),
            Filter::new(QuickOpenMode::Open),
            &search_tokens("film"),
            &progress,
        )
        .unwrap();
        assert!(results.is_empty());
        assert!(!truncated);
        assert_eq!(progress.counts(), (0, 0));
    }

    #[test]
    fn skips_dot_folders_but_searches_dot_files_and_explicit_dot_roots() {
        let root = Fixture::new();
        let hidden = root.0.join(".Film Folder");
        fs::create_dir(&hidden).unwrap();
        let nested_media = hidden.join("Film Hidden.MP4");
        let dot_media = root.0.join(".Film Root.MP4");
        fs::write(&nested_media, []).unwrap();
        fs::write(&dot_media, []).unwrap();

        let matching_paths = |directory: &Path, show_hidden_folders| {
            let (results, _) = scan(
                directory,
                Sort::default(),
                Filter {
                    only_media: false,
                    show_dot_folders: show_hidden_folders,
                    ..Filter::new(QuickOpenMode::Open)
                },
                &search_tokens("film"),
                &SearchProgress::default(),
            )
            .unwrap();
            results
                .range(0, results.len())
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>()
        };
        let visible = matching_paths(&root.0, false);
        assert!(visible.contains(&dot_media));
        assert!(!visible.contains(&hidden));
        assert!(!visible.contains(&nested_media));
        let showing_hidden = matching_paths(&root.0, true);
        assert!(showing_hidden.contains(&hidden));
        assert!(showing_hidden.contains(&nested_media));
        assert_eq!(matching_paths(&hidden, false), [nested_media]);
    }

    #[test]
    fn search_uses_the_active_category_and_only_media_setting() {
        let root = Fixture::new();
        let audio = root.0.join("Family Notes.WAV");
        let text = root.0.join("Family Notes.TXT");
        fs::write(&audio, []).unwrap();
        fs::write(&text, []).unwrap();
        let matching_paths = |category, only_media| {
            let (results, _) = scan(
                &root.0,
                Sort::default(),
                Filter {
                    category,
                    only_media,
                    ..Filter::new(QuickOpenMode::Open)
                },
                &search_tokens("family notes"),
                &SearchProgress::default(),
            )
            .unwrap();
            results
                .range(0, results.len())
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            matching_paths(Category::Audio, false),
            std::slice::from_ref(&audio)
        );
        assert_eq!(
            matching_paths(Category::All, true),
            [root.0.join("Family Notes.MP4"), audio.clone()]
        );
        assert_eq!(
            matching_paths(Category::All, false),
            [root.0.join("Family Notes.MP4"), text, audio]
        );
    }
}
