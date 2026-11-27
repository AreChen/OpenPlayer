mod decode;
mod input;
mod media_properties;
mod quick_open;
mod timeline;
mod volume;

use std::{
    ffi::c_void,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Context, DispatchPhase, Div, ExternalPaths, FocusHandle, FontWeight, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathPromptOptions, Pixels, Platform,
    Point, Render, SharedString, Stateful, Styled, Task, Window, WindowControlArea, canvas,
    deferred, div, prelude::*, px, rgb, transparent_black,
};
use gpui_ce_motion::{Motion, MotionState, Spring, Target, Transition};
use libmpv2::Mpv;
use serde_json::{Value, json};
use windows::{
    Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SW_MAXIMIZE, SW_RESTORE,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ShowWindowAsync, SystemParametersInfoW,
        },
    },
    core::BOOL,
};

use self::{
    media_properties::{MediaProperties, MediaPropertiesStatus},
    quick_open::{QuickOpen, QuickOpenMode},
    timeline::{PlaybackClock, SAMPLE_INTERVAL, time_label},
    volume::VolumeControl,
};
use super::{
    VideoRenderer,
    controller::Controller,
    hwnd,
    settings::ExplorerPreview,
    theme::{Palette, icon, logo},
    ui_state::UiState,
    updates::{self, UpdateStatus},
    widgets::{button_feedback, tooltip},
};

const CHROME_IDLE: Duration = Duration::from_secs(3);
const FEEDBACK_TIME: Duration = Duration::from_millis(1100);
const SETTLE_TIME: Duration = Duration::from_millis(450);
const AUDIO_BAR_CYCLE: f64 = 2.6;
const AUDIO_BAR_HEIGHTS: [f32; 12] = [24., 42., 31., 54., 72., 58., 79., 48., 32., 54., 38., 24.];

#[derive(Clone, Debug, PartialEq)]
pub(super) enum EditTarget {
    Stream,
    Accent,
}

pub(super) struct PlayerView {
    renderer: Option<VideoRenderer>,
    render_updates: Option<Task<()>>,
    video_adapter: Option<u32>,
    controller: Controller,
    platform: Rc<dyn Platform>,
    pub(super) ui: Arc<UiState>,
    pub(super) palette: Palette,
    focus: FocusHandle,
    last_activity: Instant,
    last_pointer: Option<Point<Pixels>>,
    was_hovered: bool,
    chrome_visible: bool,
    clock: Instant,
    playback_clock: PlaybackClock,
    chrome_motion: MotionState,
    panel_motion: MotionState,
    decode_motion: MotionState,
    volume_control: VolumeControl,
    reduced_motion: bool,
    sample_task: Option<Task<()>>,
    sample_until: Instant,
    idle_task: Option<Task<()>>,
    feedback_task: Option<Task<()>>,
    indicator: Option<(&'static str, SharedString)>,
    drag_origin: Option<Point<Pixels>>,
    last_stage_click: Option<(Instant, Point<Pixels>)>,
    pub(super) picker_open: bool,
    quick_open: Option<QuickOpen>,
    media_properties: Option<MediaProperties>,
    media_properties_task: Option<Task<()>>,
    restore_quick_focus: bool,
    pub(super) seek_preview: Option<f64>,
    topmost: bool,
    video_error: Option<String>,
    pub(super) edit: Option<(EditTarget, String)>,
    pub(super) show_history: bool,
    pub(super) update_status: UpdateStatus,
    pub(super) explorer_preview: ExplorerPreview,
}

impl PlayerView {
    pub(super) fn new(
        video_adapter: Option<u32>,
        mut controller: Controller,
        platform: Rc<dyn Platform>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        cx.observe_window_activation(window, |view, window, cx| {
            if window.is_window_active() {
                view.focus.focus(window, cx);
            }
            cx.notify();
        })
        .detach();

        let ui = controller.snapshot();
        let now = Instant::now();
        let mut view = Self {
            renderer: None,
            render_updates: None,
            video_adapter,
            controller,
            platform,
            palette: palette_for(&ui),
            playback_clock: PlaybackClock::new(&ui),
            ui,
            focus,
            last_activity: now,
            last_pointer: None,
            was_hovered: false,
            chrome_visible: false,
            clock: now,
            chrome_motion: MotionState::default(),
            panel_motion: MotionState::default(),
            decode_motion: MotionState::default(),
            volume_control: VolumeControl::default(),
            reduced_motion: system_reduced_motion(),
            sample_task: None,
            sample_until: now + SETTLE_TIME,
            idle_task: None,
            feedback_task: None,
            indicator: None,
            drag_origin: None,
            last_stage_click: None,
            picker_open: false,
            quick_open: None,
            media_properties: None,
            media_properties_task: None,
            restore_quick_focus: false,
            seek_preview: None,
            topmost: false,
            video_error: None,
            edit: None,
            show_history: false,
            update_status: UpdateStatus::default(),
            explorer_preview: ExplorerPreview::new(),
        };
        view.schedule_samples(cx);
        view.arm_idle(cx);
        view
    }

