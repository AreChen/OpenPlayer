mod address;
mod locations;
mod motion;
mod preview_state;
mod previews;
mod search;
mod view;

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use gpui::{
    AppContext, Context, Entity, KeyDownEvent, SharedString, Task, UniformListScrollHandle, Window,
};
use gpui_ce_motion::MotionState;
use serde_json::json;

use super::PlayerView;
use crate::media_paths::is_supported_media_path;

use address::AddressInput;
use locations::{QuickLocation, home_directory, system_locations};
use motion::ListingMotion;
use preview_state::QuickPreviews;
use search::QuickSearch;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum QuickOpenMode {
    Open,
    AppendFiles,
    AppendFolder,
}

pub(super) struct QuickOpen {
    mode: QuickOpenMode,
    directory: PathBuf,
    entries: DirectoryEntries,
    hidden_folders: Vec<Entry>,
    show_hidden_folders: bool,
    show_media_favorites: bool,
    context: Option<view::QuickEntryContext>,
    selected: BTreeSet<usize>,
    scroll: UniformListScrollHandle,
    history: Vec<PathBuf>,
    history_index: usize,
    address: Option<Entity<AddressInput>>,
    address_unavailable: bool,
    search: Option<QuickSearch>,
    search_motion: MotionState,
    listing_motion: ListingMotion,
    locations: Vec<(QuickLocation, PathBuf)>,
    drives: Vec<PathBuf>,
    previews: QuickPreviews,
    navigation: Option<Task<()>>,
    _locations_task: Task<()>,
}

impl QuickOpen {
    fn can_go_back(&self) -> bool {
        self.show_media_favorites || self.history_index > 0
    }

    fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    fn visible_entries(&self) -> &DirectoryEntries {
        self.search
            .as_ref()
            .and_then(|search| search.results.as_ref())
            .unwrap_or(&self.entries)
    }

    fn showing_search_results(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.results.is_some())
    }

    fn reset_listing(&mut self, cx: &mut Context<PlayerView>) {
        self.context = None;
        self.previews.clear_visible(cx);
        self.selected.clear();
        self.scroll = UniformListScrollHandle::new();
    }

    fn restart_listing_motion(&mut self) {
        self.listing_motion.restart();
    }

    fn apply_folder_visibility(&mut self) {
        let DirectoryEntries::Ready(entries) = &mut self.entries else {
            return;
        };
        if self.show_hidden_folders {
            if !self.hidden_folders.is_empty() {
                entries.append(&mut self.hidden_folders);
                sort_entries(entries);
            }
        } else {
            self.hidden_folders.extend(entries.extract_if(.., |entry| {
                entry.is_directory && entry.path.file_name().is_some_and(is_dot_named)
            }));
        }
    }
}

enum DirectoryEntries {
    Loading,
    Ready(Vec<Entry>),
    Unreadable,
}

struct Entry {
    path: PathBuf,
    name: SharedString,
    is_directory: bool,
}

#[derive(Clone, Copy)]
enum Navigation {
    Initial,
    NewLocation,
    History(usize),
    Address,
}

impl PlayerView {
    pub(super) fn show_quick_open(&mut self, mode: QuickOpenMode, cx: &mut Context<Self>) {
        if self.quick_open.is_some() || self.picker_open {
            return;
        }
        let directory = self
            .controller
            .media_browser_directory()
            .or_else(home_directory)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        let locations = home_directory()
            .map(|path| vec![(QuickLocation::Home, path)])
            .unwrap_or_default();
        let work = cx.background_executor().spawn(async { system_locations() });
        let locations_task = cx.spawn(async move |view, cx| {
            let (locations, drives) = work.await;
            let _ = view.update(cx, |view, cx| {
                if let Some(picker) = &mut view.quick_open {
                    picker.locations = locations;
                    picker.drives = drives;
                    cx.notify();
                }
            });
        });
        self.quick_open = Some(QuickOpen {
            mode,
            directory: directory.clone(),
            entries: DirectoryEntries::Loading,
            hidden_folders: Vec::new(),
            show_hidden_folders: false,
            show_media_favorites: false,
            context: None,
            selected: BTreeSet::new(),
            scroll: UniformListScrollHandle::new(),
            history: vec![directory.clone()],
            history_index: 0,
            address: None,
            address_unavailable: false,
            search: None,
            search_motion: MotionState::default(),
            listing_motion: ListingMotion::default(),
            locations,
            drives: Vec::new(),
            previews: QuickPreviews::default(),
            navigation: None,
            _locations_task: locations_task,
        });
        self.drag_origin = None;
        self.last_stage_click = None;
        self.navigate_quick_open(directory, Navigation::Initial, cx);
    }

