//! `permessage-deflate` (RFC 7692): negotiation and the per-connection codec.
//!
//! Compression is negotiated during the handshake and then applied per
//! message: a compressed message has RSV1 set on its first frame, and its
//! payload is a raw DEFLATE stream flushed with `Z_SYNC_FLUSH`, minus the
//! trailing `00 00 ff ff`.
//!
//! The codec lives behind the `websocket-compression` feature (it needs
//! `flate2`). Without it, [`PerMessageDeflate`] is an uninhabited type, so
//! the connection code compiles unchanged and every compression branch is
//! statically unreachable.

// Negotiation is only reachable with the `websocket-compression` feature.
#![cfg_attr(not(feature = "websocket-compression"), allow(dead_code))]

use hyper::header::{HeaderMap, HeaderValue, SEC_WEBSOCKET_EXTENSIONS};

/// Decide on compression for a handshake: returns the codec for the
/// connection and the `Sec-WebSocket-Extensions` response header, or `None`
/// to run uncompressed.
pub(crate) fn accept(
    enabled: bool,
    headers: &HeaderMap,
) -> Option<(PerMessageDeflate, HeaderValue)> {
    if !enabled {
        return None;
    }
    #[cfg(feature = "websocket-compression")]
    {
        negotiate(headers).map(|params| (PerMessageDeflate::new(params), params.response_header()))
    }
    #[cfg(not(feature = "websocket-compression"))]
    {
        let _ = headers;
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| {
            tracing::warn!(
                "WebSocketConfig::compression is set, but ultimo was built without the \
                 `websocket-compression` feature; connections stay uncompressed"
            );
        });
        None
    }
}

/// The extension parameters this server agreed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DeflateParams {
    /// The client asked us to reset our compressor after every message.
    pub server_no_context_takeover: bool,
}

impl DeflateParams {
    /// The `Sec-WebSocket-Extensions` value accepting these parameters.
    pub(crate) fn response_header(&self) -> HeaderValue {
        if self.server_no_context_takeover {
            HeaderValue::from_static("permessage-deflate; server_no_context_takeover")
        } else {
            HeaderValue::from_static("permessage-deflate")
        }
    }
}

/// Pick the first `permessage-deflate` offer in the request we can honor.
///
/// Offers are tried in the client's order of preference. An offer is
/// declined if it carries an unknown or duplicated parameter, an invalid
/// value, or `server_max_window_bits` below 15: the pure-Rust DEFLATE backend
/// always uses a 32 KiB window, so it can't promise a smaller one. Every
/// window size the client may use is fine for decompression, so
/// `client_max_window_bits` is accepted and left out of the response, which
/// leaves the client free to use any size.
pub(crate) fn negotiate(headers: &HeaderMap) -> Option<DeflateParams> {
    headers
        .get_all(SEC_WEBSOCKET_EXTENSIONS)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .find_map(parse_offer)
}

fn parse_offer(offer: &str) -> Option<DeflateParams> {
    let mut parts = offer.split(';').map(str::trim);
    if !parts.next()?.eq_ignore_ascii_case("permessage-deflate") {
        return None;
    }

    let mut params = DeflateParams::default();
    let mut seen = [false; 4];
    for part in parts {
        let (name, value) = match part.split_once('=') {
            Some((n, v)) => (n.trim(), Some(v.trim().trim_matches('"'))),
            None => (part, None),
        };
        let slot = match name.to_ascii_lowercase().as_str() {
            "server_no_context_takeover" if value.is_none() => {
                params.server_no_context_takeover = true;
                0
            }
            "client_no_context_takeover" if value.is_none() => 1,
            "server_max_window_bits" => {
                // We only ever use a 15-bit window; a smaller limit can't be met.
                if parse_window_bits(value?)? != 15 {
                    return None;
                }
                2
            }
            "client_max_window_bits" => {
                if let Some(v) = value {
                    parse_window_bits(v)?;
                }
                3
            }
            _ => return None,
        };
        if std::mem::replace(&mut seen[slot], true) {
            return None; // duplicated parameter
        }
    }
    Some(params)
}

