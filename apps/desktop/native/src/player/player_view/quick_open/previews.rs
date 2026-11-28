use std::{
    collections::VecDeque,
    ffi::{CStr, CString, c_void},
    path::{Path, PathBuf},
    ptr::{self, NonNull},
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use gpui::RenderImage;
use image::{Frame, RgbaImage};
use libmpv2::Mpv;
use libmpv2_sys as sys;
#[cfg(windows)]
use std::path::{Component, Prefix};
use tokio::sync::oneshot;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::GetDriveTypeW;

const WIDTH: usize = 256;
const HEIGHT: usize = 144;
const STRIDE: usize = WIDTH * 4;
const BYTES: usize = STRIDE * HEIGHT;
const MAX_PENDING: usize = 24;
const HOVER_FRAME_COUNT: usize = 5;
const IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const FRAME_TIMEOUT: Duration = Duration::from_millis(1200);
const POLL: Duration = Duration::from_millis(12);
pub(super) const HOVER_INTERVAL: Duration = Duration::from_millis(180);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(72);

pub(super) type PreviewFrames = Vec<Arc<RenderImage>>;

#[derive(Clone)]
pub(super) struct PreviewWorker(Arc<PreviewSession>);

impl PreviewWorker {
    pub(super) fn new() -> Self {
        static WORKER: OnceLock<Option<Arc<Queue>>> = OnceLock::new();
        Self(Arc::new(PreviewSession {
            queue: WORKER.get_or_init(start_worker).clone(),
            closed: Arc::default(),
        }))
    }

    pub(super) fn close(&self) {
        self.0.close();
    }

    pub(super) fn request(
        &self,
        path: PathBuf,
        animated: bool,
    ) -> Option<oneshot::Receiver<PreviewFrames>> {
        if !supports_preview(&path) {
            return None;
        }
        let queue = self.0.queue.as_ref()?;
        let mut pending = queue.pending.lock().ok()?;
        if self.0.closed.load(Ordering::Relaxed) {
            return None;
        }
        pending.retain(|request| !request.is_cancelled());
        let (reply, receiver) = oneshot::channel();
        let request = Request {
            path,
            animated,
            reply,
            session_closed: self.0.closed.clone(),
        };
        if animated {
            if pending.len() == MAX_PENDING {
                pending.pop_back();
            }
            pending.push_front(request);
        } else {
            if pending.len() == MAX_PENDING {
                return None;
            }
            pending.push_back(request);
        }
        queue.ready.notify_one();
        Some(receiver)
    }
}

struct PreviewSession {
    queue: Option<Arc<Queue>>,
    closed: Arc<AtomicBool>,
}

impl PreviewSession {
    fn close(&self) {
        if !self.closed.swap(true, Ordering::Relaxed)
            && let Some(queue) = &self.queue
            && let Ok(mut pending) = queue.pending.lock()
        {
            pending.retain(|request| !request.is_cancelled());
            queue.ready.notify_one();
        }
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        self.close();
    }
}

pub(super) fn supports_preview(path: &Path) -> bool {
    #[cfg(windows)]
    let local = disk_letter(path).is_some();
    #[cfg(unix)]
    let local = path.is_absolute();
    local && is_video_file(path)
}

pub(super) fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "3g2"
                    | "3gp"
                    | "3gp2"
                    | "3gpp"
                    | "asf"
                    | "avi"
                    | "divx"
                    | "dv"
                    | "dvr-ms"
                    | "f4v"
                    | "flv"
                    | "h264"
                    | "h265"
                    | "hevc"
                    | "m1v"
                    | "m2t"
                    | "m2ts"
                    | "m2v"
                    | "m4v"
                    | "mk3d"
                    | "mkv"
                    | "mov"
                    | "mp4"
                    | "mp4v"
                    | "mpe"
                    | "mpeg"
                    | "mpg"
                    | "mpv"
                    | "mts"
                    | "mxf"
                    | "nsv"
                    | "nut"
                    | "ogm"
                    | "ogv"
                    | "qt"
                    | "rm"
                    | "rmvb"
                    | "roq"
                    | "tod"
                    | "trp"
                    | "ts"
                    | "vob"
                    | "webm"
                    | "wm"
                    | "wmv"
                    | "y4m"
            )
        })
}

