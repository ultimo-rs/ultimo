//! End-to-end tests for `permessage-deflate` (RFC 7692).
//!
//! The client is hand-rolled on a raw TCP socket so every frame bit — RSV1,
//! masking, the stripped `00 00 ff ff` tail — is visible to the assertions.

#![cfg(feature = "websocket-compression")]

use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use ultimo::prelude::*;
use ultimo::websocket::{Message, WebSocket, WebSocketConfig, WebSocketHandler};

struct Echo;

#[async_trait::async_trait]
impl WebSocketHandler for Echo {
    type Data = ();

    async fn on_message(&self, ws: &WebSocket<()>, msg: Message) {
        match msg {
            Message::Text(t) => {
                ws.send(t).await.ok();
            }
            Message::Binary(b) => {
                ws.send_binary(b).await.ok();
            }
            _ => {}
        }
    }
}

async fn start(compression: bool, max_message_size: usize) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let mut app = Ultimo::new_without_defaults();
    let config = WebSocketConfig {
        compression,
        max_message_size,
        ping_interval: None,
        ..Default::default()
    };
    app.websocket_with_config("/ws", Echo, config);
    tokio::spawn(async move {
        app.listen(&format!("127.0.0.1:{port}")).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    port
}

/// A WebSocket client speaking raw frames.
struct Client {
    stream: TcpStream,
    buf: Vec<u8>,
    compress: Compress,
    decompress: Decompress,
}

/// A frame as the server sent it.
#[derive(Debug)]
struct RawFrame {
    fin: bool,
    rsv1: bool,
    opcode: u8,
    payload: Vec<u8>,
}

impl Client {
    /// Connect and handshake; returns the client and the response headers.
    async fn connect(port: u16, extensions: Option<&str>) -> (Self, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut req = format!(
            "GET /ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n"
        );
        if let Some(ext) = extensions {
            req.push_str(&format!("Sec-WebSocket-Extensions: {ext}\r\n"));
        }
        req.push_str("\r\n");
        stream.write_all(req.as_bytes()).await.unwrap();

        let mut buf = Vec::new();
        let head_end = loop {
            if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
            let mut chunk = [0u8; 1024];
            let n = stream.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed during handshake");
            buf.extend_from_slice(&chunk[..n]);
        };
        let head = String::from_utf8(buf[..head_end].to_vec()).unwrap();
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        buf.drain(..head_end);

        let client = Self {
            stream,
            buf,
            compress: Compress::new(Compression::default(), false),
            decompress: Decompress::new(false),
        };
        (client, head.to_ascii_lowercase())
    }

    async fn send_frame(&mut self, fin: bool, rsv1: bool, opcode: u8, payload: &[u8]) {
        let mask = [0x12, 0x34, 0x56, 0x78];
        let mut frame = vec![(fin as u8) << 7 | (rsv1 as u8) << 6 | opcode];
        let len = payload.len();
        if len < 126 {
            frame.push(0x80 | len as u8);
        } else if len <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        self.stream.write_all(&frame).await.unwrap();
    }

    fn deflate(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len() + 64);
        self.compress
            .compress_vec(data, &mut out, FlushCompress::Sync)
            .unwrap();
        assert!(out.ends_with(&[0, 0, 0xff, 0xff]));
        out.truncate(out.len() - 4);
        out
    }

    fn inflate(&mut self, data: &[u8]) -> Vec<u8> {
        let mut input = data.to_vec();
        input.extend_from_slice(&[0, 0, 0xff, 0xff]);
        let mut out = Vec::with_capacity(1 << 20);
        self.decompress
            .decompress_vec(&input, &mut out, FlushDecompress::Sync)
            .unwrap();
        out
    }

    async fn read_frame(&mut self) -> Option<RawFrame> {
        loop {
            if let Some((frame, used)) = parse(&self.buf) {
                self.buf.drain(..used);
                return Some(frame);
            }
            let mut chunk = [0u8; 65536];
            let n = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.stream.read(&mut chunk),
            )
            .await
            .expect("timed out waiting for a frame")
            .unwrap();
            if n == 0 {
                return None;
            }
            self.buf.extend_from_slice(&chunk[..n]);
        }
    }

    /// Read the next frame and expect a close with this code.
    async fn expect_close(&mut self, code: u16) {
        let frame = self.read_frame().await.expect("expected a close frame");
        assert_eq!(frame.opcode, 0x8, "{frame:?}");
        assert_eq!(
            u16::from_be_bytes([frame.payload[0], frame.payload[1]]),
            code
        );
    }
}

fn parse(buf: &[u8]) -> Option<(RawFrame, usize)> {
    if buf.len() < 2 {
        return None;
    }
    assert_eq!(buf[1] & 0x80, 0, "server frames must not be masked");
    let (len, mut pos) = match buf[1] & 0x7f {
        126 if buf.len() >= 4 => (u16::from_be_bytes([buf[2], buf[3]]) as usize, 4),
        127 if buf.len() >= 10 => (
            u64::from_be_bytes(buf[2..10].try_into().unwrap()) as usize,
            10,
        ),
        126 | 127 => return None,
        n => (n as usize, 2),
    };
    if buf.len() < pos + len {
        return None;
    }
    let frame = RawFrame {
        fin: buf[0] & 0x80 != 0,
        rsv1: buf[0] & 0x40 != 0,
        opcode: buf[0] & 0x0f,
        payload: buf[pos..pos + len].to_vec(),
    };
    pos += len;
    Some((frame, pos))
}

