use anyhow::{Context, Result, anyhow};
use libmpv2::Mpv;
use libmpv2_sys as sys;
use std::{
    ffi::{CStr, c_void},
    ptr::{self, NonNull},
    sync::Arc,
};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub(super) struct RenderContext {
    pub(super) raw: NonNull<sys::mpv_render_context>,
    _mpv: Arc<Mpv>,
    updates: Box<Sender<()>>,
}

impl RenderContext {
    pub(super) fn create(
        mpv: Arc<Mpv>,
        params: &mut [sys::mpv_render_param],
    ) -> Result<(Self, Receiver<()>)> {
        let mut raw = ptr::null_mut();
        mpv_result(unsafe {
            sys::mpv_render_context_create(&mut raw, mpv.ctx.as_ptr(), params.as_mut_ptr())
        })
        .context("initialize libmpv render API")?;
        let (sender, receiver) = mpsc::channel(1);
        let mut render = Self {
            raw: NonNull::new(raw).context("libmpv did not return a render context")?,
            _mpv: mpv,
            updates: Box::new(sender),
        };
        unsafe {
            sys::mpv_render_context_set_update_callback(
                raw,
                Some(wake_render),
                ptr::from_mut(render.updates.as_mut()).cast(),
            );
        }
        Ok((render, receiver))
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        unsafe {
            sys::mpv_render_context_set_update_callback(self.raw.as_ptr(), None, ptr::null_mut());
            sys::mpv_render_context_free(self.raw.as_ptr());
        }
    }
}

unsafe extern "C" fn wake_render(userdata: *mut c_void) {
    // libmpv may call back on any thread. One queued wake is enough.
    let sender = unsafe { &*userdata.cast::<Sender<()>>() };
    let _ = sender.try_send(());
}

pub(super) fn render_param<T>(
    type_: sys::mpv_render_param_type,
    data: &mut T,
) -> sys::mpv_render_param {
    sys::mpv_render_param {
        type_,
        data: ptr::from_mut(data).cast(),
    }
}

pub(super) fn param_end() -> sys::mpv_render_param {
    sys::mpv_render_param {
        type_: sys::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
        data: ptr::null_mut(),
    }
}

pub(super) fn mpv_result(status: i32) -> Result<()> {
    if status < 0 {
        let error = unsafe { CStr::from_ptr(sys::mpv_error_string(status)) };
        return Err(anyhow!("{}", error.to_string_lossy()));
    }
    Ok(())
}
