use thiserror::Error;

pub trait MediaBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBackendInfo {
    pub backend_id: String,
    pub display_name: String,
}

impl MediaBackendInfo {
    pub fn from_backend(backend: &dyn MediaBackend) -> Self {
        Self {
            backend_id: backend.backend_id().to_string(),
            display_name: backend.display_name().to_string(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaError {
    #[error("media backend is unavailable: {0}")]
    BackendUnavailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;

    impl MediaBackend for TestBackend {
        fn backend_id(&self) -> &'static str {
            "test"
        }

        fn display_name(&self) -> &'static str {
            "Test Backend"
        }
    }

    #[test]
    fn backend_info_is_derived_from_trait() {
        let info = MediaBackendInfo::from_backend(&TestBackend);

        assert_eq!(info.backend_id, "test");
        assert_eq!(info.display_name, "Test Backend");
    }
}
