use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use gpui::{RenderImage, Window};
use image::{Frame, RgbaImage};
use libmpv2::Mpv;
use libmpv2_sys as sys;
use tokio::sync::mpsc::Receiver;

use super::render_context::{RenderContext, mpv_result, param_end, render_param};

/// A libmpv frame is uploaded into the same GPUI scene as the transport. This also
/// works on Wayland, where clients cannot embed another client's native window.
pub(super) struct VideoRenderer {
    render: RenderContext,
    frame: Option<Arc<RenderImage>>,
    pixels: Vec<AlignedRowBlock>,
    dimensions: (u32, u32),
}

#[repr(C, align(64))]
#[derive(Clone)]
struct AlignedRowBlock([u8; 64]);

impl VideoRenderer {
    pub(super) fn new(mpv: Arc<Mpv>, _: Option<u32>) -> Result<(Self, Receiver<()>)> {
        let mut params = [
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: c"sw".as_ptr().cast_mut().cast(),
            },
            param_end(),
        ];
        let (render, updates) = RenderContext::create(mpv, &mut params)?;
        Ok((
            Self {
                render,
                frame: None,
                pixels: Vec::new(),
                dimensions: (0, 0),
            },
            updates,
        ))
    }

    pub(super) fn attach(&mut self, _: &Window) -> Result<()> {
        Ok(())
    }

    pub(super) fn detach(&mut self, window: &mut Window) {
        if let Some(previous) = self.frame.take() {
            let _ = window.drop_image(previous);
        }
    }

    pub(super) fn frame(&self) -> Option<Arc<RenderImage>> {
        self.frame.clone()
    }

    pub(super) fn draw(&mut self, window: &mut Window) -> Result<()> {
        let viewport = window.viewport_size();
        let scale = window.scale_factor();
        let width = (f32::from(viewport.width) * scale).round() as u32;
        let height = (f32::from(viewport.height) * scale).round() as u32;
        let updates = unsafe { sys::mpv_render_context_update(self.render.raw.as_ptr()) };
        if width == 0
            || height == 0
            || (self.dimensions == (width, height)
                && updates & u64::from(sys::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME) == 0)
        {
            return Ok(());
        }

        let row = width as usize * 4;
        let mut stride = row.next_multiple_of(64);
        let bytes = stride
            .checked_mul(height as usize)
            .context("video viewport is too large")?;
        ensure!(bytes <= 512 * 1024 * 1024, "video surface exceeds 512 MiB");
        self.pixels.resize(bytes / 64, AlignedRowBlock([0; 64]));
        let mut size = [width as i32, height as i32];
        let mut wait = 0_i32;
        let mut params = [
            render_param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_SIZE,
                &mut size,
            ),
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_FORMAT,
                data: c"bgr0".as_ptr().cast_mut().cast(),
            },
            render_param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_STRIDE,
                &mut stride,
            ),
            sys::mpv_render_param {
                type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_SW_POINTER,
                data: self.pixels.as_mut_ptr().cast(),
            },
            render_param(
                sys::mpv_render_param_type_MPV_RENDER_PARAM_BLOCK_FOR_TARGET_TIME,
                &mut wait,
            ),
            param_end(),
        ];
        // libmpv writes synchronously into aligned rows. GPUI expects packed BGRA.
        mpv_result(unsafe {
            sys::mpv_render_context_render(self.render.raw.as_ptr(), params.as_mut_ptr())
        })?;
        let source =
            unsafe { std::slice::from_raw_parts(self.pixels.as_ptr().cast::<u8>(), bytes) };
        let mut bgra = Vec::with_capacity(row * height as usize);
        for line in source.chunks_exact(stride) {
            bgra.extend_from_slice(&line[..row]);
        }
        for alpha in bgra.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        let buffer =
            RgbaImage::from_raw(width, height, bgra).context("invalid video frame size")?;
        self.detach(window);
        self.frame = Some(Arc::new(RenderImage::new([Frame::new(buffer)])));
        self.dimensions = (width, height);
        unsafe { sys::mpv_render_context_report_swap(self.render.raw.as_ptr()) };
        Ok(())
    }
}
