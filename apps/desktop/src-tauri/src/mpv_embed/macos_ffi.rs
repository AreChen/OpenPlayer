use std::ffi::{c_char, c_void};

unsafe extern "C" {
    pub(super) fn openplayer_mpv_gl_view_create(parent: *mut c_void) -> *mut c_void;
    pub(super) fn openplayer_mpv_gl_view_remove(view: *mut c_void);
    pub(super) fn openplayer_mpv_gl_view_resize(view: *mut c_void);
    pub(super) fn openplayer_mpv_gl_view_set_render_context(
        view: *mut c_void,
        render_context: *mut c_void,
    );
    pub(super) fn openplayer_mpv_gl_view_make_current(view: *mut c_void);
    pub(super) fn openplayer_mpv_gl_view_draw(view: *mut c_void);
    pub(super) fn openplayer_mpv_gl_get_proc_address(name: *const c_char) -> *mut c_void;
}
