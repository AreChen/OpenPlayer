use std::{
    ffi::{c_char, c_void},
    ptr,
};

use super::super::*;
use super::errors::mpv_error_message;

pub(in crate::mpv_embed) fn create_macos_render_context(
    mpv: &libmpv2::Mpv,
    host: &MpvVideoHost,
) -> Result<MacosMpvRenderContext, String> {
    unsafe {
        openplayer_mpv_gl_view_make_current(host.render_view_ptr());
    }

    let mut init_params = libmpv2_sys::mpv_opengl_init_params {
        get_proc_address: Some(macos_mpv_get_proc_address),
        get_proc_address_ctx: ptr::null_mut(),
    };
    let mut render_params = [
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            data: libmpv2_sys::MPV_RENDER_API_TYPE_OPENGL.as_ptr() as *mut c_void,
        },
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            data: (&mut init_params as *mut libmpv2_sys::mpv_opengl_init_params).cast(),
        },
        libmpv2_sys::mpv_render_param {
            type_: 0,
            data: ptr::null_mut(),
        },
    ];
    let mut context: *mut libmpv2_sys::mpv_render_context = ptr::null_mut();
    let result = unsafe {
        libmpv2_sys::mpv_render_context_create(
            &mut context,
            mpv.ctx.as_ptr(),
            render_params.as_mut_ptr(),
        )
    };
    if result < 0 {
        return Err(format!(
            "mpv render context init failed: {}",
            mpv_error_message(result)
        ));
    }

    unsafe {
        openplayer_mpv_gl_view_set_render_context(host.render_view_ptr(), context.cast());
        libmpv2_sys::mpv_render_context_set_update_callback(
            context,
            Some(macos_mpv_render_update),
            host.render_view_ptr(),
        );
    }

    Ok(MacosMpvRenderContext {
        ctx: context as usize,
        view: host.render_view,
    })
}

impl Drop for MacosMpvRenderContext {
    fn drop(&mut self) {
        let context = self.ctx as *mut libmpv2_sys::mpv_render_context;
        unsafe {
            libmpv2_sys::mpv_render_context_set_update_callback(context, None, ptr::null_mut());
            openplayer_mpv_gl_view_set_render_context(self.view as *mut c_void, ptr::null_mut());
            libmpv2_sys::mpv_render_context_free(context);
        }
    }
}

unsafe extern "C" fn macos_mpv_get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    unsafe { openplayer_mpv_gl_get_proc_address(name) }
}

unsafe extern "C" fn macos_mpv_render_update(ctx: *mut c_void) {
    unsafe {
        openplayer_mpv_gl_view_draw(ctx);
    }
}
