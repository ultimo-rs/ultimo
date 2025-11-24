//! WebSocket connection handling

use super::frame::{Frame, Message, OpCode};
use super::pubsub::ChannelManager;
use bytes::{Bytes, BytesMut};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use serde::Serialize;
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
}

impl<T> WebSocket<T> {
    /// Create a new WebSocket connection
    pub(crate) fn new(
        data: T,
        sender: mpsc::UnboundedSender<Message>,
        channel_manager: Arc<ChannelManager>,
        connection_id: uuid::Uuid,
        remote_addr: Option<SocketAddr>,
    ) -> Self {
        Self {
            data,
            sender,
            channel_manager,
            connection_id,
            remote_addr,
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
    pub async fn close(&self, code: Option<u16>, reason: Option<&str>) -> Result<(), std::io::Error> {
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
    sender: mpsc::UnboundedSender<Message>,
    receiver: mpsc::UnboundedReceiver<Message>,
    channel_manager: Arc<ChannelManager>,
    connection_id: uuid::Uuid,
}

impl ConnectionHandler {
    pub fn new(
        upgraded: Upgraded,
        channel_manager: Arc<ChannelManager>,
    ) -> (Self, mpsc::UnboundedSender<Message>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let connection_id = uuid::Uuid::new_v4();
        
        let handler = Self {
            upgraded,
            sender: tx.clone(),
            receiver: rx,
            channel_manager,
            connection_id,
        };
        
        (handler, tx)
    }
    
    pub async fn handle(self) -> Result<(), std::io::Error> {
        let mut read_buf = BytesMut::with_capacity(8192);
        let io = TokioIo::new(self.upgraded);
        let (mut reader, mut writer) = tokio::io::split(io);
        let mut receiver = self.receiver;
        let channel_manager = self.channel_manager;
        let connection_id = self.connection_id;
        
        loop {
            tokio::select! {
                // Read frames from client
                result = reader.read_buf(&mut read_buf) => {
                    match result {
                        Ok(0) => break, // Connection closed
                        Ok(_) => {
                            // Try to parse frames
                            while let Some(frame) = Frame::parse(&mut read_buf)? {
                                if let Err(e) = handle_frame(frame, &mut writer).await {
                                    tracing::error!("Error handling frame: {}", e);
                                    break;
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
                    let frame = message.to_frame();
                    let encoded = frame.encode();
                    
                    if let Err(e) = writer.write_all(&encoded).await {
                        tracing::error!("Error writing to socket: {}", e);
                        break;
                    }
                }
            }
        }
        
        // Cleanup on disconnect
        channel_manager.disconnect(connection_id).await;
        
        Ok(())
    }
}

async fn handle_frame(
    frame: Frame,
    writer: &mut tokio::io::WriteHalf<TokioIo<Upgraded>>,
) -> Result<(), std::io::Error> {
    match frame.opcode {
        OpCode::Text | OpCode::Binary => {
            // These are handled by the user's message handler
            // We just validate here
            Ok(())
        }
        OpCode::Close => {
            // Echo close frame back
            let close_frame = Frame::close(Some(1000), Some("Normal closure"));
            writer.write_all(&close_frame.encode()).await?;
            Ok(())
        }
        OpCode::Ping => {
            // Respond with pong
            let pong = Frame::pong(frame.payload);
            writer.write_all(&pong.encode()).await?;
            Ok(())
        }
        OpCode::Pong => {
            // Ignore pong frames
            Ok(())
        }
        OpCode::Continue => {
            // TODO: Handle fragmented messages
            Ok(())
        }
    }
}

