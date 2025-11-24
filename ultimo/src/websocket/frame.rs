//! WebSocket frame encoding/decoding (RFC 6455)

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::{self, ErrorKind};

/// WebSocket opcode (4 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Continue = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl OpCode {
    pub fn from_u8(byte: u8) -> Result<Self, io::Error> {
        match byte & 0x0F {
            0x0 => Ok(OpCode::Continue),
            0x1 => Ok(OpCode::Text),
            0x2 => Ok(OpCode::Binary),
            0x8 => Ok(OpCode::Close),
            0x9 => Ok(OpCode::Ping),
            0xA => Ok(OpCode::Pong),
            _ => Err(io::Error::new(ErrorKind::InvalidData, "invalid opcode")),
        }
    }
    
    pub fn is_control(&self) -> bool {
        matches!(self, OpCode::Close | OpCode::Ping | OpCode::Pong)
    }
}

/// WebSocket frame structure
#[derive(Debug, Clone)]
pub struct Frame {
    pub fin: bool,
    pub opcode: OpCode,
    pub mask: Option<[u8; 4]>,
    pub payload: Bytes,
}

impl Frame {
    /// Create a new text frame
    pub fn text(data: impl Into<String>) -> Self {
        Self {
            fin: true,
            opcode: OpCode::Text,
            mask: None,
            payload: Bytes::from(data.into().into_bytes()),
        }
    }
    
    /// Create a new binary frame
    pub fn binary(data: impl Into<Bytes>) -> Self {
        Self {
            fin: true,
            opcode: OpCode::Binary,
            mask: None,
            payload: data.into(),
        }
    }
    
    /// Create a close frame
    pub fn close(code: Option<u16>, reason: Option<&str>) -> Self {
        let mut payload = BytesMut::new();
        
        if let Some(code) = code {
            payload.put_u16(code);
            if let Some(reason) = reason {
                payload.put_slice(reason.as_bytes());
            }
        }
        
        Self {
            fin: true,
            opcode: OpCode::Close,
            mask: None,
            payload: payload.freeze(),
        }
    }
    
    /// Create a ping frame
    pub fn ping(data: impl Into<Bytes>) -> Self {
        Self {
            fin: true,
            opcode: OpCode::Ping,
            mask: None,
            payload: data.into(),
        }
    }
    
    /// Create a pong frame
    pub fn pong(data: impl Into<Bytes>) -> Self {
        Self {
            fin: true,
            opcode: OpCode::Pong,
            mask: None,
            payload: data.into(),
        }
    }
    
    /// Parse a frame from buffer
    pub fn parse(buf: &mut BytesMut) -> Result<Option<Self>, io::Error> {
        if buf.len() < 2 {
            return Ok(None); // Need at least 2 bytes
        }
        
        // First byte: FIN (1 bit) + RSV (3 bits) + OpCode (4 bits)
        let first = buf[0];
        let fin = (first & 0x80) != 0;
        let opcode = OpCode::from_u8(first)?;
        
        // Second byte: MASK (1 bit) + Payload length (7 bits)
        let second = buf[1];
        let masked = (second & 0x80) != 0;
        let mut payload_len = (second & 0x7F) as u64;
        
        let mut header_len = 2;
        
        // Extended payload length
        if payload_len == 126 {
            if buf.len() < 4 {
                return Ok(None); // Need 2 more bytes
            }
            payload_len = u16::from_be_bytes([buf[2], buf[3]]) as u64;
            header_len += 2;
        } else if payload_len == 127 {
            if buf.len() < 10 {
                return Ok(None); // Need 8 more bytes
            }
            payload_len = u64::from_be_bytes([
                buf[2], buf[3], buf[4], buf[5],
                buf[6], buf[7], buf[8], buf[9],
            ]);
            header_len += 8;
        }
        
        // Masking key (4 bytes if masked)
        let mask = if masked {
            if buf.len() < header_len + 4 {
                return Ok(None);
            }
            let mask_bytes = [
                buf[header_len],
                buf[header_len + 1],
                buf[header_len + 2],
                buf[header_len + 3],
            ];
            header_len += 4;
            Some(mask_bytes)
        } else {
            None
        };
        
        // Check if we have the full payload
        let total_len = header_len + payload_len as usize;
        if buf.len() < total_len {
            return Ok(None);
        }
        
        // Extract payload
        buf.advance(header_len);
        let mut payload = buf.split_to(payload_len as usize);
        
        // Unmask payload if needed
        if let Some(mask_key) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask_key[i % 4];
            }
        }
        
        Ok(Some(Frame {
            fin,
            opcode,
            mask,
            payload: payload.freeze(),
        }))
    }
    
    /// Encode frame to bytes
    pub fn encode(&self) -> Bytes {
        let payload_len = self.payload.len();
        let mut buf = BytesMut::new();
        
        // First byte: FIN + RSV + OpCode
        let mut first = self.opcode as u8;
        if self.fin {
            first |= 0x80;
        }
        buf.put_u8(first);
        
        // Second byte: MASK + Payload length
        let mut second = 0u8;
        if self.mask.is_some() {
            second |= 0x80;
        }
        
        // Payload length encoding
        if payload_len < 126 {
            second |= payload_len as u8;
            buf.put_u8(second);
        } else if payload_len <= u16::MAX as usize {
            second |= 126;
            buf.put_u8(second);
            buf.put_u16(payload_len as u16);
        } else {
            second |= 127;
            buf.put_u8(second);
            buf.put_u64(payload_len as u64);
        }
        
        // Masking key
        if let Some(mask_key) = self.mask {
            buf.put_slice(&mask_key);
        }
        
        // Payload (masked if needed)
        if let Some(mask_key) = self.mask {
            let mut masked = self.payload.to_vec();
            for (i, byte) in masked.iter_mut().enumerate() {
                *byte ^= mask_key[i % 4];
            }
            buf.put_slice(&masked);
        } else {
            buf.put_slice(&self.payload);
        }
        
        buf.freeze()
    }
}

