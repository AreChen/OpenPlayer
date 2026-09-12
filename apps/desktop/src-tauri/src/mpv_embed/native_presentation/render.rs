use libmpv2::{
    Format, Mpv,
    events::{Event, PropertyData},
};
use libmpv2_sys as sys;
use openplayer_native_sdk::presentation::{
    self as transport, FrameMeta,
    windows::{Control, Producer, SendOutcome, qpc_frequency, qpc_now},
};
use std::{
    ffi::c_void,
    ptr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND, RECT},
    System::Threading::{CreateEventW, SetEvent, WaitForSingleObject},
    UI::WindowsAndMessaging::GetClientRect,
};

pub(super) struct Shared {
    pub control: Control,
    pub active: AtomicBool,
    pub paused: AtomicBool,
    pub stop: AtomicBool,
    duration_ns: AtomicU64,
    pub sent: AtomicU64,
    pub dropped: AtomicU64,
    #[cfg(feature = "window-smoke")]
    pub last_hash: AtomicU64,
    pub error: Mutex<Option<String>>,
}
impl Shared {
    pub fn invalidate(&self) {
        self.control.advance_epoch();
    }
    fn fail(&self, error: String) {
        if let Ok(mut state) = self.error.lock() {
            *state = Some(error);
        }
        self.active.store(false, Ordering::Release);
        self.control.close();
    }
}

struct Wake(HANDLE);
unsafe impl Send for Wake {}
unsafe impl Sync for Wake {}
impl Drop for Wake {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
impl Wake {
    fn signal(&self) {
        unsafe {
            SetEvent(self.0);
        }
    }
}

pub(super) struct Worker {
    pub shared: Arc<Shared>,
    wake: Arc<Wake>,
    render: Option<thread::JoinHandle<()>>,
    observer: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn start(
        mpv: &Mpv,
        producer: Producer,
        width: u32,
        height: u32,
        fps: f64,
        window: isize,
    ) -> Result<Self, String> {
        let render_client = mpv.create_client(None).map_err(|e| e.to_string())?;
        let observer_client = mpv.create_client(None).map_err(|e| e.to_string())?;
        let event = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
        if event.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let wake = Arc::new(Wake(event));
        let shared = Arc::new(Shared {
            control: producer.control(),
            active: AtomicBool::new(false),
            paused: AtomicBool::new(mpv.get_property("pause").unwrap_or(true)),
            stop: AtomicBool::new(false),
            duration_ns: AtomicU64::new((1e9 / fps) as u64),
            sent: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            #[cfg(feature = "window-smoke")]
            last_hash: AtomicU64::new(0),
            error: Mutex::new(None),
        });
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let state = shared.clone();
        let signal = wake.clone();
        let render = thread::Builder::new()
            .name("native-present-source".into())
            .spawn(move || {
                let result = render_loop(
                    render_client,
                    producer,
                    (width, height),
                    window,
                    &state,
                    &signal,
                    tx,
                );
                if let Err(error) = result {
                    state.fail(error);
                }
            })
            .map_err(|e| e.to_string())?;
        let mut worker = Self {
            shared,
            wake,
            render: Some(render),
            observer: None,
        };
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|e| e.to_string())??;
        let state = worker.shared.clone();
        worker.observer = Some(
            thread::Builder::new()
                .name("native-present-events".into())
                .spawn(move || {
                    if let Err(error) = observe(observer_client, &state, fps) {
                        state.fail(error);
                    }
                })
                .map_err(|e| e.to_string())?,
        );
        Ok(worker)
    }
    pub fn stop(&mut self) -> Result<(), String> {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.active.store(false, Ordering::Release);
        self.shared.control.close();
        self.wake.signal();
        let deadline = Instant::now() + Duration::from_secs(3);
        for slot in [&mut self.render, &mut self.observer] {
            while slot.as_ref().is_some_and(|thread| !thread.is_finished())
                && Instant::now() < deadline
            {
                thread::sleep(Duration::from_millis(5));
            }
            if slot.as_ref().is_some_and(|thread| !thread.is_finished()) {
                return Err("native presentation worker did not stop".into());
            }
            if let Some(thread) = slot.take() {
                thread
                    .join()
                    .map_err(|_| "native presentation worker panicked")?;
            }
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("OpenPlayer {error}");
        }
        // Each thread owns its mpv client, keeping the core alive even on a timeout.
    }
}

