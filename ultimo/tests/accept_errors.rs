//! A transient `accept()` failure must not stop the server.
//!
//! Running out of file descriptors (`EMFILE`) is the classic case: under load
//! `accept()` fails until some connection closes. The server has to log it,
//! back off, and keep accepting — not return and drop its listener.
//!
//! This lowers the process's open-file limit, so it lives in its own test
//! binary (one test) to keep it from starving unrelated tests.

#![cfg(unix)]

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use ultimo::prelude::*;

#[tokio::test]
async fn server_keeps_accepting_after_running_out_of_file_descriptors() {
    let mut app = Ultimo::new_without_defaults();
    app.get(
        "/",
        |ctx: Context| async move { ctx.text("still here").await },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(app.serve_with_shutdown(listener, std::future::pending()));

    // Cap the open-file limit so exhausting it is cheap, then use up every fd.
    let (soft, hard) = rlimit::Resource::NOFILE.get().unwrap();
    rlimit::Resource::NOFILE.set(soft.min(256), hard).unwrap();
    let mut fillers = Vec::new();
    while let Ok(f) = std::fs::File::open("/dev/null") {
        fillers.push(f);
    }
    // Free exactly one fd: enough for the client socket, none left for the
    // server's `accept()`, which now fails with EMFILE. (Whether that pending
    // connection survives is up to the OS — BSDs drop it — so it isn't checked.)
    fillers.pop();
    let _pending = tokio::net::TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let still_running = !server.is_finished();

    // Give the fds back: the server must still be accepting new connections.
    drop(fillers);
    rlimit::Resource::NOFILE.set(soft, hard).unwrap();
    assert!(
        still_running,
        "server stopped on a transient accept() error: {:?}",
        server.await
    );

    let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
    client
        .write_all(b"GET / HTTP/1.1\r\nhost: test\r\nconnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = String::new();
    // The server may be in its 1s back-off; allow for it.
    tokio::time::timeout(
        Duration::from_secs(10),
        client.read_to_string(&mut response),
    )
    .await
    .expect("server never answered after the fds were freed")
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with("still here"), "{response}");
}
