use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use gpui::{App, Context, RenderImage, Task};

use super::{
    DirectoryEntries, PlayerView,
    previews::{HOVER_INTERVAL, PreviewFrames, PreviewWorker},
};

const MAX_COVERS: usize = 64;
const HOVER_DELAY: Duration = Duration::from_millis(360);

#[derive(Default)]
pub(super) struct QuickPreviews {
    worker: Option<PreviewWorker>,
    covers: VecDeque<(PathBuf, Option<Arc<RenderImage>>)>,
    visible: Vec<PathBuf>,
    cover_job: Option<(PathBuf, Task<()>)>,
    hover: Option<Hover>,
}

struct Hover {
    path: PathBuf,
    frames: PreviewFrames,
    frame: usize,
    _job: Task<()>,
}

impl QuickPreviews {
    pub(super) fn clear_visible(&mut self, cx: &mut App) {
        self.visible.clear();
        self.cover_job = None;
        self.clear_hover(cx);
    }

    fn clear_hover(&mut self, cx: &mut App) {
        if let Some(hover) = self.hover.take() {
            release_images(hover.frames, cx);
        }
    }

    pub(super) fn release(&mut self, cx: &mut App) {
        if let Some(worker) = self.worker.take() {
            worker.close();
        }
        self.clear_visible(cx);
        release_images(
            self.covers
                .drain(..)
                .filter_map(|(_, image)| image)
                .collect(),
            cx,
        );
    }
}

impl PlayerView {
    pub(super) fn quick_thumbnail(&self, path: &Path) -> Option<Arc<RenderImage>> {
        let previews = &self.quick_open.as_ref()?.previews;
        if let Some(hover) = &previews.hover
            && hover.path == path
            && let Some(image) = hover.frames.get(hover.frame)
        {
            return Some(image.clone());
        }
        previews
            .covers
            .iter()
            .find_map(|(cached, image)| (cached == path).then(|| image.clone()).flatten())
    }

    pub(super) fn queue_quick_thumbnails(
        &mut self,
        mut paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        if picker.showing_search_results() {
            if let DirectoryEntries::Ready(entries) = picker.visible_entries() {
                paths.retain(|path| {
                    entries
                        .range(0, entries.len())
                        .any(|entry| &entry.path == path)
                });
            } else {
                paths.clear();
            }
        } else if !picker.show_media_favorites {
            paths.retain(|path| path.parent() == Some(picker.directory.as_path()));
        }
        let previews = &mut picker.previews;
        if let Some(hover) = &previews.hover
            && !paths.contains(&hover.path)
        {
            previews.clear_hover(cx);
        }
        if let Some((path, _)) = &previews.cover_job
            && !paths.contains(path)
        {
            previews.cover_job = None;
        }
        previews.visible = paths;
        self.request_next_cover(cx);
    }

    fn request_next_cover(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        let previews = &mut picker.previews;
        if previews.cover_job.is_some() {
            return;
        }
        let Some(path) = previews
            .visible
            .iter()
            .find(|path| !previews.covers.iter().any(|(cached, _)| cached == *path))
            .cloned()
        else {
            return;
        };
        let worker = previews.worker.get_or_insert_with(PreviewWorker::new);
        let reply = worker.request(path.clone(), false);
        let request_path = path.clone();
        let task = cx.spawn(async move |view, cx| {
            let image = match reply {
                Some(reply) => reply
                    .await
                    .ok()
                    .and_then(|frames| frames.into_iter().next()),
                None => None,
            };
            let _ = view.update(cx, |view, cx| {
                let Some(picker) = &mut view.quick_open else {
                    return;
                };
                let previews = &mut picker.previews;
                if previews
                    .cover_job
                    .as_ref()
                    .is_none_or(|(path, _)| *path != request_path)
                {
                    return;
                }
                if let Some((_, task)) = previews.cover_job.take() {
                    task.detach();
                }
                if previews.covers.len() == MAX_COVERS
                    && let Some((_, Some(old))) = previews.covers.pop_front()
                {
                    release_images(vec![old], cx);
                }
                let loaded = image.is_some();
                previews.covers.push_back((request_path, image));
                if loaded {
                    cx.notify();
                }
                view.request_next_cover(cx);
            });
        });
        previews.cover_job = Some((path, task));
    }

    pub(super) fn quick_preview_hover(
        &mut self,
        path: &Path,
        entered: bool,
        cx: &mut Context<Self>,
    ) {
        if self.reduced_motion || cx.reduce_motion() {
            return;
        }
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        let previews = &mut picker.previews;
        if !entered {
            if previews
                .hover
                .as_ref()
                .is_some_and(|hover| hover.path == path)
            {
                previews.clear_hover(cx);
                cx.notify();
            }
            return;
        }
        if previews
            .hover
            .as_ref()
            .is_some_and(|hover| hover.path == path)
        {
            return;
        }
        previews.clear_hover(cx);
        let worker = previews
            .worker
            .get_or_insert_with(PreviewWorker::new)
            .clone();
        let path = path.to_path_buf();
        let request_path = path.clone();
        let timer = cx.background_executor().clone();
        let job = cx.spawn(async move |view, cx| {
            timer.timer(HOVER_DELAY).await;
            let Some(reply) = worker.request(request_path.clone(), true) else {
                return;
            };
            let Ok(frames) = reply.await else {
                return;
            };
            if frames.len() < 2 {
                return;
            }
            let count = frames.len();
            let attached = view.update(cx, |view, cx| {
                if let Some(hover) = view
                    .quick_open
                    .as_mut()
                    .and_then(|picker| picker.previews.hover.as_mut())
                    && hover.path == request_path
                {
                    hover.frames = frames;
                    cx.notify();
                    return true;
                }
                false
            });
            if !attached.unwrap_or(false) {
                return;
            }
            let mut frame = 0;
            loop {
                timer.timer(HOVER_INTERVAL).await;
                frame = (frame + 1) % count;
                let active = view.update(cx, |view, cx| {
                    if let Some(hover) = view
                        .quick_open
                        .as_mut()
                        .and_then(|picker| picker.previews.hover.as_mut())
                        && hover.path == request_path
                    {
                        hover.frame = frame;
                        cx.notify();
                        return true;
                    }
                    false
                });
                if !active.unwrap_or(false) {
                    break;
                }
            }
        });
        previews.hover = Some(Hover {
            path,
            frames: Vec::new(),
            frame: 0,
            _job: job,
        });
        cx.notify();
    }
}

fn release_images(images: PreviewFrames, cx: &mut App) {
    if !images.is_empty() {
        cx.defer(move |cx| {
            for image in images {
                cx.drop_image(image, None);
            }
        });
    }
}