#[cfg(windows)]
fn disk_letter(path: &Path) -> Option<u8> {
    if !path.is_absolute() {
        return None;
    }
    let Some(Component::Prefix(prefix)) = path.components().next() else {
        return None;
    };
    match prefix.kind() {
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => Some(letter),
        _ => None,
    }
}

#[cfg(windows)]
fn is_local_disk(letter: u8) -> bool {
    let root = [u16::from(letter), b':' as u16, b'\\' as u16, 0];
    // Removable or fixed disk. Mapped network drives still browse and play normally.
    matches!(unsafe { GetDriveTypeW(root.as_ptr()) }, 2 | 3)
}

struct Request {
    path: PathBuf,
    animated: bool,
    reply: oneshot::Sender<PreviewFrames>,
    session_closed: Arc<AtomicBool>,
}

impl Request {
    fn is_cancelled(&self) -> bool {
        self.session_closed.load(Ordering::Relaxed) || self.reply.is_closed()
    }
}

#[derive(Default)]
struct Queue {
    pending: Mutex<VecDeque<Request>>,
    ready: Condvar,
}

impl Queue {
    fn take(&self, decoder_session: Option<&AtomicBool>) -> Option<Request> {
        let mut pending = self.pending.lock().ok()?;
        loop {
            if decoder_session.is_some_and(|session| session.load(Ordering::Relaxed)) {
                return None;
            }
            if let Some(request) = pending.pop_front() {
                return Some(request);
            }
            if decoder_session.is_some() {
                let (next, deadline) = self.ready.wait_timeout(pending, IDLE_TIMEOUT).ok()?;
                pending = next;
                if deadline.timed_out() && pending.is_empty() {
                    return None;
                }
            } else {
                pending = self.ready.wait(pending).ok()?;
            }
        }
    }
}

fn start_worker() -> Option<Arc<Queue>> {
    let queue = Arc::new(Queue::default());
    let worker = Arc::clone(&queue);
    thread::Builder::new()
        .name("OpenPlayer thumbnails".into())
        .spawn(move || {
            let mut decoder: Option<(Decoder, Arc<AtomicBool>)> = None;
            loop {
                let session = decoder.as_ref().map(|(_, session)| session.as_ref());
                let Some(request) = worker.take(session) else {
                    decoder = None;
                    continue;
                };
                if request.is_cancelled() {
                    continue;
                }
                let mut frames = Vec::new();
                #[cfg(windows)]
                let can_decode = disk_letter(&request.path).is_some_and(is_local_disk);
                #[cfg(unix)]
                let can_decode = request.path.is_file();
                if can_decode {
                    if decoder.is_none() {
                        decoder =
                            Decoder::new().map(|decoder| (decoder, request.session_closed.clone()));
                    }
                    if let Some((decoder, session)) = decoder.as_mut() {
                        session.clone_from(&request.session_closed);
                        frames = decoder.preview(&request);
                    }
                }
                let _ = request.reply.send(frames);
            }
        })
        .ok()?;
    Some(queue)
}

// Field order releases the software render context before its owning mpv core.
struct Decoder {
    render: SoftwareRender,
    mpv: Mpv,
    pixels: Box<PixelBuffer>,
}

impl Decoder {
    fn new() -> Option<Self> {
        let mpv = Mpv::with_initializer(|init| {
            for (key, value) in [
                ("vo", "libmpv"),
                ("hwdec", "no"),
                ("aid", "no"),
                ("sid", "no"),
                ("audio-display", "no"),
                ("sub-auto", "no"),
                ("audio-file-auto", "no"),
                ("load-scripts", "no"),
                ("access-references", "no"),
                ("osd-level", "0"),
                ("cache", "no"),
                ("demuxer-max-bytes", "8MiB"),
                ("demuxer-max-back-bytes", "0"),
                ("demuxer-readahead-secs", "0.25"),
                ("vd-lavc-threads", "2"),
                ("vd-lavc-dr", "no"),
                ("sws-fast", "yes"),
                // mpv's zimg repacker can abort on unaligned chroma widths.
                ("sws-allow-zimg", "no"),
            ] {
                init.set_option(key, value)?;
            }
            Ok(())
        })
        .ok()?;
        let render = SoftwareRender::new(&mpv)?;
        Some(Self {
            render,
            mpv,
            pixels: Box::new(PixelBuffer([0; BYTES])),
        })
    }