    fn sync_playback_core(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.controller.should_release_playback_core() {
            self.render_updates = None;
            if let Some(mut renderer) = self.renderer.take() {
                renderer.detach(window);
            }
            self.controller.detach_playback_core();
            self.video_error = None;
        }
        if !self.controller.awaiting_playback_core() {
            return;
        }

        let prepared = (|| -> anyhow::Result<_> {
            let mpv = Arc::new(Mpv::with_initializer(|init| {
                init.set_option("vo", "libmpv")?;
                init.set_option("hwdec", "no")?;
                init.set_option("keep-open", true)?;
                init.set_option("input-default-bindings", false)?;
                init.set_option("osc", false)?;
                Ok(())
            })?);
            if let Ok(version) = mpv.get_property::<String>("mpv-version") {
                eprintln!("OpenPlayer GPUI playback core: {version}");
            }
            let (mut renderer, updates) = VideoRenderer::new(mpv.clone(), self.video_adapter)?;
            renderer.attach(window)?;
            if let Err(error) = self.controller.attach_playback_core(mpv) {
                renderer.detach(window);
                return Err(error);
            }
            Ok((renderer, updates))
        })();
        match prepared {
            Ok((renderer, mut updates)) => {
                self.renderer = Some(renderer);
                self.render_updates = Some(cx.spawn_in(window, async move |view, cx| {
                    while updates.recv().await.is_some() {
                        if view.update_in(cx, |_, _, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                }));
            }
            Err(error) => self.controller.playback_core_failed(error),
        }
        let state = self.controller.snapshot();
        self.apply(state, cx, true);
        self.schedule_samples(cx);
    }

    fn apply(&mut self, state: Arc<UiState>, cx: &mut Context<Self>, from_command: bool) {
        let transition = self.ui.timeline_generation != state.timeline_generation
            || self.ui.loaded != state.loaded
            || self.ui.loading != state.loading
            || self.ui.playing != state.playing
            || self.ui.advancing != state.advancing
            || self.ui.error != state.error;
        if self.ui.stream.is_none() && state.stream.is_some() {
            self.edit = Some((EditTarget::Stream, String::new()));
        } else if (self.ui.stream.is_some() && state.stream.is_none())
            || (self.ui.settings.is_some() && state.settings.is_none() && self.ui.stream.is_none())
        {
            self.edit = None;
        }
        if self.ui.appearance != state.appearance
            || self.ui.titlebar_opacity != state.titlebar_opacity
            || self.ui.transport_opacity != state.transport_opacity
        {
            self.palette = palette_for(&state);
        }
        self.playback_clock.update(&state);
        self.ui = state;
        if from_command || transition || self.chrome_visible || self.ui.audio_only {
            cx.notify();
        }
    }

    fn schedule_samples(&mut self, cx: &Context<Self>) {
        if self.sample_task.is_some() {
            return;
        }
        let executor = cx.background_executor().clone();
        self.sample_task = Some(cx.spawn(async move |view, cx| {
            loop {
                executor.timer(SAMPLE_INTERVAL).await;
                let result = view.update(cx, |view, cx| {
                    let state = view.controller.snapshot();
                    view.apply(state, cx, false);
                    let keep_sampling =
                        view.controller.needs_poll() || Instant::now() < view.sample_until;
                    if !keep_sampling {
                        view.sample_task = None;
                    }
                    keep_sampling
                });
                if !result.unwrap_or(false) {
                    break;
                }
            }
        }));
    }

    fn drop_paths(&mut self, paths: &ExternalPaths, _: &mut Window, cx: &mut Context<Self>) {
        if self.media_properties.is_some() {
            return;
        }
        if !self.accept_quick_drop(&paths.0, cx) {
            self.send("paths", json!(paths.0.as_slice()), cx);
        }
    }

    fn open_quick_media_properties(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.exit_quick_address();
        self.restore_quick_focus = true;
        let title = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        self.media_properties = Some(MediaProperties {
            title: title.into(),
            location: Some(path.clone()),
            status: MediaPropertiesStatus::Loading,
            copied: false,
        });
        let work = cx
            .background_executor()
            .spawn(async move { super::media_info::probe_local(&path) });
        self.media_properties_task = Some(cx.spawn(async move |view, cx| {
            let result = work.await;
            let _ = view.update(cx, |view, cx| {
                if let Some(properties) = &mut view.media_properties {
                    properties.status = match result {
                        Ok(info) => MediaPropertiesStatus::Ready(Box::new(info)),
                        Err(error) => {
                            eprintln!("OpenPlayer media properties: {error:#}");
                            MediaPropertiesStatus::Failed
                        }
                    };
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    fn open_active_media_properties(&mut self, cx: &mut Context<Self>) {
        if let Some((title, location, info)) = self.controller.active_media_info() {
            self.media_properties_task = None;
            self.media_properties = Some(MediaProperties {
                title: title.into(),
                location,
                status: MediaPropertiesStatus::Ready(Box::new(info)),
                copied: false,
            });
            cx.notify();
        }
    }

    fn close_media_properties(&mut self, cx: &mut Context<Self>) {
        self.media_properties_task = None;
        self.media_properties = None;
        self.activity(cx);
        cx.notify();
    }

    pub(super) fn send(&mut self, kind: &'static str, value: Value, cx: &mut Context<Self>) {
        match kind {
            "checkForUpdates" => {
                self.check_for_updates(cx);
                return;
            }
            "downloadUpdate" => {
                if let UpdateStatus::Available(update) = &self.update_status {
                    cx.open_url(&update.url);
                }
                return;
            }
            "togglePlayback" if self.ui.media.is_none() && !self.controller.has_queue() => {
                self.show_quick_open(QuickOpenMode::Open, cx);
                return;
            }
            "openMedia" | "appendFiles" | "appendFolder" => {
                let mode = match kind {
                    "appendFiles" => QuickOpenMode::AppendFiles,
                    "appendFolder" => QuickOpenMode::AppendFolder,
                    _ => QuickOpenMode::Open,
                };
                self.show_quick_open(mode, cx);
                return;
            }
            "addSubtitle" => {
                self.prompt_subtitle(cx);
                return;
            }
            _ => self.controller.dispatch(kind, value),
        }
        let state = self.controller.snapshot();
        self.apply(state, cx, true);
        if self.ui.settings.as_deref() == Some("about")
            && matches!(self.update_status, UpdateStatus::Idle)
        {
            self.check_for_updates(cx);
        }
        self.sample_until = Instant::now() + SETTLE_TIME;
        self.schedule_samples(cx);
        if kind == "toggleMute" {
            self.show_volume(cx);
        }
    }

    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if matches!(self.update_status, UpdateStatus::Checking) {
            return;
        }
        self.update_status = UpdateStatus::Checking;
        cx.notify();
        let check = cx.background_executor().spawn(async { updates::check() });
        cx.spawn(async move |view, cx| {
            let result = check.await;
            let _ = view.update(cx, |view, cx| {
                view.update_status = match result {
                    Ok(Some(update)) => UpdateStatus::Available(update),
                    Ok(None) => UpdateStatus::Current,
                    Err(error) => UpdateStatus::Failed(format!("{error:#}")),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn send_in_window(
        &mut self,
        kind: &'static str,
        value: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind != "contextItem" {
            self.send(kind, value, cx);
            return;
        }
        match value.as_str() {
            Some("toggleFullscreen") => {
                self.send("closeContext", Value::Null, cx);
                self.toggle_fullscreen(window, cx);
            }
            Some("toggleAlwaysOnTop") => {
                self.send("closeContext", Value::Null, cx);
                self.toggle_topmost(window, cx);
            }
            Some("closeWindow") => window.remove_window(),
            Some("mediaProperties") => {
                self.send("closeContext", Value::Null, cx);
                self.open_active_media_properties(cx);
            }
            Some(action @ ("openMedia" | "appendFiles" | "appendFolder" | "addSubtitle")) => {
                self.send("closeContext", Value::Null, cx);
                match action {
                    "addSubtitle" => self.prompt_subtitle(cx),
                    "appendFiles" => self.show_quick_open(QuickOpenMode::AppendFiles, cx),
                    "appendFolder" => self.show_quick_open(QuickOpenMode::AppendFolder, cx),
                    _ => self.show_quick_open(QuickOpenMode::Open, cx),
                }
            }
            _ => self.send(kind, value, cx),
        }
    }

    fn prompt_subtitle(&mut self, cx: &mut Context<Self>) {
        if self.picker_open {
            return;
        }
        self.picker_open = true;
        self.last_stage_click = None;
        cx.notify();
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn(async move |view, cx| {
            let result = paths.await;
            let _ = view.update(cx, |view, cx| {
                view.picker_open = false;
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            view.send("addSubtitlePath", json!(path), cx);
                        }
                    }
                    Ok(Err(error)) => {
                        view.video_error = Some(error.to_string());
                    }
                    _ => {}
                }
                view.activity(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn activity(&mut self, cx: &mut Context<Self>) {
        if self.last_activity.elapsed() >= CHROME_IDLE {
            cx.notify();
        }
        self.last_activity = Instant::now();
        self.arm_idle(cx);
    }

    fn arm_idle(&mut self, cx: &Context<Self>) {
        if self.idle_task.is_some() {
            return;
        }
        let executor = cx.background_executor().clone();
        let mut remaining = CHROME_IDLE.saturating_sub(self.last_activity.elapsed());
        self.idle_task = Some(cx.spawn(async move |view, cx| {
            loop {
                executor.timer(remaining).await;
                let result = view.update(cx, |view, cx| {
                    let remaining = CHROME_IDLE.saturating_sub(view.last_activity.elapsed());
                    if remaining.is_zero() {
                        view.idle_task = None;
                        cx.notify();
                    }
                    remaining
                });
                match result {
                    Ok(delay) if !delay.is_zero() => remaining = delay,
                    _ => break,
                }
            }
        }));
    }

    fn show_indicator(&mut self, glyph: &'static str, value: SharedString, cx: &mut Context<Self>) {
        self.indicator = Some((glyph, value));
        let executor = cx.background_executor().clone();
        self.feedback_task = Some(cx.spawn(async move |view, cx| {
            executor.timer(FEEDBACK_TIME).await;
            let _ = view.update(cx, |view, cx| {
                view.feedback_task = None;
                view.indicator = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn show_volume(&mut self, cx: &mut Context<Self>) {
        self.show_indicator(self.volume_glyph(), self.volume_text().into(), cx);
    }

    fn edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some((_, draft)) = self.edit.as_mut() else {
            return;
        };
        cx.stop_propagation();
        let stroke = &event.keystroke;
        match stroke.key.as_str() {
            "enter" => self.commit_edit(cx),
            "backspace" => {
                draft.pop();
            }
            "delete" => draft.clear(),
            "a" if stroke.modifiers.control => draft.clear(),
            "v" if stroke.modifiers.control => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    draft.push_str(text.trim());
                }
            }
            _ => {
                if let Some(text) = stroke
                    .key_char
                    .as_ref()
                    .filter(|_| !stroke.modifiers.control)
                {
                    draft.push_str(text);
                }
            }
        }
        cx.notify();
    }

    pub(super) fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text)) = self.edit.take() {
            match target {
                EditTarget::Stream => {
                    self.send("submitStream", json!(text), cx);
                    if self.ui.stream.is_some() {
                        self.edit = Some((EditTarget::Stream, text));
                    }
                }
                EditTarget::Accent => self.send("accent", json!(text), cx),
            }
        }
    }

    pub(super) fn timeline(&self) -> (f64, String, String) {
        let position = self.seek_preview.unwrap_or_else(|| {
            self.playback_clock
                .position(Instant::now(), self.ui.duration)
        });
        (
            position,
            time_label(position, &self.ui),
            time_label(self.ui.duration, &self.ui),
        )
    }

    fn titlebar(&self, window: &Window) -> Div {
        let palette = self.palette;
        let (maximize_icon, maximize_label) = if window.is_maximized() {
            ("restore", "controls.restore")
        } else {
            ("maximize", "controls.maximizeWindow")
        };
        let caption_button =
            |name, label| tooltip(caption(name, palette), self.ui.label(label), palette);
        div()
            .absolute()
            .top(px(10.))
            .right(px(24.))
            .h(px(30.))
            .flex()
            .rounded(px(9.))
            .overflow_hidden()
            .bg(palette.titlebar)
            .occlude()
            .child(
                caption_button("minimize", "controls.minimize")
                    .window_control_area(WindowControlArea::Min),
            )
            .child(
                // Client-area hit testing keeps Windows from showing Snap Layout on hover.
                caption_button(maximize_icon, maximize_label).on_click(|_, window, _| {
                    if let Ok(handle) = hwnd(window) {
                        let show = if window.is_maximized() {
                            SW_RESTORE
                        } else {
                            SW_MAXIMIZE
                        };
                        let _ = unsafe { ShowWindowAsync(HWND(handle as *mut c_void), show) };
                    }
                }),
            )
            .child(
                caption_button("close", "controls.close")
                    .window_control_area(WindowControlArea::Close),
            )
    }

    fn stage(&self, window: &Window, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let viewport = f32::from(window.viewport_size().width);
        let open_label = self.ui.label(if self.picker_open {
            "media.openingFilePicker"
        } else {
            "media.doubleClickOpen"
        });
        div().absolute().inset_0()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::pointer_down))
            .on_mouse_up(MouseButton::Left, cx.listener(|view, _, _, _| view.drag_origin = None))
            .on_mouse_down(MouseButton::Middle, cx.listener(|view, _, window, cx| {
                view.toggle_fullscreen(window, cx);
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(|view, event: &MouseDownEvent, _, cx| {
                view.send("context", json!({ "x": f32::from(event.position.x), "y": f32::from(event.position.y) }), cx);
                view.activity(cx);
            }))
            .on_scroll(cx.listener(Self::wheel))
            .when(self.ui.media.is_none(), |el| el.bg(palette.surface).flex().items_center().justify_center().child(
                div().flex().flex_col().items_center().gap_2()
                    .child(logo(palette.accent, (viewport * 0.105).clamp(84., 136.)))
                    .child(div().text_size(px((viewport * 0.02).clamp(18., 26.)))
                        .text_color(palette.muted)
                        .font_weight(FontWeight::SEMIBOLD).child(open_label))))
            .when_some(self.ui.media.as_ref().filter(|_| self.ui.audio_only), |el, media| {
                let codec = self.ui.audio_codec.as_deref().unwrap_or("audio").to_uppercase();
                let phase = if self.ui.duration > 0. {
                    self.playback_clock.position(Instant::now(), self.ui.duration)
                } else {
                    self.playback_clock.position(Instant::now(), f64::INFINITY)
                } / AUDIO_BAR_CYCLE;
                let (_, current, total) = self.timeline();
                el.bg(palette.surface).flex().items_center().justify_center().child(
                    div().flex().flex_col().items_center().gap_3()
                        .child(div().flex().items_center().gap_2().children(AUDIO_BAR_HEIGHTS.into_iter().enumerate().map(|(index, base)| {
                            let height = if self.reduced_motion {
                                base
                            } else {
                                let wave = ((phase - index as f64 * 0.12) * std::f64::consts::TAU).cos();
                                base * (0.86 + wave as f32 * 0.14)
                            };
                            div().w(px(7.)).h(px(height)).rounded_full().bg(palette.accent_border)
                        })))
                        .child(div().font_weight(FontWeight::BOLD).text_size(px(22.)).child(media.name.clone()))
                        .child(div().text_color(palette.muted).child(format!("{codec} · {current} / {total}"))))
            })
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.restore_quick_focus) {
            self.focus.focus(window, cx);
        }
        self.sync_playback_core(window, cx);
        if self.video_error.is_none()
            && let Some(renderer) = &mut self.renderer
            && let Err(error) = renderer.draw(window)
        {
            self.video_error = Some(error.to_string());
            eprintln!("D3D11 video draw failed: {error:#}");
        }
        if self.ui.audio_only
            && self.ui.advancing
            && !self.reduced_motion
            && window.visibility().is_visible()
        {
            window.request_animation_frame();
        }
        let has_panel = self.ui.panel.is_some()
            || self.ui.playlist
            || self.ui.settings.is_some()
            || self.ui.stream.is_some();
        let pinned = has_panel
            || self.ui.context.is_some()
            || self.picker_open
            || self.quick_open.is_some()
            || self.media_properties.is_some()
            || self.seek_preview.is_some();
        let hovered = window.is_window_hovered();
        if hovered && !self.was_hovered {
            self.last_activity = Instant::now();
        }
        if !hovered {
            self.last_pointer = None;
        }
        self.was_hovered = hovered;
        let idle = self.last_activity.elapsed() >= CHROME_IDLE;
        let show_chrome = pinned || (hovered && !idle);
        self.chrome_visible = show_chrome;
        let volume_allowed = show_chrome
            && hovered
            && self.ui.context.is_none()
            && self.ui.settings.is_none()
            && self.ui.stream.is_none()
            && !self.picker_open
            && self.quick_open.is_none()
            && self.media_properties.is_none();
        if !volume_allowed {
            self.volume_control.reset();
        }
        if hovered && !idle {
            self.arm_idle(cx);
        }
        if hovered && idle && !pinned && self.platform.is_cursor_visible() {
            self.platform.hide_cursor_until_mouse_moves();
        }
        let chrome = motion(
            &mut self.chrome_motion,
            self.clock,
            show_chrome,
            self.reduced_motion,
            window,
        );
        let panel = motion(
            &mut self.panel_motion,
            self.clock,
            has_panel,
            self.reduced_motion,
            window,
        );

        let view = cx.weak_entity();
        let release_view = view.clone();
        let pointer_activity = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                window.on_mouse_event(
                    move |event: &MouseMoveEvent, phase, window, cx: &mut App| {
                        if phase == DispatchPhase::Capture {
                            let _ = view.update(cx, |view, cx| {
                                if view.last_pointer != Some(event.position) {
                                    view.last_pointer = Some(event.position);
                                    view.activity(cx);
                                }
                                view.pointer_move(event, window, cx);
                            });
                        }
                    },
                );
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx: &mut App| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        let _ = release_view.update(cx, |view, cx| {
                            view.finish_volume_drag(event.position.y, cx);
                        });
                    }
                });
            },
        )
        .size(px(0.));

        let palette = self.palette;
        let mut overlay = div()
            .id("openplayer")
            .absolute()
            .inset_0()
            .font_family("Segoe UI")
            .text_size(px(14.))
            .text_color(palette.text)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::keyboard))
            .on_drop(cx.listener(Self::drop_paths))
            .child(pointer_activity)
            .child(self.stage(window, cx));

        if chrome.is_mounted() {
            if show_chrome
                && self.ui.advancing
                && self.ui.duration > 0.
                && window.visibility().is_visible()
            {
                window.request_animation_frame();
            }
            let target = chrome.target();
            let controls = div()
                .absolute()
                .inset_0()
                .opacity(target.alpha().clamp(0., 1.))
                .when(!window.is_fullscreen(), |el| {
                    el.child(self.titlebar(window))
                })
                .child(self.transport(target.y_px(), volume_allowed, window, cx));
            overlay = overlay.child(controls);
        }
        overlay = overlay
            .when_some(self.ui.context.as_ref(), |el, menu| {
                el.child(self.context_menu(menu, window, cx))
            })
            .when(has_panel && panel.is_mounted(), |el| {
                el.child(self.panels(
                    window,
                    panel.target().alpha().clamp(0., 1.),
                    panel.target().y_px(),
                    cx,
                ))
            })
            .when_some(self.indicator.as_ref(), |el, (glyph, value)| {
                el.child(
                    div()
                        .absolute()
                        .top(px(10.))
                        .left(px(24.))
                        .min_w(px(104.))
                        .h(px(30.))
                        .px_3()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .rounded(px(9.))
                        .bg(palette.titlebar)
                        .text_color(palette.accent)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon(*glyph, palette.accent, 14.))
                        .child(value.clone()),
                )
            })
            .when_some(
                self.video_error.as_ref().or(self.ui.error.as_ref()),
                |el, message| {
                    el.child(
                        div()
                            .absolute()
                            .left_6()
                            .top(px(52.))
                            .max_w(px(540.))
                            .rounded_md()
                            .p_3()
                            .bg(palette.strong)
                            .border_1()
                            .border_color(palette.danger)
                            .text_color(palette.danger)
                            .child(message.clone()),
                    )
                },
            )
            .when_some(self.ui.feedback.as_ref(), |el, message| {
                el.child(
                    div()
                        .absolute()
                        .top(px(54.))
                        .right_6()
                        .rounded_md()
                        .px_3()
                        .py_2()
                        .bg(palette.strong)
                        .child(message.clone()),
                )
            });
        if self.quick_open.is_some() {
            overlay = overlay.child(self.quick_open_view(window, cx));
        }
        if self.media_properties.is_some() {
            overlay = overlay.child(self.media_properties_view(window, cx));
        }
        div()
            .size_full()
            .bg(rgb(0x000000))
            .child(deferred(overlay).priority(1))
    }
}

fn palette_for(state: &UiState) -> Palette {
    Palette::new(
        state.appearance.as_ref(),
        state.titlebar_opacity,
        state.transport_opacity,
    )
}

fn system_reduced_motion() -> bool {
    let mut animate = BOOL(1);
    let known = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&raw mut animate).cast::<c_void>()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .is_ok();
    known && !animate.as_bool()
}

fn motion(
    state: &mut MotionState,
    start: Instant,
    present: bool,
    reduced: bool,
    window: &mut Window,
) -> gpui_ce_motion::Frame {
    let hidden = Target::default().y(12.).opacity(0.);
    let transition = Motion::new(Target::default().opacity(1.))
        .initial(hidden)
        .exit(hidden)
        .present(present)
        .transition(Transition::Spring(Spring::default()));
    let frame = state.sample(transition, start.elapsed(), reduced);
    if frame.is_animating() {
        window.request_animation_frame();
    }
    frame
}

fn caption(name: &'static str, palette: Palette) -> Stateful<Div> {
    let id = if matches!(name, "maximize" | "restore") {
        "maximize-window"
    } else {
        name
    };
    let button = div()
        .id(id)
        .w(px(38.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(7.))
        .border_1()
        .border_color(transparent_black())
        .child(icon(name, palette.accent, 17.));
    if name == "close" {
        button
            .hover(move |style| style.bg(rgb(0x7f2f28)).border_color(palette.danger))
            .active(move |style| style.bg(rgb(0xa03b32)).border_color(palette.danger))
    } else {
        button_feedback(button, palette)
    }
}