const JSON: &str = r#"{"type":"tick","symbol":"ACME","bid":101.25,"ask":101.27,"volume":1200}"#;

#[tokio::test]
async fn negotiates_when_enabled_and_offered() {
    let port = start(true, 1 << 20).await;
    let (_, head) = Client::connect(port, Some("permessage-deflate; client_max_window_bits")).await;
    assert!(
        head.contains("sec-websocket-extensions: permessage-deflate\r\n"),
        "{head}"
    );
}

#[tokio::test]
async fn does_not_negotiate_when_disabled() {
    let port = start(false, 1 << 20).await;
    let (_, head) = Client::connect(port, Some("permessage-deflate")).await;
    assert!(!head.contains("sec-websocket-extensions"), "{head}");
}

#[tokio::test]
async fn does_not_negotiate_when_not_offered() {
    let port = start(true, 1 << 20).await;
    let (_, head) = Client::connect(port, None).await;
    assert!(!head.contains("sec-websocket-extensions"), "{head}");
}

#[tokio::test]
async fn echoes_through_compression_both_ways() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;

    // Twice: the second message exercises the shared window on both sides.
    let mut sizes = Vec::new();
    for _ in 0..2 {
        let body = client.deflate(JSON.as_bytes());
        client.send_frame(true, true, 0x1, &body).await;

        let frame = client.read_frame().await.unwrap();
        assert!(
            frame.fin && frame.rsv1,
            "echo must come back compressed: {frame:?}"
        );
        assert_eq!(frame.opcode, 0x1);
        assert_eq!(client.inflate(&frame.payload), JSON.as_bytes());
        sizes.push(frame.payload.len());
    }
    // A short message alone barely shrinks; repeated, it refers back to the
    // first copy and nearly vanishes.
    assert!(
        sizes[1] < JSON.len() / 4,
        "{sizes:?} for {} bytes",
        JSON.len()
    );
}

#[tokio::test]
async fn small_messages_go_uncompressed() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;

    // An uncompressed frame is still valid on a compressed connection.
    client.send_frame(true, false, 0x1, b"hi").await;
    let frame = client.read_frame().await.unwrap();
    assert!(!frame.rsv1);
    assert_eq!(frame.payload, b"hi");
}

#[tokio::test]
async fn reassembles_a_fragmented_compressed_message() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;

    let body = client.deflate(JSON.as_bytes());
    let (a, b) = body.split_at(body.len() / 2);
    // RSV1 only on the first frame; the continuation carries none.
    client.send_frame(false, true, 0x1, a).await;
    client.send_frame(true, false, 0x0, b).await;

    let frame = client.read_frame().await.unwrap();
    assert_eq!(client.inflate(&frame.payload), JSON.as_bytes());
}

#[tokio::test]
async fn compressed_binary_round_trips() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;

    let data: Vec<u8> = (0..4096u32).map(|i| (i % 7) as u8).collect();
    let body = client.deflate(&data);
    client.send_frame(true, true, 0x2, &body).await;

    let frame = client.read_frame().await.unwrap();
    assert!(frame.rsv1);
    assert_eq!(frame.opcode, 0x2);
    assert_eq!(client.inflate(&frame.payload), data);
}

#[tokio::test]
async fn rsv1_without_negotiation_is_a_protocol_error() {
    let port = start(false, 1 << 20).await;
    let (mut client, _) = Client::connect(port, None).await;
    client
        .send_frame(true, true, 0x1, b"not really compressed")
        .await;
    client.expect_close(1002).await;
}

#[tokio::test]
async fn rsv1_on_a_control_frame_is_a_protocol_error() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;
    client.send_frame(true, true, 0x9, b"").await;
    client.expect_close(1002).await;
}

#[tokio::test]
async fn rsv1_on_a_continuation_frame_is_a_protocol_error() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;
    let body = client.deflate(JSON.as_bytes());
    client.send_frame(false, true, 0x1, &body).await;
    client.send_frame(true, true, 0x0, b"").await;
    client.expect_close(1002).await;
}

#[tokio::test]
async fn a_decompression_bomb_is_refused_with_1009() {
    // 64 KiB limit; the message inflates to 8 MiB but is only a few KiB on the wire.
    let port = start(true, 64 * 1024).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;

    let bomb = client.deflate(&vec![b'a'; 8 * 1024 * 1024]);
    assert!(bomb.len() < 64 * 1024, "{} bytes on the wire", bomb.len());
    client.send_frame(true, true, 0x2, &bomb).await;
    client.expect_close(1009).await;
}

#[tokio::test]
async fn invalid_deflate_data_closes_with_1007() {
    let port = start(true, 1 << 20).await;
    let (mut client, _) = Client::connect(port, Some("permessage-deflate")).await;
    client.send_frame(true, true, 0x1, &[0xff; 16]).await;
    client.expect_close(1007).await;
}
