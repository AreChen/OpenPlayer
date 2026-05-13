use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("storage is not configured")]
    NotConfigured,
}

pub fn storage_crate_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_crate_reports_ready() {
        assert!(storage_crate_ready());
    }
}
