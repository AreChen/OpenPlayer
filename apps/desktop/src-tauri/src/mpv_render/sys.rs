use std::{
    cell::Cell,
    ffi::{CStr, CString},
    os::raw::{c_char, c_double, c_int, c_void},
    ptr::null_mut,
    sync::mpsc,
};

use libmpv2_sys as mpv;

pub struct RawMpv {
    handle: *mut mpv::mpv_handle,
}

pub struct RawRenderContext {
    ctx: *mut mpv::mpv_render_context,
    update_sender: Cell<*mut mpsc::Sender<()>>,
}

// The render actor moves these raw handles onto its dedicated render thread.
unsafe impl Send for RawMpv {}
unsafe impl Send for RawRenderContext {}

impl RawMpv {
    pub fn new() -> Result<Self, String> {
        let handle = unsafe { mpv::mpv_create() };
        if handle.is_null() {
            return Err("mpv_create returned null".to_string());
        }

        let player = Self { handle };
        if let Err(error) = player.configure_and_initialize() {
            unsafe { mpv::mpv_terminate_destroy(player.handle) };
            std::mem::forget(player);
            return Err(error);
        }

        Ok(player)
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let cstrings = args
            .iter()
            .map(|arg| {
                CString::new(*arg).map_err(|_| format!("mpv command contains null byte: {arg}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = cstrings
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Vec<*const c_char>>();
        pointers.push(std::ptr::null());

        mpv_result(
            unsafe { mpv::mpv_command(self.handle, pointers.as_mut_ptr()) },
            "mpv_command",
        )
    }

    pub fn set_option_string(&self, name: &str, value: &str) -> Result<(), String> {
        let name =
            CString::new(name).map_err(|_| "mpv option name contains null byte".to_string())?;
        let value =
            CString::new(value).map_err(|_| "mpv option value contains null byte".to_string())?;

        mpv_result(
            unsafe { mpv::mpv_set_option_string(self.handle, name.as_ptr(), value.as_ptr()) },
            "mpv_set_option_string",
        )
    }

    pub fn set_flag_property(&self, name: &str, value: bool) -> Result<(), String> {
        let name =
            CString::new(name).map_err(|_| "mpv property name contains null byte".to_string())?;
        let mut flag: c_int = i32::from(value);

        mpv_result(
            unsafe {
                mpv::mpv_set_property(
                    self.handle,
                    name.as_ptr(),
                    mpv::mpv_format_MPV_FORMAT_FLAG,
                    (&mut flag as *mut c_int).cast(),
                )
            },
            "mpv_set_property flag",
        )
    }

    pub fn set_double_property(&self, name: &str, value: f64) -> Result<(), String> {
        let name =
            CString::new(name).map_err(|_| "mpv property name contains null byte".to_string())?;
        let mut value = value as c_double;

        mpv_result(
            unsafe {
                mpv::mpv_set_property(
                    self.handle,
                    name.as_ptr(),
                    mpv::mpv_format_MPV_FORMAT_DOUBLE,
                    (&mut value as *mut c_double).cast(),
                )
            },
            "mpv_set_property double",
        )
    }

    pub fn get_double_property(&self, name: &str) -> f64 {
        let Ok(name) = CString::new(name) else {
            return 0.0;
        };
        let mut value: c_double = 0.0;
        let result = unsafe {
            mpv::mpv_get_property(
                self.handle,
                name.as_ptr(),
                mpv::mpv_format_MPV_FORMAT_DOUBLE,
                (&mut value as *mut c_double).cast(),
            )
        };

        if result < 0 { 0.0 } else { value }
    }

    pub fn get_flag_property(&self, name: &str) -> bool {
        let Ok(name) = CString::new(name) else {
            return false;
        };
        let mut value: c_int = 0;
        let result = unsafe {
            mpv::mpv_get_property(
                self.handle,
                name.as_ptr(),
                mpv::mpv_format_MPV_FORMAT_FLAG,
                (&mut value as *mut c_int).cast(),
            )
        };

        result >= 0 && value != 0
    }

    pub fn create_render_context(
        &self,
        get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    ) -> Result<RawRenderContext, String> {
        let mut init_params = mpv::mpv_opengl_init_params {
            get_proc_address: Some(get_proc_address),
            get_proc_address_ctx: null_mut(),
        };
        let mut params = [
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: mpv::MPV_RENDER_API_TYPE_OPENGL.as_ptr().cast::<c_void>() as *mut c_void,
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: (&mut init_params as *mut mpv::mpv_opengl_init_params).cast(),
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                data: null_mut(),
            },
        ];
        let mut ctx = null_mut();

        mpv_result(
            unsafe { mpv::mpv_render_context_create(&mut ctx, self.handle, params.as_mut_ptr()) },
            "mpv_render_context_create",
        )?;

        Ok(RawRenderContext {
            ctx,
            update_sender: Cell::new(null_mut()),
        })
    }

    fn configure_and_initialize(&self) -> Result<(), String> {
        self.set_option_string("vo", "libmpv")?;
        self.set_option_string("hwdec", "auto-safe")?;
        self.set_option_string("keep-open", "yes")?;
        mpv_result(
            unsafe { mpv::mpv_initialize(self.handle) },
            "mpv_initialize",
        )
    }
}

impl RawRenderContext {
    pub fn set_update_callback(&self, sender: mpsc::Sender<()>) {
        let boxed = Box::into_raw(Box::new(sender));
        unsafe {
            mpv::mpv_render_context_set_update_callback(
                self.ctx,
                Some(render_update_callback),
                boxed.cast(),
            )
        };

        drop_update_sender(self.update_sender.replace(boxed));
    }

    pub fn update(&self) -> bool {
        let flags = unsafe { mpv::mpv_render_context_update(self.ctx) };

        flags & u64::from(mpv::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME) != 0
    }

    pub fn render(&self, width: i32, height: i32) -> Result<(), String> {
        let mut fbo = mpv::mpv_opengl_fbo {
            fbo: 0,
            w: width,
            h: height,
            internal_format: 0,
        };
        let mut flip_y: c_int = 1;
        let mut params = [
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
                data: (&mut fbo as *mut mpv::mpv_opengl_fbo).cast(),
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
                data: (&mut flip_y as *mut c_int).cast(),
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                data: null_mut(),
            },
        ];

        mpv_result(
            unsafe { mpv::mpv_render_context_render(self.ctx, params.as_mut_ptr()) },
            "mpv_render_context_render",
        )
    }

    pub fn report_swap(&self) {
        unsafe { mpv::mpv_render_context_report_swap(self.ctx) };
    }
}

impl Drop for RawRenderContext {
    fn drop(&mut self) {
        unsafe { mpv::mpv_render_context_set_update_callback(self.ctx, None, null_mut()) };
        drop_update_sender(self.update_sender.replace(null_mut()));
        unsafe { mpv::mpv_render_context_free(self.ctx) };
    }
}

impl Drop for RawMpv {
    fn drop(&mut self) {
        unsafe { mpv::mpv_terminate_destroy(self.handle) };
    }
}

unsafe extern "C" fn render_update_callback(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }

    let sender = unsafe { &*(ctx as *const mpsc::Sender<()>) };
    let _ = sender.send(());
}

fn drop_update_sender(sender: *mut mpsc::Sender<()>) {
    if sender.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(sender)) };
}

fn mpv_result(code: c_int, operation: &str) -> Result<(), String> {
    if code >= 0 {
        return Ok(());
    }

    let message = unsafe {
        let raw = mpv::mpv_error_string(code);
        if raw.is_null() {
            "unknown mpv error".to_string()
        } else {
            CStr::from_ptr(raw).to_string_lossy().into_owned()
        }
    };

    Err(format!("{operation} failed: {message}"))
}
