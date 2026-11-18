//! WebSocket connection handling

use super::deflate::PerMessageDeflate;
use super::frame::{Frame, Message, OpCode};
use super::pubsub::ChannelManager;
use super::WebSocketConfig;
use bytes::{Bytes, BytesMut};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use serde::Serialize;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time;

/// WebSocket connection with typed context data
pub struct WebSocket<T = ()> {
    data: T,
    sender: mpsc::Sender<Message>,
    channel_manager: Arc<ChannelManager>,
    connection_id: uuid::Uuid,
    remote_addr: Option<SocketAddr>,
    config: Arc<WebSocketConfig>,
}

impl<T> WebSocket<T> {
    /// Create a new WebSocket connection
    pub(crate) fn new(
        data: T,
        sender: mpsc::Sender<Message>,
        channel_manager: Arc<ChannelManager>,
        connection_id: uuid::Uuid,
        remote_addr: Option<SocketAddr>,
        config: Arc<WebSocketConfig>,
    ) -> Self {
        Self {
            data,
            sender,
            channel_manager,
            connection_id,
            remote_addr,
            config,
        }
    }

    /// Get reference to typed context data
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Get mutable reference to typed context data
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// Get reference to WebSocket configuration
    pub fn config(&self) -> &WebSocketConfig {
        &self.config
    }

    /// Send text message
    ///
    /// Returns `Err` if the connection is closed or the write buffer is full.
    /// When the buffer is full, consider waiting for `on_drain` callback before retrying.
    pub async fn send(&self, text: impl Into<String>) -> Result<(), std::io::Error> {
        self.sender
            .try_send(Message::Text(text.into()))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => {
                    std::io::Error::new(std::io::ErrorKind::WouldBlock, "write buffer full")
                }
                mpsc::error::TrySendError::Closed(_) => {
                    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed")
                }
            })
    }

    /// Send binary message
    ///
    /// Returns `Err` if the connection is closed or the write buffer is full.
    /// When the buffer is full, consider waiting for `on_drain` callback before retrying.
    pub async fn send_binary(&self, data: impl Into<Bytes>) -> Result<(), std::io::Error> {
        self.sender
            .try_send(Message::Binary(data.into()))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => {
                    std::io::Error::new(std::io::ErrorKind::WouldBlock, "write buffer full")
                }
                mpsc::error::TrySendError::Closed(_) => {
                    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed")
                }
            })
    }

    /// Send JSON message
    pub async fn send_json<S: Serialize>(&self, data: &S) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.send(json).await
    }

    /// Subscribe to a topic/channel
    pub async fn subscribe(&self, topic: impl AsRef<str>) -> Result<(), std::io::Error> {
        self.channel_manager
            .subscribe(self.connection_id, topic.as_ref(), self.sender.clone())
            .await
    }

    /// Unsubscribe from a topic/channel
    pub async fn unsubscribe(&self, topic: impl AsRef<str>) -> Result<(), std::io::Error> {
        self.channel_manager
            .unsubscribe(self.connection_id, topic.as_ref())
            .await
    }

    /// Publish message to all subscribers of a topic
    pub async fn publish<S: Serialize>(
        &self,
        topic: impl AsRef<str>,
        data: &S,
    ) -> Result<usize, std::io::Error> {
        let json = serde_json::to_string(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.channel_manager
            .publish(topic.as_ref(), Message::Text(json))
            .await
    }

    /// Close the WebSocket connection
    pub async fn close(
        &self,
        code: Option<u16>,
        reason: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let close_frame = if let Some(c) = code {
            let r = reason.unwrap_or("");
            let mut payload = BytesMut::new();
            payload.extend_from_slice(&c.to_be_bytes());
            payload.extend_from_slice(r.as_bytes());
            Message::Close(Some(super::frame::CloseFrame {
                code: c,
                reason: r.to_string(),
            }))
        } else {
            Message::Close(None)
        };

        self.sender.try_send(close_frame).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                std::io::Error::new(std::io::ErrorKind::WouldBlock, "write buffer full")
            }
            mpsc::error::TrySendError::Closed(_) => {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed")
            }
        })
    }

    /// Get remote address
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Check if connection is writable (for backpressure)
    pub fn is_writable(&self) -> bool {
        !self.sender.is_closed()
    }

    /// Get maximum capacity of the write queue
    pub fn max_capacity(&self) -> usize {
        self.sender.max_capacity()
    }

    /// Get current number of messages in the write queue
    pub fn capacity(&self) -> usize {
        self.sender.capacity()
    }

    /// Check if the write buffer has available capacity
    pub fn has_capacity(&self) -> bool {
        self.sender.capacity() > 0
    }
}