fn parse_window_bits(value: &str) -> Option<u8> {
    // RFC 7692 §7.1.2: 1*DIGIT in 8..=15, no leading zeros.
    if value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok().filter(|bits| (8..=15).contains(bits))
}

pub(crate) use imp::PerMessageDeflate;

/// Messages smaller than this are sent uncompressed: the frame overhead of a
/// DEFLATE block outweighs any saving.
pub(crate) const MIN_COMPRESS_SIZE: usize = 32;

#[cfg(feature = "websocket-compression")]
mod imp {
    use super::{DeflateParams, MIN_COMPRESS_SIZE};
    use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};
    use std::io::{self, ErrorKind};

    /// The empty stored block a sync flush ends with, which RFC 7692 strips
    /// on send and restores on receive.
    const SYNC_TAIL: [u8; 4] = [0x00, 0x00, 0xff, 0xff];

    /// Output grows in steps of this size while inflating, so an oversized
    /// message is caught within one step of the limit.
    const INFLATE_STEP: usize = 32 * 1024;

    /// Per-connection compression state. The compressor and decompressor are
    /// created on first use: each holds a 32 KiB window plus working tables,
    /// so a connection that never sends (or never receives) a compressed
    /// message doesn't pay for them.
    pub(crate) struct PerMessageDeflate {
        params: DeflateParams,
        compressor: Option<Compress>,
        decompressor: Option<Decompress>,
    }

    impl std::fmt::Debug for PerMessageDeflate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PerMessageDeflate")
                .field("params", &self.params)
                .finish_non_exhaustive()
        }
    }

    impl PerMessageDeflate {
        pub(crate) fn new(params: DeflateParams) -> Self {
            Self {
                params,
                compressor: None,
                decompressor: None,
            }
        }

        /// Whether an outgoing payload of this size is worth compressing.
        pub(crate) fn should_compress(&self, len: usize) -> bool {
            len >= MIN_COMPRESS_SIZE
        }

        /// Compress one message payload.
        ///
        /// Once data is fed to the compressor it is part of the shared
        /// window, so the result must be sent compressed even if it came out
        /// larger; the decision is made up front by [`Self::should_compress`].
        pub(crate) fn compress(&mut self, payload: &[u8]) -> io::Result<Vec<u8>> {
            let compressor = self
                .compressor
                .get_or_insert_with(|| Compress::new(Compression::fast(), false));

            let mut out = Vec::with_capacity(payload.len() / 2 + 64);
            let mut consumed = 0;
            loop {
                let before = compressor.total_in();
                compressor
                    .compress_vec(&payload[consumed..], &mut out, FlushCompress::Sync)
                    .map_err(|e| io::Error::other(format!("deflate failed: {e}")))?;
                consumed += (compressor.total_in() - before) as usize;
                // The flush is complete once all input is in and the
                // compressor stopped with room to spare.
                if consumed == payload.len() && out.len() < out.capacity() {
                    break;
                }
                out.reserve(out.capacity().max(64));
            }

            if out.ends_with(&SYNC_TAIL) {
                out.truncate(out.len() - SYNC_TAIL.len());
            }
            if self.params.server_no_context_takeover {
                compressor.reset();
            }
            Ok(out)
        }

        /// Decompress one message payload, refusing to produce more than
        /// `max_size` bytes — a small compressed message can expand enormously.
        pub(crate) fn decompress(
            &mut self,
            payload: &[u8],
            max_size: usize,
        ) -> io::Result<Vec<u8>> {
            let decompressor = self
                .decompressor
                .get_or_insert_with(|| Decompress::new(false));

            let mut input = Vec::with_capacity(payload.len() + SYNC_TAIL.len());
            input.extend_from_slice(payload);
            input.extend_from_slice(&SYNC_TAIL);

            let mut out = Vec::with_capacity((payload.len() * 4).clamp(64, INFLATE_STEP));
            let mut consumed = 0;
            loop {
                let (before_in, before_out) = (decompressor.total_in(), decompressor.total_out());
                let status = decompressor
                    .decompress_vec(&input[consumed..], &mut out, FlushDecompress::Sync)
                    .map_err(|e| {
                        io::Error::new(ErrorKind::InvalidData, format!("invalid deflate data: {e}"))
                    })?;
                consumed += (decompressor.total_in() - before_in) as usize;
                let progressed =
                    decompressor.total_in() != before_in || decompressor.total_out() != before_out;

                if out.len() > max_size {
                    return Err(too_big(max_size));
                }
                if status == Status::StreamEnd {
                    // A final block ends the DEFLATE stream; start a fresh one
                    // for the next message.
                    decompressor.reset(false);
                    break;
                }
                if consumed == input.len() && out.len() < out.capacity() {
                    break;
                }
                if out.len() == out.capacity() {
                    if out.len() >= max_size {
                        return Err(too_big(max_size));
                    }
                    out.reserve(INFLATE_STEP);
                } else if !progressed {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "truncated deflate data",
                    ));
                }
            }
            Ok(out)
        }
    }

    /// `FileTooLarge` lets the connection answer with close code 1009
    /// (message too big) rather than 1007 (invalid data).
    fn too_big(max_size: usize) -> io::Error {
        io::Error::new(
            ErrorKind::FileTooLarge,
            format!("decompressed message exceeds maximum {max_size}"),
        )
    }
}