    fn preview(&mut self, request: &Request) -> PreviewFrames {
        if request.is_cancelled() {
            return Vec::new();
        }
        let Some(path) = request
            .path
            .to_str()
            .and_then(|path| CString::new(path).ok())
        else {
            return Vec::new();
        };
        let desired = if request.animated {
            HOVER_FRAME_COUNT
        } else {
            1
        };
        let options = if request.animated {
            // A short silent sample runs at 2.5× in the decoder; the browser
            // displays frames at their original timing.
            c"start=12%,pause=no,speed=2.5"
        } else {
            c"start=12%,pause=yes"
        };
        let mut frames = self.load(&path, options, desired, request);
        // A short fade at the start should not leave a black cover for the entire file.
        if !request.animated
            && frames.first().is_some_and(|frame| near_black(frame))
            && !request.is_cancelled()
        {
            let alternative = self.load(&path, c"start=36%,pause=yes", 1, request);
            if alternative.first().is_some_and(|frame| !near_black(frame)) {
                frames = alternative;
            }
        }
        frames
    }

    fn load(
        &mut self,
        path: &CStr,
        options: &CStr,
        count: usize,
        request: &Request,
    ) -> PreviewFrames {
        while self.event().event_id != sys::mpv_event_id_MPV_EVENT_NONE {}
        let load = if crate::player::loadfile_has_playlist_index() {
            self.command(1, &[c"loadfile", path, c"replace", c"-1", options])
        } else {
            self.command(1, &[c"loadfile", path, c"replace", options])
        };
        if !load {
            return Vec::new();
        }
        let frames = self.collect(count, request);
        self.command(2, &[c"stop"]);
        frames
    }

    fn collect(&mut self, count: usize, request: &Request) -> PreviewFrames {
        let deadline = Instant::now() + FRAME_TIMEOUT + SAMPLE_INTERVAL * (count as u32 - 1);
        let mut frames = Vec::with_capacity(count);
        let mut next_frame = Instant::now();
        let mut started = false;
        let mut loaded = false;
        while !request.is_cancelled() && Instant::now() < deadline {
            loop {
                let event = self.event();
                match event.event_id {
                    sys::mpv_event_id_MPV_EVENT_NONE => break,
                    sys::mpv_event_id_MPV_EVENT_START_FILE => started = true,
                    sys::mpv_event_id_MPV_EVENT_FILE_LOADED if started => loaded = true,
                    sys::mpv_event_id_MPV_EVENT_END_FILE if started => return frames,
                    sys::mpv_event_id_MPV_EVENT_COMMAND_REPLY
                        if event.reply_userdata == 1 && event.error < 0 =>
                    {
                        return frames;
                    }
                    _ => {}
                }
            }
            if self.render.updated() {
                let due = loaded && self.render.frame_ready() && Instant::now() >= next_frame;
                if let Some(frame) = self.render.draw(&mut self.pixels, due) {
                    frames.push(frame);
                    if frames.len() == count {
                        break;
                    }
                    next_frame = Instant::now() + SAMPLE_INTERVAL;
                }
            }
            thread::sleep(POLL);
        }
        frames
    }

    fn command(&self, id: u64, args: &[&CStr]) -> bool {
        let mut args: Vec<_> = args
            .iter()
            .map(|arg| arg.as_ptr())
            .chain([ptr::null()])
            .collect();
        // The asynchronous API copies the arguments before returning and does not
        // block waiting for the renderer to acknowledge a pending frame.
        unsafe { sys::mpv_command_async(self.mpv.ctx.as_ptr(), id, args.as_mut_ptr()) >= 0 }
    }

    fn event(&self) -> sys::mpv_event {
        // A zero timeout makes polling safe on the renderer thread. The event
        // address remains valid only through the next call to mpv_wait_event.
        unsafe { *sys::mpv_wait_event(self.mpv.ctx.as_ptr(), 0.0) }
    }
}

fn near_black(frame: &RenderImage) -> bool {
    frame.as_bytes(0).is_some_and(|bytes| {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .step_by(97)
            .all(|pixel| pixel[..3].iter().all(|channel| *channel < 28))
    })
}

#[repr(align(64))]
struct PixelBuffer([u8; BYTES]);

struct SoftwareRender(NonNull<sys::mpv_render_context>);

