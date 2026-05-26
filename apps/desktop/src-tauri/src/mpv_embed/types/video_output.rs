#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MpvVideoOutputConfig {
    pub(crate) vo: Option<String>,
    pub(crate) gpu_context: Option<String>,
    pub(crate) hwdec: String,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) struct LinuxVideoOutputEnvironment<'a> {
    pub(crate) override_vo: Option<&'a str>,
    pub(crate) override_gpu_context: Option<&'a str>,
    pub(crate) override_hwdec: Option<&'a str>,
    pub(crate) has_dri_render_node: bool,
    pub(crate) virtual_drm_driver: bool,
}
