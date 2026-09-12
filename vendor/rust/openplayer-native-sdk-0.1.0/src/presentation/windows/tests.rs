use super::*;
use crate::presentation::tests::meta;
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn pair() -> (Producer, Consumer) {
    let producer = Producer::create(1024).unwrap();
    let consumer = Consumer::open(producer.endpoint()).unwrap();
    (producer, consumer)
}

#[test]
fn lease_protects_pixels_and_busy_does_not_queue() {
    let (mut producer, mut consumer) = pair();
    assert!(consumer.receive(0).unwrap().is_none());
    let bytes = vec![77; 1024];
    assert_eq!(
        producer.try_send(meta(), &bytes).unwrap(),
        SendOutcome::Sent
    );
    let next = FrameMeta {
        sequence: 2,
        ..meta()
    };
    assert_eq!(producer.try_send(next, &bytes).unwrap(), SendOutcome::Busy);
    let lease = consumer.receive(0).unwrap().unwrap();
    assert_eq!(lease.meta(), meta());
    assert_eq!(lease.pixels(), bytes);
    assert_eq!(
        producer.try_send(next, &vec![88; 1024]).unwrap(),
        SendOutcome::Busy
    );
    assert_eq!(lease.pixels(), bytes);
    drop(lease);
    assert_eq!(
        producer.try_send(next, &vec![88; 1024]).unwrap(),
        SendOutcome::Sent
    );
    assert_eq!(
        consumer.receive(0).unwrap().unwrap().pixels(),
        vec![88; 1024]
    );
    assert!(consumer.receive(0).unwrap().is_none());
}

#[test]
fn epoch_invalidates_pending_and_already_leased_frames() {
    let (mut producer, mut consumer) = pair();
    producer.try_send(meta(), &[0; 1024]).unwrap();
    assert_eq!(producer.control().advance_epoch(), 2);
    assert!(consumer.receive(0).unwrap().is_none());
    let next = FrameMeta {
        sequence: 2,
        ..meta()
    };
    assert_eq!(
        producer.try_send(next, &[0; 1024]).unwrap(),
        SendOutcome::Stale
    );
    let next = FrameMeta { epoch: 2, ..next };
    assert_eq!(
        producer.try_send(next, &[0; 1024]).unwrap(),
        SendOutcome::Sent
    );
    let lease = consumer.receive(0).unwrap().unwrap();
    assert!(lease.is_current());
    producer.control().advance_epoch();
    assert!(!lease.is_current());
}

#[test]
fn rejects_invalid_frames_without_consuming_the_free_slot() {
    let (mut producer, mut consumer) = pair();
    assert!(Producer::create(MAX_CAPACITY + 1).is_err());
    assert!(Producer::create(0).is_err());
    assert!(Consumer::open(producer.endpoint()).is_err());
    assert!(consumer.receive(1001).is_err());
    assert!(producer.try_send(meta(), &[0; 1023]).is_err());
    let oversized = FrameMeta {
        width: 32,
        stride: 128,
        ..meta()
    };
    assert!(producer.try_send(oversized, &[0; 2048]).is_err());
    assert_eq!(
        producer.try_send(meta(), &[0; 1024]).unwrap(),
        SendOutcome::Sent
    );
    drop(consumer.receive(0).unwrap().unwrap());
    assert!(producer.try_send(meta(), &[0; 1024]).is_err());
}

#[test]
fn close_wakes_waiter_and_last_drop_removes_objects() {
    let (producer, mut consumer) = pair();
    let endpoint = producer.endpoint().clone();
    let waiter = thread::spawn(move || {
        loop {
            match consumer.receive(1000) {
                Ok(None) => {}
                Err(error) => {
                    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
                    break;
                }
                Ok(Some(_)) => panic!("unexpected frame"),
            }
        }
    });
    drop(producer);
    waiter.join().unwrap();
    assert!(Consumer::open(&endpoint).is_err());
    let name = wide(&endpoint.mapping);
    let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr()) };
    assert!(handle.is_null());
}

#[test]
fn consumer_drop_closes_producer() {
    let (mut producer, consumer) = pair();
    drop(consumer);
    assert_eq!(
        producer.try_send(meta(), &[0; 1024]).unwrap(),
        SendOutcome::Closed
    );
}

#[test]
fn corrupt_header_is_fail_closed() {
    let (mut producer, mut consumer) = pair();
    producer.try_send(meta(), &[0; 1024]).unwrap();
    // Test-only protocol violation before consumer access: reserved byte nonzero.
    unsafe {
        producer
            .control
            .mapping
            .base()
            .add(GLOBAL_SIZE + 63)
            .write(1);
    }
    assert!(consumer.receive(0).is_err());
    assert!(producer.control().is_closed());
}

const CHILD_ENV: &str = "OPENPLAYER_PRESENTATION_TEST_ENDPOINT";

#[test]
fn transfers_frames_between_real_processes() {
    run_child(false);
}

#[test]
fn crashed_consumer_cannot_block_the_producer() {
    run_child(true);
}

fn run_child(crash: bool) {
    let mut producer = Producer::create(1024).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "presentation::windows::tests::child_process_receiver",
            "--ignored",
        ])
        .env(
            CHILD_ENV,
            serde_json::to_string(producer.endpoint()).unwrap(),
        )
        .env(
            "OPENPLAYER_PRESENTATION_TEST_CRASH",
            if crash { "1" } else { "0" },
        )
        .spawn()
        .unwrap();
    producer.try_send(meta(), &[123; 1024]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            if crash {
                assert_eq!(status.code(), Some(23));
                // The process died with a lease. The supervisor must close the
                // session; a stale free token must never be fabricated/reused.
                let next = FrameMeta {
                    sequence: 2,
                    ..meta()
                };
                assert_eq!(
                    producer.try_send(next, &[0; 1024]).unwrap(),
                    SendOutcome::Busy
                );
                producer.control().close();
                assert_eq!(
                    producer.try_send(next, &[0; 1024]).unwrap(),
                    SendOutcome::Closed
                );
            } else {
                assert!(status.success());
                assert!(producer.control().is_closed());
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("presentation child timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "launched by transfers_frames_between_real_processes"]
fn child_process_receiver() {
    let endpoint: Endpoint = serde_json::from_str(&std::env::var(CHILD_ENV).unwrap()).unwrap();
    assert_ne!(endpoint.producer_pid, std::process::id());
    let mut consumer = Consumer::open(&endpoint).unwrap();
    let frame = consumer.receive(1000).unwrap().expect("missing IPC frame");
    assert_eq!(frame.meta(), meta());
    assert_eq!(frame.pixels(), &[123; 1024]);
    assert!(frame.is_current());
    if std::env::var("OPENPLAYER_PRESENTATION_TEST_CRASH").as_deref() == Ok("1") {
        std::process::exit(23);
    }
}