impl SoftwareRender {
    fn new(mpv: &Mpv) -> Option<Self> {
        let mut context = ptr::null_mut();
        let mut params = [
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                c"sw".as_ptr().cast_mut().cast(),
            ),
            param(0, ptr::null_mut()),
        ];
        let status = unsafe {
            sys::mpv_render_context_create(&mut context, mpv.ctx.as_ptr(), params.as_mut_ptr())
        };
        if status < 0 {
            return None;
        }
        NonNull::new(context).map(Self)
    }

    fn updated(&self) -> bool {
        let update = unsafe { sys::mpv_render_context_update(self.0.as_ptr()) };
        update & u64::from(sys::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME) != 0
    }

    fn frame_ready(&self) -> bool {
        let mut info = sys::mpv_render_frame_info {
            flags: 0,
            target_time: 0,
        };
        let code = unsafe {
            sys::mpv_render_context_get_info(
                self.0.as_ptr(),
                param(
                    sys::mpv_render_param_type_MPV_RENDER_PARAM_NEXT_FRAME_INFO,
                    ptr::from_mut(&mut info).cast(),
                ),
            )
        };
        code >= 0
            && info.flags & u64::from(sys::mpv_render_frame_info_flag_MPV_RENDER_FRAME_INFO_PRESENT)
                != 0
    }

    fn draw(&self, pixels: &mut PixelBuffer, capture: bool) -> Option<Arc<RenderImage>> {
        let mut size = [WIDTH as i32, HEIGHT as i32];
        let mut stride = STRIDE;
        let mut timed = 0i32;
        let mut skip = i32::from(!capture);
        let mut params = [
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                size.as_mut_ptr().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                c"bgr0".as_ptr().cast_mut().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                ptr::from_mut(&mut stride).cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                pixels.0.as_mut_ptr().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                ptr::from_mut(&mut timed).cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SKIP_RENDERING,
                ptr::from_mut(&mut skip).cast(),
            ),
            param(0, ptr::null_mut()),
        ];
        // The aligned surface and stride remain valid for this synchronous render.
        let status =
            unsafe { sys::mpv_render_context_render(self.0.as_ptr(), params.as_mut_ptr()) };
        if status < 0 || !capture {
            return None;
        }
        let mut bgra = pixels.0.to_vec();
        // GPUI uploads BGRA directly to its Windows texture atlas. The BGR0
        // renderer leaves the fourth byte undefined, so make every pixel opaque.
        for alpha in bgra[3..].iter_mut().step_by(4) {
            *alpha = 255;
        }
        let buffer = RgbaImage::from_raw(WIDTH as u32, HEIGHT as u32, bgra)?;
        Some(Arc::new(RenderImage::new([Frame::new(buffer)])))
    }
}

impl Drop for SoftwareRender {
    fn drop(&mut self) {
        unsafe { sys::mpv_render_context_free(self.0.as_ptr()) };
    }
}

fn param(kind: sys::mpv_render_param_type, data: *mut c_void) -> sys::mpv_render_param {
    sys::mpv_render_param { type_: kind, data }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;

    #[test]
    fn renders_thumbnail_from_odd_sized_chroma_subsampled_video() {
        const SOURCE_WIDTH: usize = 97;
        const SOURCE_HEIGHT: usize = 55;
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "openplayer-preview-{}-{nonce}.y4m",
            std::process::id()
        ));
        let mut video =
            format!("YUV4MPEG2 W{SOURCE_WIDTH} H{SOURCE_HEIGHT} F24:1 Ip A1:1 C420jpeg\n")
                .into_bytes();
        let chroma_samples = SOURCE_WIDTH.div_ceil(2) * SOURCE_HEIGHT.div_ceil(2);
        for _ in 0..12 {
            video.extend_from_slice(b"FRAME\n");
            video.extend(std::iter::repeat_n(80, SOURCE_WIDTH * SOURCE_HEIGHT));
            video.extend(std::iter::repeat_n(200, chroma_samples));
            video.extend(std::iter::repeat_n(100, chroma_samples));
        }
        fs::write(&path, video).unwrap();
        let (reply, _receiver) = oneshot::channel();
        let request = Request {
            path,
            animated: false,
            reply,
            session_closed: Arc::default(),
        };
        let frames = Decoder::new().unwrap().preview(&request);
        let _ = fs::remove_file(request.path);
        let bytes = frames.first().unwrap().as_bytes(0).unwrap();
        assert_eq!(bytes.len(), BYTES);
        assert!(bytes.as_chunks::<4>().0.iter().any(|pixel| pixel[0] > 100));
    }
}