/// WebSocket connection handler that manages the connection lifecycle
pub(crate) struct ConnectionHandler {
    upgraded: Upgraded,
    receiver: mpsc::Receiver<Message>,
    incoming_tx: mpsc::UnboundedSender<Message>,
    drain_tx: mpsc::UnboundedSender<()>,
    channel_manager: Arc<ChannelManager>,
    connection_id: uuid::Uuid,
    config: Arc<WebSocketConfig>,
    deflate: Option<PerMessageDeflate>,
}

/// Fragment accumulator for reassembling fragmented messages
struct FragmentAccumulator {
    opcode: Option<OpCode>,
    fragments: BytesMut,
    total_size: usize,
    /// RSV1 was set on the first frame: the reassembled payload is compressed.
    compressed: bool,
}

/// Why the connection is being closed by the server mid-stream.
struct CloseReason {
    code: u16,
    reason: &'static str,
}

impl CloseReason {
    const PROTOCOL_ERROR: Self = Self {
        code: 1002,
        reason: "Protocol error",
    };

    fn for_inflate_error(e: &io::Error) -> Self {
        if e.kind() == ErrorKind::FileTooLarge {
            Self {
                code: 1009,
                reason: "Message too big",
            }
        } else {
            Self {
                code: 1007,
                reason: "Invalid compressed data",
            }
        }
    }
}

/// Encode an outgoing message as wire frames, compressing it when
/// `permessage-deflate` was negotiated and the message is large enough.
fn encode_outgoing(
    message: &Message,
    max_frame_size: usize,
    deflate: Option<&mut PerMessageDeflate>,
) -> io::Result<Vec<Bytes>> {
    let (opcode, payload): (OpCode, &[u8]) = match message {
        Message::Text(text) => (OpCode::Text, text.as_bytes()),
        Message::Binary(data) => (OpCode::Binary, data),
        _ => {
            return Ok(message
                .to_fragmented_frames(max_frame_size)
                .iter()
                .map(Frame::encode)
                .collect())
        }
    };
    let Some(deflate) = deflate.filter(|d| d.should_compress(payload.len())) else {
        return Ok(message
            .to_fragmented_frames(max_frame_size)
            .iter()
            .map(Frame::encode)
            .collect());
    };

    let compressed = Bytes::from(deflate.compress(payload)?);
    let chunk = max_frame_size.max(1);
    let mut frames = Vec::with_capacity(compressed.len() / chunk + 1);
    let mut offset = 0;
    loop {
        let end = (offset + chunk).min(compressed.len());
        let frame = Frame {
            fin: end == compressed.len(),
            opcode: if offset == 0 {
                opcode
            } else {
                OpCode::Continue
            },
            mask: None,
            payload: compressed.slice(offset..end),
        };
        // RSV1 marks the message as compressed, on its first frame only.
        frames.push(frame.encode_with_rsv1(offset == 0));
        if end == compressed.len() {
            break;
        }
        offset = end;
    }
    Ok(frames)
}

impl ConnectionHandler {
    pub fn new(
        upgraded: Upgraded,
        channel_manager: Arc<ChannelManager>,
        config: Arc<WebSocketConfig>,
        deflate: Option<PerMessageDeflate>,
    ) -> (
        Self,
        mpsc::Sender<Message>,
        mpsc::UnboundedReceiver<Message>,
        mpsc::UnboundedReceiver<()>,
    ) {
        // Use bounded channel for outgoing messages (backpressure)
        let (tx, rx) = mpsc::channel(config.max_write_queue_size);
        // Use unbounded channel for incoming messages (from client)
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        // Use unbounded channel for drain notifications
        let (drain_tx, drain_rx) = mpsc::unbounded_channel();
        let connection_id = uuid::Uuid::new_v4();

        let handler = Self {
            upgraded,
            receiver: rx,
            incoming_tx,
            drain_tx,
            channel_manager,
            connection_id,
            config,
            deflate,
        };

        (handler, tx, incoming_rx, drain_rx)
    }

