//! [LSP-LIFECYCLE] The endings the serve loop must recognise.
//!
//! The integration pin lives in `tests/lifecycle.rs`, which drives the real
//! binary. It caught this defect only on a slow instrumented runner, where
//! the analysis pass outlived the client. These isolate the mechanism so the
//! contract does not depend on how fast the machine underneath it is.

use std::{future::pending, sync::Arc};

use tokio::{
    io::AsyncReadExt,
    sync::Notify,
    time::{timeout, Duration},
};

use super::{EndOnEof, ServeEnd};

/// Long enough that reaching it means "never", short enough to fail fast.
/// Nothing here waits on real work, so a correct implementation resolves
/// immediately and never spends this.
const NEVER: Duration = Duration::from_secs(5);

#[tokio::test]
async fn a_reader_at_end_of_file_raises_the_end_signal() {
    let ended = Arc::new(Notify::new());
    let mut stdin = EndOnEof {
        inner: &[][..],
        ended: Arc::clone(&ended),
    };

    let read = stdin.read_u8().await;

    assert!(read.is_err(), "an empty reader must report end of file");
    assert!(
        timeout(NEVER, ended.notified()).await.is_ok(),
        "a client that closed stdin has gone, and the serve loop must be told \
         so rather than waiting on the analysis pass it will never collect"
    );
}

#[tokio::test]
async fn bytes_before_end_of_file_raise_nothing() {
    const TRAFFIC: &[u8] = b"Content-Length: 2\r\n\r\n{}";
    let ended = Arc::new(Notify::new());
    let mut stdin = EndOnEof {
        inner: TRAFFIC,
        ended: Arc::clone(&ended),
    };
    let mut seen = Vec::new();

    let read = stdin.read_buf(&mut seen).await;

    assert_eq!(
        read.ok(),
        Some(TRAFFIC.len()),
        "every byte offered must be read"
    );
    assert_eq!(seen, TRAFFIC, "the wrapper must not alter the traffic");
    assert!(
        timeout(NEVER, ended.notified()).await.is_err(),
        "a client that is still sending has not gone"
    );
}

#[tokio::test]
async fn the_signal_ends_the_loop_while_serving_is_still_running() {
    let exited = Notify::new();
    exited.notify_one();

    let ended = timeout(NEVER, super::serve_until_exit(pending(), &exited)).await;

    assert!(
        ended.is_ok(),
        "the loop must end on the signal alone — waiting for a pass with no \
         client left to read it is the leak this contract forbids"
    );
}

#[test]
fn an_ending_with_no_exit_notification_is_a_vanished_client() {
    let lifecycle = super::Lifecycle::default();

    assert_eq!(
        lifecycle.end(),
        ServeEnd::ClientVanished,
        "closing stdin is not an `exit`, and must not be reported as one"
    );
}