struct Context(*mut sys::mpv_render_context);
#[repr(C, align(64))]
#[derive(Clone)]
struct PixelBlock([u8; 64]);
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            sys::mpv_render_context_set_update_callback(self.0, None, ptr::null_mut());
            sys::mpv_render_context_free(self.0);
        }
    }
}
unsafe extern "C" fn updated(data: *mut c_void) {
    unsafe {
        SetEvent(data);
    }
}
fn param(type_: sys::mpv_render_param_type, data: *mut c_void) -> sys::mpv_render_param {
    sys::mpv_render_param { type_, data }
}
fn checked(result: i32, operation: &str) -> Result<(), String> {
    if result < 0 {
        Err(format!("{operation}: mpv {result}"))
    } else {
        Ok(())
    }
}

fn render_loop(
    mpv: Mpv,
    mut producer: Producer,
    limits: (u32, u32),
    window: isize,
    state: &Shared,
    wake: &Wake,
    ready: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let mut advanced: i32 = 1;
    let mut params = [
        param(
            sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            c"sw".as_ptr().cast_mut().cast(),
        ),
        param(
            sys::mpv_render_param_type_MPV_RENDER_PARAM_ADVANCED_CONTROL,
            (&mut advanced as *mut i32).cast(),
        ),
        param(0, ptr::null_mut()),
    ];
    let mut raw = ptr::null_mut();
    let result = checked(
        unsafe { sys::mpv_render_context_create(&mut raw, mpv.ctx.as_ptr(), params.as_mut_ptr()) },
        "create native presentation source",
    );
    if let Err(error) = result {
        let _ = ready.send(Err(error.clone()));
        return Err(error);
    }
    let context = Context(raw);
    unsafe {
        sys::mpv_render_context_set_update_callback(context.0, Some(updated), wake.0);
    }
    let _ = ready.send(Ok(()));
    let frequency = qpc_frequency().map_err(|e| e.to_string())?;
    let capacity = limits.0 as usize * limits.1 as usize * 4;
    let mut storage = vec![PixelBlock([0; 64]); capacity.div_ceil(64)];
    // Fully initialized byte blocks with explicit SIMD alignment; the vector
    // does not move/reallocate while libmpv borrows its output pointer.
    let pixels =
        unsafe { std::slice::from_raw_parts_mut(storage.as_mut_ptr().cast::<u8>(), capacity) };
    let mut dimensions = viewport_size(window, limits.0, limits.1)?;
    let mut pending = dimensions;
    let mut changed = Instant::now();
    let mut sequence = 0;
    let mut previous_epoch = 0;
    let mut redraw_pending = false;
    while !state.stop.load(Ordering::Acquire) {
        unsafe {
            WaitForSingleObject(wake.0, 50);
        }
        let updated = unsafe { sys::mpv_render_context_update(context.0) }
            & sys::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64
            != 0;
        let viewport = viewport_size(window, limits.0, limits.1)?;
        if viewport != pending {
            pending = viewport;
            changed = Instant::now();
        }
        // Keep native dragging responsive. Reallocate input/swapchain only after
        // a short stable interval, not on every WM_SIZE message while dragging.
        let resized = dimensions != pending
            && changed.elapsed() >= Duration::from_millis(150)
            && state.active.load(Ordering::Acquire);
        if resized {
            dimensions = pending;
            state.invalidate();
        }
        let paused = state.paused.load(Ordering::Acquire);
        let retry_redraw = state.active.load(Ordering::Acquire)
            && paused
            && (redraw_pending || previous_epoch != state.control.epoch());
        if !updated && !resized && !retry_redraw {
            continue;
        }
        let epoch = state.control.epoch();
        let mut info: sys::mpv_render_frame_info = unsafe { std::mem::zeroed() };
        if updated {
            checked(
                unsafe {
                    sys::mpv_render_context_get_info(
                        context.0,
                        param(
                            sys::mpv_render_param_type_MPV_RENDER_PARAM_NEXT_FRAME_INFO,
                            (&mut info as *mut sys::mpv_render_frame_info).cast(),
                        ),
                    )
                },
                "read native frame info",
            )?;
        }
        // Pinned Windows mpv 2bd9c3229f uses nanoseconds here despite render.h's
        // stale microsecond comment. Convert in-process mpv time to shared QPC.
        let mpv_now = unsafe { sys::mpv_get_time_ns(mpv.ctx.as_ptr()) };
        let qpc = qpc_now().map_err(|e| e.to_string())?;
        let mut size = [dimensions.0 as i32, dimensions.1 as i32];
        let mut stride = dimensions.0 as usize * 4;
        let mut timed: i32 = 1;
        let mut output = [
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                size.as_mut_ptr().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                c"rgb0".as_ptr().cast_mut().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                (&mut stride as *mut usize).cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                pixels.as_mut_ptr().cast(),
            ),
            param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                (&mut timed as *mut i32).cast(),
            ),
            param(0, ptr::null_mut()),
        ];
        checked(
            unsafe { sys::mpv_render_context_render(context.0, output.as_mut_ptr()) },
            "render native frame",
        )?;
        let present_requested =
            info.flags & sys::mpv_render_frame_info_flag_MPV_RENDER_FRAME_INFO_PRESENT as u64 != 0;
        if !present_requested && !resized && !retry_redraw {
            continue;
        }
        // Always acknowledge a requested present, including old epochs and busy
        // consumers. The external GPU never controls mpv's audio/media clock.
        let result = (|| {
            if !state.active.load(Ordering::Acquire) || !state.control.is_current(epoch) {
                return Ok(());
            }
            sequence += 1;
            let mut flags = 0;
            if resized
                || info.flags & sys::mpv_render_frame_info_flag_MPV_RENDER_FRAME_INFO_REDRAW as u64
                    != 0
                || state.paused.load(Ordering::Acquire)
            {
                flags |= transport::REDRAW;
            }
            if info.flags & sys::mpv_render_frame_info_flag_MPV_RENDER_FRAME_INFO_REPEAT as u64 != 0
            {
                flags |= transport::REPEAT;
            }
            if previous_epoch != epoch {
                flags |= transport::RESET;
            }
            let (width, height) = dimensions;
            let frame_pixels = &mut pixels[..width as usize * height as usize * 4];
            for alpha in frame_pixels.iter_mut().skip(3).step_by(4) {
                *alpha = 255;
            }
            let target =
                if flags & (transport::REDRAW | transport::REPEAT) != 0 || info.target_time <= 0 {
                    qpc_now().map_err(|e| e.to_string())?
                } else {
                    to_qpc(info.target_time, mpv_now, qpc, frequency)?
                };
            let meta = FrameMeta {
                sequence,
                epoch,
                target_qpc: target,
                duration_ns: state.duration_ns.load(Ordering::Acquire),
                width,
                height,
                stride: width * 4,
                flags,
            };
            match producer
                .try_send(meta, frame_pixels)
                .map_err(|e| e.to_string())?
            {
                SendOutcome::Sent => {
                    redraw_pending = false;
                    #[cfg(feature = "window-smoke")]
                    state.last_hash.store(
                        frame_pixels
                            .iter()
                            .step_by(257)
                            .fold(14695981039346656037u64, |hash, byte| {
                                (hash ^ *byte as u64).wrapping_mul(1099511628211)
                            }),
                        Ordering::Release,
                    );
                    state.sent.fetch_add(1, Ordering::Relaxed);
                    previous_epoch = epoch;
                }
                _ => {
                    // A paused frame has no future playback frame to replace it.
                    // Retry from mpv's retained frame without queueing old pixels.
                    redraw_pending = state.paused.load(Ordering::Acquire);
                    state.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
            Ok::<(), String>(())
        })();
        if present_requested {
            unsafe {
                sys::mpv_render_context_report_swap(context.0);
            }
        }
        if let Err(error) = result {
            state.fail(error);
        }
    }
    Ok(())
}

