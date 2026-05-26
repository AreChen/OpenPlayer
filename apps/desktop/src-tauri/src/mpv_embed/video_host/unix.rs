use super::super::*;
use super::raw_handle::window_mpv_wid;

impl MpvVideoHost {
    pub(in crate::mpv_embed) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        Ok(Self {
            wid: window_mpv_wid(window)?,
        })
    }

    pub(in crate::mpv_embed) fn wid(&self) -> i64 {
        self.wid
    }

    pub(in crate::mpv_embed) fn resize(&self) -> Result<(), String> {
        Ok(())
    }
}