/// High-level WebSocket message
#[derive(Debug, Clone)]
pub enum Message {
    Text(String),
    Binary(Bytes),
    Ping(Bytes),
    Pong(Bytes),
    Close(Option<CloseFrame>),
}

#[derive(Debug, Clone)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}

impl Message {
    /// Create message from frame
    pub fn from_frame(frame: Frame) -> Result<Self, io::Error> {
        match frame.opcode {
            OpCode::Text => {
                let text = String::from_utf8(frame.payload.to_vec())
                    .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid UTF-8"))?;
                Ok(Message::Text(text))
            }
            OpCode::Binary => Ok(Message::Binary(frame.payload)),
            OpCode::Ping => Ok(Message::Ping(frame.payload)),
            OpCode::Pong => Ok(Message::Pong(frame.payload)),
            OpCode::Close => {
                if frame.payload.len() >= 2 {
                    let mut buf = frame.payload.clone();
                    let code = buf.get_u16();
                    let reason = if buf.has_remaining() {
                        String::from_utf8(buf.to_vec())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };
                    Ok(Message::Close(Some(CloseFrame { code, reason })))
                } else {
                    Ok(Message::Close(None))
                }
            }
            OpCode::Continue => {
                Err(io::Error::new(ErrorKind::InvalidData, "unexpected continuation frame"))
            }
        }
    }
    
    /// Convert message to frame
    pub fn to_frame(&self) -> Frame {
        match self {
            Message::Text(text) => Frame::text(text.clone()),
            Message::Binary(data) => Frame::binary(data.clone()),
            Message::Ping(data) => Frame::ping(data.clone()),
            Message::Pong(data) => Frame::pong(data.clone()),
            Message::Close(close_frame) => {
                if let Some(cf) = close_frame {
                    Frame::close(Some(cf.code), Some(&cf.reason))
                } else {
                    Frame::close(None, None)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_text_encode_decode() {
        let frame = Frame::text("Hello, WebSocket!");
        let encoded = frame.encode();
        
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.fin, true);
        assert_eq!(decoded.opcode, OpCode::Text);
        assert_eq!(decoded.payload, Bytes::from("Hello, WebSocket!"));
    }
    
    #[test]
    fn test_frame_binary_encode_decode() {
        let data = vec![1, 2, 3, 4, 5];
        let frame = Frame::binary(Bytes::from(data.clone()));
        let encoded = frame.encode();
        
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.opcode, OpCode::Binary);
        assert_eq!(decoded.payload, Bytes::from(data));
    }
    
    #[test]
    fn test_frame_close_encode_decode() {
        let frame = Frame::close(Some(1000), Some("Normal closure"));
        let encoded = frame.encode();
        
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.opcode, OpCode::Close);
        assert!(decoded.payload.len() >= 2);
    }
    
    #[test]
    fn test_frame_masking() {
        let mut frame = Frame::text("Test");
        frame.mask = Some([1, 2, 3, 4]);
        
        let encoded = frame.encode();
        let mut buf = BytesMut::from(encoded.as_ref());
        let decoded = Frame::parse(&mut buf).unwrap().unwrap();
        
        assert_eq!(decoded.payload, Bytes::from("Test"));
    }
    
    #[test]
    fn test_message_from_frame() {
        let frame = Frame::text("Hello");
        let message = Message::from_frame(frame).unwrap();
        
        match message {
            Message::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected text message"),
        }
    }
}
