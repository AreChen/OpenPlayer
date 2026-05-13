use openplayer_media::MediaBackend;

#[derive(Debug, Default, Clone, Copy)]
pub struct MpvBackendDescriptor;

impl MediaBackend for MpvBackendDescriptor {
    fn backend_id(&self) -> &'static str {
        "mpv"
    }

    fn display_name(&self) -> &'static str {
        "libmpv"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::MediaBackendInfo;

    #[test]
    fn exposes_mpv_backend_identity() {
        let descriptor = MpvBackendDescriptor;
        let info = MediaBackendInfo::from_backend(&descriptor);

        assert_eq!(info.backend_id, "mpv");
        assert_eq!(info.display_name, "libmpv");
    }
}