    pub(super) fn quick_open_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.is_held {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if key == "escape"
            && let Some(picker) = &mut self.quick_open
            && picker.context.take().is_some()
        {
            cx.notify();
            return;
        }
        if modifiers.control && key == "l" {
            self.focus_quick_address(window, cx);
            return;
        }
        if modifiers.control && key == "f" {
            self.focus_quick_search(window, cx);
            return;
        }
        // On Windows GPUI folds Shift + period into ">" on US keyboards.
        if modifiers.control
            && !modifiers.alt
            && !modifiers.platform
            && (key == ">" || (modifiers.shift && key == "."))
        {
            self.toggle_quick_hidden_folders(cx);
            return;
        }
        if self
            .quick_open
            .as_ref()
            .and_then(|picker| picker.address.as_ref())
            .is_some_and(|address| address.read(cx).is_focused(window))
        {
            match key {
                "escape" => {
                    if let Some(picker) = &mut self.quick_open {
                        picker.address_unavailable = false;
                        picker.navigation = None;
                    }
                    self.exit_quick_address();
                    cx.notify();
                }
                "enter" => self.submit_quick_address(cx),
                _ => {}
            }
            return;
        }
        if let Some(search) = self
            .quick_open
            .as_ref()
            .and_then(|picker| picker.search.as_ref())
        {
            if key == "escape" {
                self.close_quick_search(cx);
                return;
            }
            if search.input.read(cx).is_focused(window) {
                if key == "enter" {
                    self.restore_quick_focus = true;
                    cx.notify();
                }
                return;
            }
        }
        match key {
            "escape" => self.close_quick_open(cx),
            "enter" => self.open_quick_selection(cx),
            "left" if modifiers.alt => self.quick_open_back(cx),
            "right" if modifiers.alt => self.quick_open_forward(cx),
            "backspace" | "up" if key == "backspace" || modifiers.alt => {
                if let Some(parent) = self
                    .quick_open
                    .as_ref()
                    .and_then(|picker| picker.directory.parent())
                {
                    self.quick_open_directory(parent.to_path_buf(), cx);
                }
            }
            "f5" => {
                if let Some(picker) = &self.quick_open {
                    self.quick_open_directory(picker.directory.clone(), cx);
                }
            }
            _ => {}
        }
    }

    fn quick_open_back(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &self.quick_open
            && picker.show_media_favorites
        {
            self.quick_open_directory(picker.directory.clone(), cx);
            return;
        }
        if let Some(picker) = &self.quick_open
            && picker.can_go_back()
        {
            let index = picker.history_index - 1;
            self.navigate_quick_open(
                picker.history[index].clone(),
                Navigation::History(index),
                cx,
            );
        }
    }

