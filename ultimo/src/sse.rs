//! Server-Sent Events (SSE) — typed server→client push over `text/event-stream`.
//!
//! Built on [`Context::stream`](crate::context::Context::stream): an [`SseEvent`]
//! encodes to the SSE wire format, and [`Context::sse`](crate::context::Context::sse)
//! streams a `Stream<Item = SseEvent>` to the client. See the [SSE guide](https://docs.ultimo.dev/sse).

use crate::error::Result;
use hyper::body::Bytes;
use serde::Serialize;
use std::time::Duration;

/// A single Server-Sent Event.
///
/// Construct with [`SseEvent::new`] (a typed `Serialize` payload → JSON in the
/// `data:` field), [`SseEvent::data`] (raw text), or [`SseEvent::comment`] (a
/// keep-alive comment line), then chain [`event`](SseEvent::event),
/// [`id`](SseEvent::id), and [`retry`](SseEvent::retry).
#[derive(Debug, Clone, Default)]
pub struct SseEvent {
    data: String,
    event: Option<String>,
    id: Option<String>,
    retry: Option<u64>,
    comment: Option<String>,
}

impl SseEvent {
    /// A typed event: `value` is serialized to JSON in the `data:` field.
    pub fn new<T: Serialize>(value: &T) -> Result<Self> {
        Ok(Self {
            data: serde_json::to_string(value)?,
            ..Default::default()
        })
    }

    /// An event whose `data:` field is the given raw text (used verbatim).
    pub fn data(text: impl Into<String>) -> Self {
        Self {
            data: text.into(),
            ..Default::default()
        }
    }

    /// A comment line (`: <text>`), typically a keep-alive `ping`.
    pub fn comment(text: impl Into<String>) -> Self {
        Self {
            comment: Some(text.into()),
            ..Default::default()
        }
    }

    /// Set the `event:` name.
    pub fn event(mut self, name: impl Into<String>) -> Self {
        self.event = Some(name.into());
        self
    }

    /// Set the `id:` field (surfaced to the client as `Last-Event-ID` on reconnect).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the `retry:` reconnection hint (emitted as integer milliseconds).
    pub fn retry(mut self, dur: Duration) -> Self {
        self.retry = Some(dur.as_millis() as u64);
        self
    }

    /// Encode to the SSE wire format (terminated by a blank line).
    pub fn encode(&self) -> Bytes {
        let mut out = String::new();
        if let Some(c) = &self.comment {
            for line in c.split('\n') {
                out.push_str(": ");
                out.push_str(line);
                out.push('\n');
            }
        }
        if let Some(e) = &self.event {
            out.push_str("event: ");
            out.push_str(e);
            out.push('\n');
        }
        if let Some(id) = &self.id {
            out.push_str("id: ");
            out.push_str(id);
            out.push('\n');
        }
        if let Some(r) = &self.retry {
            out.push_str("retry: ");
            out.push_str(&r.to_string());
            out.push('\n');
        }
        if !self.data.is_empty() {
            // The SSE spec requires one `data:` line per newline-separated segment.
            for line in self.data.split('\n') {
                out.push_str("data: ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push('\n');
        Bytes::from(out)
    }
}

/// The client disconnected: the SSE stream's receiver was dropped.
#[derive(Debug)]
pub struct SseClosed;

impl std::fmt::Display for SseClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SSE client disconnected")
    }
}

impl std::error::Error for SseClosed {}

/// Sends events into an SSE stream created by [`sse_channel`]. Cloneable and
/// `Send`, so it can be shared across tasks (broadcast, notifications).
#[derive(Clone)]
pub struct SseSender {
    tx: tokio::sync::mpsc::UnboundedSender<SseEvent>,
}

impl SseSender {
    /// Push an event to the client. Returns [`SseClosed`] if the client is gone.
    pub fn send(&self, event: SseEvent) -> std::result::Result<(), SseClosed> {
        self.tx.send(event).map_err(|_| SseClosed)
    }
}

/// Create an SSE `(sender, stream)` pair for the push/broadcast case.
///
/// Hand a [`SseSender`] to your producers and return the stream from
/// [`Context::sse`](crate::context::Context::sse). When every sender is dropped
/// the stream ends and the response closes.
pub fn sse_channel() -> (
    SseSender,
    impl futures_util::Stream<Item = SseEvent> + Send + 'static,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();
    let stream =
        futures_util::stream::unfold(
            rx,
            |mut rx| async move { rx.recv().await.map(|ev| (ev, rx)) },
        );
    (SseSender { tx }, stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s(ev: &SseEvent) -> String {
        String::from_utf8(ev.encode().to_vec()).unwrap()
    }

    #[test]
    fn typed_payload_encodes_data_line() {
        let ev = SseEvent::new(&json!({"n": 1})).unwrap();
        assert_eq!(s(&ev), "data: {\"n\":1}\n\n");
    }

    #[test]
    fn event_id_retry_lines_emitted() {
        let ev = SseEvent::data("hi")
            .event("message")
            .id("42")
            .retry(Duration::from_secs(3));
        assert_eq!(s(&ev), "event: message\nid: 42\nretry: 3000\ndata: hi\n\n");
    }

    #[test]
    fn multiline_data_splits_into_multiple_lines() {
        let ev = SseEvent::data("a\nb");
        assert_eq!(s(&ev), "data: a\ndata: b\n\n");
    }

    #[test]
    fn comment_encodes_as_colon_line() {
        assert_eq!(s(&SseEvent::comment("ping")), ": ping\n\n");
    }
}
