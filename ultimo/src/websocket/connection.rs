//! WebSocket connection handling

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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

/// WebSocket connection with typed context data
pub struct WebSocket<T = ()> {
    data: T,
    sender: mpsc::UnboundedSender<Message>,
    channel_manager: Arc<ChannelManager>,
    connection_id: uuid::Uuid,
    remote_addr: Option<SocketAddr>,
    config: Arc<WebSocketConfig>,
}

impl<T> WebSocket<T> {
    /// Create a new WebSocket connection
    pub(crate) fn new(
        data: T,
        sender: mpsc::UnboundedSender<Message>,
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
    pub async fn send(&self, text: impl Into<String>) -> Result<(), std::io::Error> {
        self.sender
            .send(Message::Text(text.into()))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed"))
    }

    /// Send binary message
    pub async fn send_binary(&self, data: impl Into<Bytes>) -> Result<(), std::io::Error> {
        self.sender
            .send(Message::Binary(data.into()))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed"))
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

        self.sender
            .send(close_frame)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "connection closed"))
    }

    /// Get remote address
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Check if connection is writable (for backpressure)
    pub fn is_writable(&self) -> bool {
        !self.sender.is_closed()
    }
}

/// WebSocket connection handler that manages the connection lifecycle
pub(crate) struct ConnectionHandler {
    upgraded: Upgraded,
    receiver: mpsc::UnboundedReceiver<Message>,
    incoming_tx: mpsc::UnboundedSender<Message>,
    channel_manager: Arc<ChannelManager>,
    connection_id: uuid::Uuid,
    config: Arc<WebSocketConfig>,
}

/// Fragment accumulator for reassembling fragmented messages
struct FragmentAccumulator {
    opcode: Option<OpCode>,
    fragments: BytesMut,
    total_size: usize,
}

impl ConnectionHandler {
    pub fn new(
        upgraded: Upgraded,
        channel_manager: Arc<ChannelManager>,
        config: Arc<WebSocketConfig>,
    ) -> (
        Self,
        mpsc::UnboundedSender<Message>,
        mpsc::UnboundedReceiver<Message>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let connection_id = uuid::Uuid::new_v4();

        let handler = Self {
            upgraded,
            receiver: rx,
            incoming_tx,
            channel_manager,
            connection_id,
            config,
        };

        (handler, tx, incoming_rx)
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
        let config = self.config;
        let mut fragment_accumulator: Option<FragmentAccumulator> = None;

        tracing::info!("Entering main WebSocket loop");
        loop {
            tokio::select! {
                // Read frames from client
                result = reader.read_buf(&mut read_buf) => {
                    match result {
                        Ok(0) => break, // Connection closed
                        Ok(_) => {
                            // Try to parse frames with size limits
                            while let Some(frame) = Frame::parse_with_limits(&mut read_buf, Some(config.max_frame_size))? {
                                match frame.opcode {
                                    OpCode::Text | OpCode::Binary => {
                                        if frame.fin {
                                            // Single unfragmented message
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
                                                let reassembled_frame = Frame {
                                                    fin: true,
                                                    opcode: accumulator.opcode.unwrap(),
                                                    mask: None,
                                                    payload: accumulator.fragments.freeze(),
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
                                        // Ignore pong frames
                                    }
                                }
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
                    // Use fragmentation if message exceeds max frame size
                    let frames = message.to_fragmented_frames(config.max_frame_size);

                    for frame in frames {
                        let encoded = frame.encode();
                        if let Err(e) = writer.write_all(&encoded).await {
                            tracing::error!("Error writing to socket: {}", e);
                            break;
                        }
                    }
                }
            }
        }

        // Cleanup on disconnect
        channel_manager.disconnect(connection_id).await;

        Ok(())
    }
}