pub(super) fn viewport_size(
    window: isize,
    max_width: u32,
    max_height: u32,
) -> Result<(u32, u32), String> {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(window as HWND, &mut rect) } == 0 {
        return Err("native presentation parent is unavailable".into());
    }
    Ok(fit_size(
        rect.right.max(1) as u32,
        rect.bottom.max(1) as u32,
        max_width,
        max_height,
    ))
}
fn fit_size(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    let scale = (max_width as f64 / width as f64)
        .min(max_height as f64 / height as f64)
        .min(1.0);
    (
        (width as f64 * scale)
            .round()
            .clamp(128.0, max_width as f64) as u32,
        (height as f64 * scale)
            .round()
            .clamp(128.0, max_height as f64) as u32,
    )
}

fn to_qpc(target: i64, mpv_now: i64, qpc_now: i64, frequency: i64) -> Result<i64, String> {
    let ticks =
        qpc_now as i128 + (target as i128 - mpv_now as i128) * frequency as i128 / 1_000_000_000;
    i64::try_from(ticks)
        .ok()
        .filter(|value| *value >= 0)
        .ok_or_else(|| "invalid mpv presentation clock".into())
}
fn observe(mpv: Mpv, state: &Shared, initial_fps: f64) -> Result<(), String> {
    for (id, name, format) in [
        (1, "pause", Format::Flag),
        (2, "speed", Format::Double),
        (3, "estimated-vf-fps", Format::Double),
    ] {
        mpv.observe_property(name, format, id)
            .map_err(|e| e.to_string())?;
    }
    let mut speed = 1.0;
    let mut fps = initial_fps;
    while !state.stop.load(Ordering::Acquire) {
        match mpv.wait_event(0.05) {
            Some(Ok(Event::PropertyChange {
                name: "pause",
                change: PropertyData::Flag(paused),
                ..
            })) => {
                if state.paused.swap(paused, Ordering::AcqRel) != paused {
                    state.invalidate();
                }
            }
            Some(Ok(Event::PropertyChange {
                name: "speed",
                change: PropertyData::Double(value),
                ..
            })) if value.is_finite() && value > 0.0 => {
                if value != speed {
                    speed = value;
                    state.invalidate();
                }
            }
            Some(Ok(Event::PropertyChange {
                name: "estimated-vf-fps",
                change: PropertyData::Double(value),
                ..
            })) if value.is_finite() && value > 0.0 => {
                fps = value;
            }
            Some(Ok(Event::Seek | Event::StartFile | Event::EndFile(_) | Event::QueueOverflow)) => {
                state.invalidate()
            }
            Some(Ok(Event::Shutdown)) => break,
            _ => {}
        }
        state.duration_ns.store(
            (1e9 / (fps * speed)).clamp(1.0, 10e9) as u64,
            Ordering::Release,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mpv_nanoseconds_are_converted_to_qpc_without_overflow() {
        assert_eq!(
            to_qpc(2_000_000_000, 1_000_000_000, 50, 10_000_000).unwrap(),
            10_000_050
        );
        assert_eq!(
            to_qpc(1_000_000_000, 2_000_000_000, 20_000_000, 10_000_000).unwrap(),
            10_000_000
        );
        assert!(to_qpc(i64::MAX, 0, i64::MAX, i64::MAX).is_err());
    }
    #[test]
    fn viewport_fits_capacity_and_preserves_normal_window_aspect() {
        assert_eq!(fit_size(3840, 2160, 1920, 1080), (1920, 1080));
        assert_eq!(fit_size(800, 500, 1920, 1080), (800, 500));
        assert_eq!(fit_size(1080, 1920, 1920, 1080), (608, 1080));
    }
}