#[cfg(not(feature = "websocket-compression"))]
mod imp {
    use std::convert::Infallible;
    use std::io;

    /// Without the `websocket-compression` feature compression is never
    /// negotiated, so this type has no values and its methods can't be called.
    #[derive(Debug)]
    pub(crate) struct PerMessageDeflate(Infallible);

    impl PerMessageDeflate {
        pub(crate) fn should_compress(&self, _: usize) -> bool {
            match self.0 {}
        }

        pub(crate) fn compress(&mut self, _: &[u8]) -> io::Result<Vec<u8>> {
            match self.0 {}
        }

        pub(crate) fn decompress(&mut self, _: &[u8], _: usize) -> io::Result<Vec<u8>> {
            match self.0 {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(value: &str) -> Option<DeflateParams> {
        let mut headers = HeaderMap::new();
        headers.insert(SEC_WEBSOCKET_EXTENSIONS, value.parse().unwrap());
        negotiate(&headers)
    }

    #[test]
    fn accepts_a_plain_offer() {
        assert_eq!(offer("permessage-deflate"), Some(DeflateParams::default()));
    }

    #[test]
    fn accepts_what_browsers_send() {
        // Chrome, Firefox and Safari all send this.
        assert_eq!(
            offer("permessage-deflate; client_max_window_bits"),
            Some(DeflateParams::default())
        );
    }

    #[test]
    fn honors_server_no_context_takeover() {
        let params = offer("permessage-deflate; server_no_context_takeover").unwrap();
        assert!(params.server_no_context_takeover);
        assert_eq!(
            params.response_header(),
            "permessage-deflate; server_no_context_takeover"
        );
    }

    #[test]
    fn declines_a_smaller_server_window_but_takes_the_next_offer() {
        assert_eq!(offer("permessage-deflate; server_max_window_bits=10"), None);
        assert_eq!(
            offer("permessage-deflate; server_max_window_bits=10, permessage-deflate"),
            Some(DeflateParams::default())
        );
        assert_eq!(
            offer("permessage-deflate; server_max_window_bits=15"),
            Some(DeflateParams::default())
        );
    }

    #[test]
    fn accepts_quoted_client_window_bits() {
        assert!(offer("permessage-deflate; client_max_window_bits=\"12\"").is_some());
    }

    #[test]
    fn declines_invalid_offers() {
        for bad in [
            "permessage-deflate; unknown_param",
            "permessage-deflate; server_no_context_takeover; server_no_context_takeover",
            "permessage-deflate; client_max_window_bits=7",
            "permessage-deflate; client_max_window_bits=16",
            "permessage-deflate; client_max_window_bits=09",
            "permessage-deflate; server_no_context_takeover=1",
            "x-webkit-deflate-frame",
        ] {
            assert_eq!(offer(bad), None, "{bad}");
        }
    }

    #[test]
    fn reads_offers_across_header_lines() {
        let mut headers = HeaderMap::new();
        headers.append(SEC_WEBSOCKET_EXTENSIONS, "x-foo".parse().unwrap());
        headers.append(
            SEC_WEBSOCKET_EXTENSIONS,
            "permessage-deflate".parse().unwrap(),
        );
        assert!(negotiate(&headers).is_some());
    }

    #[cfg(feature = "websocket-compression")]
    mod codec {
        use super::super::*;

        fn pair(params: DeflateParams) -> (PerMessageDeflate, PerMessageDeflate) {
            (
                PerMessageDeflate::new(params),
                PerMessageDeflate::new(params),
            )
        }

        #[test]
        fn round_trips_messages_with_context_takeover() {
            let (mut tx, mut rx) = pair(DeflateParams::default());
            let msg = br#"{"type":"price","symbol":"ACME","bid":101.25,"ask":101.27}"#;
            let first = tx.compress(msg).unwrap();
            let second = tx.compress(msg).unwrap();
            // The second copy compresses to almost nothing: it refers back to the first.
            assert!(
                second.len() < first.len() / 2,
                "{} vs {}",
                second.len(),
                first.len()
            );
            assert!(!first.ends_with(&[0, 0, 0xff, 0xff]));

            assert_eq!(rx.decompress(&first, 1 << 20).unwrap(), msg);
            assert_eq!(rx.decompress(&second, 1 << 20).unwrap(), msg);
        }

        #[test]
        fn resets_between_messages_without_context_takeover() {
            let params = DeflateParams {
                server_no_context_takeover: true,
            };
            let mut tx = PerMessageDeflate::new(params);
            let msg = b"hello hello hello hello hello hello hello";
            let first = tx.compress(msg).unwrap();
            let second = tx.compress(msg).unwrap();
            assert_eq!(first, second, "each message must stand alone");

            // A fresh decompressor (no shared history) can read the second one.
            let mut rx = PerMessageDeflate::new(params);
            assert_eq!(rx.decompress(&second, 1 << 20).unwrap(), msg);
        }

        #[test]
        fn round_trips_large_and_empty_payloads() {
            let (mut tx, mut rx) = pair(DeflateParams::default());
            let big: Vec<u8> = (0..500_000u32)
                .flat_map(|i| (i % 251).to_le_bytes())
                .collect();
            let c = tx.compress(&big).unwrap();
            assert_eq!(rx.decompress(&c, big.len()).unwrap(), big);

            let c = tx.compress(b"").unwrap();
            assert_eq!(rx.decompress(&c, 10).unwrap(), b"");
        }

        #[test]
        fn refuses_a_decompression_bomb() {
            let (mut tx, mut rx) = pair(DeflateParams::default());
            let bomb = vec![0u8; 10 * 1024 * 1024];
            let c = tx.compress(&bomb).unwrap();
            assert!(c.len() < 64 * 1024, "compressed to {} bytes", c.len());

            let err = rx.decompress(&c, 1024 * 1024).unwrap_err();
            assert!(err.to_string().contains("exceeds maximum"), "{err}");
        }

        #[test]
        fn exact_limit_is_allowed() {
            let (mut tx, mut rx) = pair(DeflateParams::default());
            let msg = vec![b'a'; 100_000];
            let c = tx.compress(&msg).unwrap();
            assert_eq!(rx.decompress(&c, 100_000).unwrap().len(), 100_000);
            let c = tx.compress(&msg).unwrap();
            assert!(rx.decompress(&c, 99_999).is_err());
        }

        #[test]
        fn rejects_garbage() {
            let mut rx = PerMessageDeflate::new(DeflateParams::default());
            assert!(rx.decompress(&[0xff, 0xff, 0xff, 0xff], 1024).is_err());
        }

        #[test]
        fn small_messages_are_not_compressed() {
            let tx = PerMessageDeflate::new(DeflateParams::default());
            assert!(!tx.should_compress(MIN_COMPRESS_SIZE - 1));
            assert!(tx.should_compress(MIN_COMPRESS_SIZE));
        }
    }
}
