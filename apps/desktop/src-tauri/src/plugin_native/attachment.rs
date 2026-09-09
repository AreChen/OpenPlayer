//! One session-owned video lease with retained rollback on cleanup failure.
type Cleanup = Box<dyn FnMut() -> Result<(), String> + Send>;

#[derive(Default)]
pub(super) struct Attachment {
    cleanup: Option<Cleanup>,
    stopped: bool,
}

impl Attachment {
    #[cfg(all(windows, feature = "mpv-embed"))]
    pub(super) fn attached(&self) -> bool {
        self.cleanup.is_some()
    }

    #[cfg(any(test, all(windows, feature = "mpv-embed")))]
    pub(super) fn mount(
        &mut self,
        mount: impl FnOnce() -> Result<(), String>,
        cleanup: Cleanup,
    ) -> Result<(), String> {
        if self.stopped || self.cleanup.is_some() {
            return Err("native video session is stopped or already attached".into());
        }
        // Register rollback before touching mpv. Failed rollback must remain
        // tracked, including when the initial filter command partially succeeded.
        self.cleanup = Some(cleanup);
        if let Err(error) = mount() {
            return match self.detach() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}; rollback failed: {cleanup}")),
            };
        }
        Ok(())
    }

    pub(super) fn detach(&mut self) -> Result<(), String> {
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup()?;
            self.cleanup = None;
        }
        Ok(())
    }

    pub(super) fn stop(&mut self) -> Result<(), String> {
        self.stopped = true;
        self.detach()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn detach_is_idempotent_and_allows_reattach_but_stop_is_terminal() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut attachment = Attachment::default();
        for _ in 0..2 {
            let calls = calls.clone();
            attachment
                .mount(
                    || Ok(()),
                    Box::new(move || {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }),
                )
                .unwrap();
            assert!(
                attachment
                    .mount(|| panic!("duplicate mount"), Box::new(|| Ok(())))
                    .is_err()
            );
            attachment.detach().unwrap();
            attachment.detach().unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        attachment.stop().unwrap();
        assert!(
            attachment
                .mount(|| panic!("stale session"), Box::new(|| Ok(())))
                .is_err()
        );
    }

    #[test]
    fn failed_cleanup_keeps_lease_for_retry_and_blocks_replacement() {
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let mut attachment = Attachment::default();
        let result = attachment.mount(
            || Err("mount failed".into()),
            Box::new(move || {
                if count.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err("detach failed".into())
                } else {
                    Ok(())
                }
            }),
        );
        assert!(result.unwrap_err().contains("rollback failed"));
        assert!(
            attachment
                .mount(|| panic!("replacement"), Box::new(|| Ok(())))
                .is_err()
        );
        assert!(attachment.stop().is_err());
        attachment.stop().unwrap();
        attachment.stop().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