    /// The id this handler cleans up on disconnect. The `WebSocket` handed to
    /// user code must use the same id, or `subscribe` registrations would
    /// never be removed by `disconnect`.
    pub(crate) fn connection_id(&self) -> uuid::Uuid {
        self.connection_id
    }

    pub async fn handle(self) -> Result<(), std::io::Error> {
        tracing::info!("ConnectionHandler::handle() started");
        let mut read_buf = BytesMut::with_capacity(8192);
        let io = TokioIo::new(self.upgraded);
        let (mut reader, mut writer) = tokio::io::split(io);
        let mut receiver = self.receiver;
        let channel_manager = self.channel_manager;
        let connection_id = self.connection_id;
        let incoming_tx = self.incoming_tx;
        let drain_tx = self.drain_tx;
        let config = self.config;
        let mut deflate = self.deflate;
        let mut fragment_accumulator: Option<FragmentAccumulator> = None;

        // Track write buffer state for backpressure
        let mut was_full = false;

        // Setup ping interval if configured
        let mut ping_interval = config
            .ping_interval
            .map(|interval_secs| time::interval(Duration::from_secs(interval_secs)));

        let mut last_pong_received = Instant::now();
        let ping_timeout = Duration::from_secs(config.ping_timeout);

        tracing::info!("Entering main WebSocket loop");
        'conn: loop {
            // Check for ping timeout
            if config.ping_interval.is_some() && last_pong_received.elapsed() > ping_timeout {
                tracing::warn!("Ping timeout - closing connection");
                let close_frame = Frame::close(Some(1001), Some("Ping timeout"));
                let _ = writer.write_all(&close_frame.encode()).await;
                break;
            }

            tokio::select! {
                // Ping interval
                _ = async {
                    match &mut ping_interval {
                        Some(interval) => interval.tick().await,
                        None => std::future::pending().await,
                    }
                } => {
                    // Send ping frame
                    let ping = Frame::ping(Bytes::new());
                    if let Err(e) = writer.write_all(&ping.encode()).await {
                        tracing::error!("Error sending ping: {}", e);
                        break;
                    }
                    tracing::trace!("Sent ping frame");
                }

                // Read frames from client
                result = reader.read_buf(&mut read_buf) => {
                    match result {
                        Ok(0) => break, // Connection closed
                        Ok(_) => {
                            // Try to parse frames with size limits
                            let mut close_with: Option<CloseReason> = None;
                            while let Some((frame, rsv)) = Frame::parse_with_rsv(&mut read_buf, Some(config.max_frame_size))? {
                                // RSV1 means "compressed" once permessage-deflate is
                                // negotiated, and only on the first frame of a data
                                // message. Any other RSV bit is a protocol error
                                // (RFC 6455 §5.2, RFC 7692 §6).
                                let compressed = rsv & 0b100 != 0;
                                if rsv & 0b011 != 0
                                    || (compressed
                                        && (deflate.is_none()
                                            || !matches!(frame.opcode, OpCode::Text | OpCode::Binary)))
                                {
                                    close_with = Some(CloseReason::PROTOCOL_ERROR);
                                    break;
                                }

                                match frame.opcode {
                                    OpCode::Text | OpCode::Binary => {
                                        if frame.fin {
                                            // Single unfragmented message
                                            let frame = if compressed {
                                                let codec = deflate.as_mut().expect("checked above");
                                                match codec.decompress(&frame.payload, config.max_message_size) {
                                                    Ok(payload) => Frame { payload: Bytes::from(payload), ..frame },
                                                    Err(e) => {
                                                        tracing::debug!("WebSocket inflate error: {}", e);
                                                        close_with = Some(CloseReason::for_inflate_error(&e));
                                                        break;
                                                    }
                                                }
                                            } else {
                                                frame
                                            };
                                            if let Ok(message) = Message::from_frame_with_limit(frame, Some(config.max_message_size)) {
                                                let _ = incoming_tx.send(message);
                                            }
                                        } else {
                                            // Start of fragmented message
                                            if fragment_accumulator.is_some() {
                                                return Err(io::Error::new(
                                                    ErrorKind::InvalidData,
                                                    "received new fragment before previous completed",
                                                ));
                                            }
                                            fragment_accumulator = Some(FragmentAccumulator {
                                                opcode: Some(frame.opcode),
                                                fragments: BytesMut::from(frame.payload.as_ref()),
                                                total_size: frame.payload.len(),
                                                compressed,
                                            });
                                        }
                                    }
                                    OpCode::Continue => {
                                        // Continuation frame
                                        let should_clear = if let Some(ref mut accumulator) = fragment_accumulator {
                                            accumulator.total_size += frame.payload.len();

                                            // Check message size limit
                                            if accumulator.total_size > config.max_message_size {
                                                return Err(io::Error::new(
                                                    ErrorKind::InvalidData,
                                                    format!("fragmented message size {} exceeds maximum {}",
                                                        accumulator.total_size, config.max_message_size),
                                                ));
                                            }

                                            accumulator.fragments.extend_from_slice(&frame.payload);
                                            frame.fin // Clear accumulator if this is the final fragment
                                        } else {
                                            return Err(io::Error::new(
                                                ErrorKind::InvalidData,
                                                "received continuation frame without initial fragment",
                                            ));
                                        };

                                        if should_clear {
                                            // Take ownership and reassemble
                                            if let Some(accumulator) = fragment_accumulator.take() {
                                                let payload = if accumulator.compressed {
                                                    let codec = deflate.as_mut().expect("compressed implies negotiated");
                                                    match codec.decompress(&accumulator.fragments, config.max_message_size) {
                                                        Ok(payload) => Bytes::from(payload),
                                                        Err(e) => {
                                                            tracing::debug!("WebSocket inflate error: {}", e);
                                                            close_with = Some(CloseReason::for_inflate_error(&e));
                                                            break;
                                                        }
                                                    }
                                                } else {
                                                    accumulator.fragments.freeze()
                                                };
                                                let reassembled_frame = Frame {
                                                    fin: true,
                                                    opcode: accumulator.opcode.unwrap(),
                                                    mask: None,
                                                    payload,
                                                };

                                                if let Ok(message) = Message::from_frame(reassembled_frame) {
                                                    let _ = incoming_tx.send(message);
                                                }
                                            }
                                        }
                                    }
                                    OpCode::Close => {
                                        // Send close frame to handler
                                        if let Ok(message) = Message::from_frame(frame) {
                                            let _ = incoming_tx.send(message);
                                        }
                                        // Echo close frame back
                                        let close_frame = Frame::close(Some(1000), Some("Normal closure"));
                                        let _ = writer.write_all(&close_frame.encode()).await;
                                        break;
                                    }
                                    OpCode::Ping => {
                                        // Respond with pong
                                        let pong = Frame::pong(frame.payload);
                                        let _ = writer.write_all(&pong.encode()).await;
                                    }
                                    OpCode::Pong => {
                                        // Update last pong received time
                                        last_pong_received = Instant::now();
                                        tracing::trace!("Received pong frame");
                                    }
                                }
                            }
                            if let Some(CloseReason { code, reason }) = close_with {
                                let close_frame = Frame::close(Some(code), Some(reason));
                                let _ = writer.write_all(&close_frame.encode()).await;
                                break 'conn;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error reading from socket: {}", e);
                            break;
                        }
                    }
                }

                // Send frames to client
                Some(message) = receiver.recv() => {
                    // Check if buffer was full before this recv
                    let is_now_draining = was_full;

                    // Compress if negotiated; fragment if it exceeds max frame size
                    let frames = match encode_outgoing(&message, config.max_frame_size, deflate.as_mut()) {
                        Ok(frames) => frames,
                        Err(e) => {
                            tracing::error!("Error encoding WebSocket message: {}", e);
                            break;
                        }
                    };

                    for encoded in frames {
                        if let Err(e) = writer.write_all(&encoded).await {
                            tracing::error!("Error writing to socket: {}", e);
                            break;
                        }
                    }

                    // Update full state - buffer is full if no capacity remains
                    let has_capacity = !receiver.is_empty() || receiver.is_closed();
                    was_full = !has_capacity;

                    // Notify on_drain if buffer was full and now has drained
                    if is_now_draining && has_capacity {
                        let _ = drain_tx.send(());
                        tracing::trace!("Write buffer drained, notified handler");
                    }
                }
            }
        }

        // Cleanup on disconnect
        channel_manager.disconnect(connection_id).await;

        Ok(())
    }
}