    fn quick_open_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &self.quick_open
            && picker.can_go_forward()
        {
            let index = picker.history_index + 1;
            self.navigate_quick_open(
                picker.history[index].clone(),
                Navigation::History(index),
                cx,
            );
        }
    }

    fn quick_locations(&self) -> &[(QuickLocation, PathBuf)] {
        self.quick_open
            .as_ref()
            .map_or(&[], |picker| &picker.locations)
    }

    fn quick_drives(&self) -> &[PathBuf] {
        self.quick_open
            .as_ref()
            .map_or(&[], |picker| &picker.drives)
    }

    fn quick_favorites(&self) -> &[PathBuf] {
        self.controller.favorite_directories()
    }

    fn toggle_quick_favorite(&mut self, directory: PathBuf, cx: &mut Context<Self>) {
        self.controller.toggle_favorite_directory(directory);
        self.schedule_samples(cx);
        cx.notify();
    }

    fn show_quick_media_favorites(&mut self, cx: &mut Context<Self>) {
        if self
            .quick_open
            .as_ref()
            .is_none_or(|picker| picker.mode == QuickOpenMode::AppendFolder)
        {
            return;
        }
        self.exit_quick_address();
        let entries = self
            .controller
            .favorite_media()
            .iter()
            .map(|path| Entry {
                path: path.clone(),
                name: path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned()
                    .into(),
                is_directory: false,
            })
            .collect();
        if let Some(picker) = &mut self.quick_open {
            self.restore_quick_focus |= picker.search.take().is_some();
            picker.navigation = None;
            picker.address_unavailable = false;
            picker.reset_listing(cx);
            picker.entries = DirectoryEntries::Ready(entries);
            picker.hidden_folders.clear();
            if !picker.show_media_favorites {
                picker.restart_listing_motion();
            }
            picker.show_media_favorites = true;
            cx.notify();
        }
    }

    fn quick_media_is_favorite(&self, path: &Path) -> bool {
        self.controller.is_media_favorite(path)
    }

    fn toggle_quick_hidden_folders(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        if picker.show_media_favorites {
            return;
        }
        picker.show_hidden_folders = !picker.show_hidden_folders;
        picker.apply_folder_visibility();
        if picker.showing_search_results() {
            self.reload_quick_search(cx);
        } else {
            picker.reset_listing(cx);
            picker.restart_listing_motion();
        }
        cx.notify();
    }

    fn toggle_quick_media_favorite(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.controller.toggle_favorite_media(path);
        if self
            .quick_open
            .as_ref()
            .is_some_and(|picker| picker.show_media_favorites)
        {
            self.show_quick_media_favorites(cx);
        }
        self.schedule_samples(cx);
        cx.notify();
    }

    fn reveal_quick_entry(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Some(path) = path.to_str() {
            self.send("revealBrowserFile", json!(path), cx);
        }
    }

    fn play_quick_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.accept_quick_paths(QuickOpenMode::Open, vec![path], cx);
    }

    fn focus_quick_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        let address = picker.address.get_or_insert_with(|| {
            cx.new(|cx| AddressInput::new(picker.directory.to_string_lossy().into_owned(), cx))
        });
        address.update(cx, |input, cx| input.focus(window, cx));
        self.restore_quick_focus = false;
        cx.notify();
    }

    pub(super) fn exit_quick_address(&mut self) {
        if let Some(picker) = &mut self.quick_open
            && picker.address.take().is_some()
        {
            self.restore_quick_focus = true;
        }
    }

    fn submit_quick_address(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &self.quick_open else {
            return;
        };
        let Some(address) = &picker.address else {
            return;
        };
        let text = address.read(cx).text().trim();
        let Some(path) = address_path(text, &picker.directory) else {
            if let Some(picker) = &mut self.quick_open {
                picker.address_unavailable = true;
                picker.navigation = None;
                cx.notify();
            }
            return;
        };
        self.navigate_quick_open(path, Navigation::Address, cx);
    }

    fn quick_open_directory(&mut self, directory: PathBuf, cx: &mut Context<Self>) {
        self.exit_quick_address();
        self.navigate_quick_open(directory, Navigation::NewLocation, cx);
    }

    fn navigate_quick_open(
        &mut self,
        directory: PathBuf,
        navigation: Navigation,
        cx: &mut Context<Self>,
    ) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        if picker.search.take().is_some() {
            picker.reset_listing(cx);
            self.restore_quick_focus |= picker.address.is_none();
        }
        picker.address_unavailable = false;
        picker.context = None;
        let mode = picker.mode;
        let path = directory.clone();
        let work = cx.background_executor().spawn(async move {
            let path = std::path::absolute(&path)?;
            if matches!(navigation, Navigation::Address)
                && mode != QuickOpenMode::AppendFolder
                && is_supported_media_path(&path)
                && fs::metadata(&path).is_ok_and(|metadata| metadata.is_file())
            {
                return Ok((path, None));
            }
            scan_directory(&path, mode).map(|entries| (path, Some(entries)))
        });
        picker.navigation = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                if let Some(task) = view
                    .quick_open
                    .as_mut()
                    .and_then(|picker| picker.navigation.take())
                {
                    task.detach();
                }
                match result {
                    Ok((path, Some(entries))) => {
                        view.exit_quick_address();
                        view.controller.remember_media_directory(&path);
                        view.schedule_samples(cx);
                        if let Some(picker) = &mut view.quick_open {
                            picker.reset_listing(cx);
                            picker.entries = DirectoryEntries::Ready(entries);
                            picker.hidden_folders.clear();
                            picker.apply_folder_visibility();
                            picker.restart_listing_motion();
                            picker.show_media_favorites = false;
                            match navigation {
                                Navigation::Initial => picker.history[0] = path.clone(),
                                Navigation::History(index) => picker.history_index = index,
                                _ if picker.directory != path => {
                                    picker.history.truncate(picker.history_index + 1);
                                    picker.history.push(path.clone());
                                    picker.history_index = picker.history.len() - 1;
                                }
                                _ => {}
                            }
                            picker.directory = path;
                        }
                    }
                    Ok((path, None)) => {
                        if let Some(parent) = path.parent() {
                            view.controller.remember_media_directory(parent);
                        }
                        view.accept_quick_paths(mode, vec![path], cx);
                    }
                    Err(_) => {
                        if matches!(navigation, Navigation::Initial)
                            && let Some(home) = home_directory().filter(|home| home != &directory)
                        {
                            view.navigate_quick_open(home, Navigation::Initial, cx);
                            return;
                        }
                        if let Some(picker) = &mut view.quick_open {
                            if matches!(picker.entries, DirectoryEntries::Loading) {
                                picker.entries = DirectoryEntries::Unreadable;
                            } else {
                                picker.address_unavailable = true;
                            }
                        }
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn click_quick_entry(&mut self, index: usize, double_click: bool, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        let DirectoryEntries::Ready(entries) = picker.visible_entries() else {
            return;
        };
        let Some(entry) = entries.get(index) else {
            return;
        };
        if entry.is_directory {
            let path = entry.path.clone();
            self.quick_open_directory(path, cx);
        } else if double_click {
            let (mode, path) = (picker.mode, entry.path.clone());
            self.accept_quick_paths(mode, vec![path], cx);
        } else {
            if !picker.selected.remove(&index) {
                picker.selected.insert(index);
            }
            self.restore_quick_focus = true;
            cx.notify();
        }
    }

    fn open_quick_selection(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &self.quick_open else {
            return;
        };
        if picker.navigation.is_some() {
            return;
        }
        let DirectoryEntries::Ready(entries) = picker.visible_entries() else {
            return;
        };
        let paths = if picker.mode == QuickOpenMode::AppendFolder {
            vec![picker.directory.clone()]
        } else {
            picker
                .selected
                .iter()
                .filter_map(|&index| entries.get(index))
                .map(|entry| entry.path.clone())
                .collect()
        };
        if !paths.is_empty() {
            self.accept_quick_paths(picker.mode, paths, cx);
        }
    }

    pub(super) fn accept_quick_drop(&mut self, paths: &[PathBuf], cx: &mut Context<Self>) -> bool {
        let Some(picker) = &self.quick_open else {
            return false;
        };
        self.accept_quick_paths(picker.mode, paths.to_vec(), cx);
        true
    }

    fn accept_quick_paths(
        &mut self,
        mode: QuickOpenMode,
        paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.close_quick_open(cx);
        let action = if mode == QuickOpenMode::Open {
            "paths"
        } else {
            "appendPaths"
        };
        let paths = paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>();
        self.send(action, json!(paths), cx);
    }

    pub(super) fn close_quick_open(&mut self, cx: &mut Context<Self>) {
        self.exit_quick_address();
        if let Some(mut picker) = self.quick_open.take() {
            self.restore_quick_focus |= picker.search.take().is_some();
            picker.previews.release(cx);
        }
        self.last_stage_click = None;
        self.drag_origin = None;
        self.activity(cx);
        cx.notify();
    }
}

fn address_path(text: &str, directory: &Path) -> Option<PathBuf> {
    let text = text.trim_matches('"');
    if text.is_empty() {
        return None;
    }
    if text == "~" {
        return home_directory();
    }
    if let Some(relative) = text.strip_prefix(r"~\").or_else(|| text.strip_prefix("~/")) {
        return home_directory().map(|home| home.join(relative));
    }
    Some(directory.join(text))
}

fn scan_directory(directory: &Path, mode: QuickOpenMode) -> io::Result<Vec<Entry>> {
    let mut entries = fs::read_dir(directory)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;
            let path = entry.path();
            let is_directory = file_type.is_dir() || (file_type.is_symlink() && path.is_dir());
            (is_directory
                || (mode != QuickOpenMode::AppendFolder && is_supported_media_path(&path)))
            .then(|| Entry {
                path,
                name: entry.file_name().to_string_lossy().into_owned().into(),
                is_directory,
            })
        })
        .collect::<Vec<_>>();
    sort_entries(&mut entries);
    Ok(entries)
}

fn is_dot_named(name: &OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by_cached_key(|entry| (!entry.is_directory, entry.name.to_lowercase()));
}
