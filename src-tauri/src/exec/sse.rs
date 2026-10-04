//! Server-Sent Events: an HTTP request whose response never really ends.
//!
//! The request that opens a stream is an ordinary HTTP request - same
//! method, URL, headers, auth and body - so it is resolved exactly like one
//! (`resolve_http_request`). What differs is the response: instead of one
//! outcome at the end there is a sequence of events for as long as the
//! connection stays open, which is why this is an executor of its own beside
//! `ProtocolExecutor` rather than an implementation of it.

use super::http::{describe_error, header_pairs, host_of, method_to_reqwest, ClientCache};
use super::{default_header, ExecutionContext, ExecutionTrace, ExecutorError, Phase, ResolvedHttpRequest, TraceRecorder};
use crate::domain::{optional_duration, KeyValue, RequestSettings};
use crate::i18n::messages;
use base64::Engine;
use serde::Serialize;
use std::future::Future;
use std::time::Instant;

/// What the stream sends to the UI, in order. The executor emits everything
/// but `End`; the command sends `End` last, once the outcome and the trace
/// are both known.
///
/// `End` travels on the same channel as the events instead of being the
/// command's return value: messages on one channel arrive in order, but the
/// command's result and the channel are separate routes, and the result
/// could otherwise overtake the last few events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SseMessage {
    /// The response head arrived.
    Open {
        status: u16,
        status_text: String,
        headers: Vec<KeyValue>,
    },
    Event {
        /// How far into the request it arrived, on the same clock as the
        /// trace log.
        at_ms: u64,
        /// The `id` this event carried itself, if any. (The specification's
        /// "last event id" persists across events; for reading a stream,
        /// what each event actually said is the more useful thing to show.)
        id: Option<String>,
        /// `message` when the event named no type, as `EventSource` does.
        event: String,
        data: String,
        retry_ms: Option<u64>,
    },
    /// A `:` line. A client ignores these; they are shown anyway, because a
    /// heartbeat that stops arriving is often what is being debugged.
    Comment { at_ms: u64, text: String },
    End {
        outcome: SseOutcome,
        trace: ExecutionTrace,
        unresolved_variables: Vec<String>,
    },
}

/// Why the stream is over.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEnd {
    /// The server finished the response.
    Closed,
    /// The user disconnected - the normal way a stream ends, not a failure.
    Cancelled,
    /// The connection broke after the stream had started. The events that
    /// came before it are still valid and stay on screen.
    Failed { message: String },
    /// The server answered with something other than an event stream (an
    /// error status, or another `Content-Type`). It was read whole and is
    /// handed over as the ordinary response it is.
    NotAStream { body_base64: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct SseOutcome {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    /// Dispatched events, comments not counted.
    pub events: u64,
    pub end: StreamEnd,
}

/// One unit the parser produced.
#[derive(Debug, Clone, PartialEq)]
pub enum SseFrame {
    Event {
        id: Option<String>,
        event: String,
        data: String,
        retry_ms: Option<u64>,
    },
    Comment(String),
}

/// The `text/event-stream` parser from the HTML specification ("Event stream
/// interpretation"), fed chunk by chunk.
///
/// It works on bytes and only decodes complete lines: a chunk can end in the
/// middle of a UTF-8 sequence, or between the CR and LF of one line break,
/// and neither may turn into garbage or an extra blank line.
#[derive(Debug, Default)]
pub struct SseParser {
    /// The unfinished line so far.
    line: Vec<u8>,
    /// The previous byte was a CR, so an LF right after it is part of the
    /// same line break.
    after_cr: bool,
    /// Whether a line has been completed yet - only the very first one may
    /// start with a byte order mark.
    seen_line: bool,
    data: String,
    event: String,
    id: Option<String>,
    retry_ms: Option<u64>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseFrame> {
        let mut frames = Vec::new();
        for &byte in chunk {
            if std::mem::take(&mut self.after_cr) && byte == b'\n' {
                continue;
            }
            match byte {
                b'\r' => {
                    self.after_cr = true;
                    self.end_line(&mut frames);
                }
                b'\n' => self.end_line(&mut frames),
                _ => self.line.push(byte),
            }
        }
        frames
    }

    /// Whether the stream stopped part way through an event - the
    /// specification drops it, but the caller may want to say so.
    pub fn has_incomplete_event(&self) -> bool {
        !self.line.is_empty() || !self.data.is_empty()
    }

    fn end_line(&mut self, frames: &mut Vec<SseFrame>) {
        let bytes = std::mem::take(&mut self.line);
        let mut line = String::from_utf8_lossy(&bytes).into_owned();
        if !self.seen_line {
            self.seen_line = true;
            if let Some(rest) = line.strip_prefix('\u{FEFF}') {
                line = rest.to_string();
            }
        }

        if line.is_empty() {
            self.dispatch(frames);
            return;
        }
        if let Some(comment) = line.strip_prefix(':') {
            frames.push(SseFrame::Comment(strip_one_space(comment).to_string()));
            return;
        }
        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, strip_one_space(value)),
            None => (line.as_str(), ""),
        };
        match field {
            "event" => self.event = value.to_string(),
            "data" => {
                self.data.push_str(value);
                self.data.push('\n');
            }
            // A NUL makes the specification ignore the field outright.
            "id" if !value.contains('\0') => self.id = Some(value.to_string()),
            "retry" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                self.retry_ms = value.parse().ok();
            }
            _ => {}
        }
    }

    fn dispatch(&mut self, frames: &mut Vec<SseFrame>) {
        let event = std::mem::take(&mut self.event);
        let id = self.id.take();
        let retry_ms = self.retry_ms.take();
        // No data line means no event, whatever else the block set.
        if self.data.is_empty() {
            return;
        }
        let mut data = std::mem::take(&mut self.data);
        data.pop();
        frames.push(SseFrame::Event {
            id,
            event: if event.is_empty() { "message".to_string() } else { event },
            data,
            retry_ms,
        });
    }
}

fn strip_one_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn is_event_stream(headers: &reqwest::header::HeaderMap) -> bool {
    headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("text/event-stream"))
}

/// The settings a stream's client is built with. Both the read and the total
/// timeout are switched off: a stream that stays quiet for a minute between
/// events is working as intended, and one that lasts an hour is the point.
/// The read timeout still bounds the wait for the response head - see
/// `SseExecutor::stream`.
fn stream_settings(settings: &RequestSettings) -> RequestSettings {
    RequestSettings {
        read_timeout_ms: 0,
        total_timeout_ms: 0,
        ..settings.clone()
    }
}

pub struct SseExecutor {
    /// Separate from the HTTP executor's: the two are built with different
    /// timeouts, and sharing one cache would rebuild the client every time
    /// the user alternated between a request and a stream.
    clients: ClientCache,
}

impl SseExecutor {
    pub fn new() -> Self {
        SseExecutor {
            clients: ClientCache::new(),
        }
    }

    /// Opens the stream and reads it until the server closes it, the
    /// connection breaks, or `cancelled` resolves. Events go out through
    /// `emit` as they are parsed.
    ///
    /// Cancellation is passed in rather than done by dropping this future:
    /// once the stream has started, stopping it is the ordinary end, and the
    /// outcome (status, headers, how many events) still has to come back.
    /// Before the head arrives there is nothing to report, and cancelling
    /// fails the attempt like a cancelled HTTP send.
    pub async fn stream(
        &self,
        request: &ResolvedHttpRequest,
        ctx: &ExecutionContext,
        trace: &mut TraceRecorder,
        cancelled: impl Future<Output = ()> + Send,
        emit: &mut (dyn FnMut(SseMessage) + Send),
    ) -> Result<SseOutcome, ExecutorError> {
        tokio::pin!(cancelled);
        let client = self.clients.client_for(&stream_settings(&ctx.settings))?;
        let method = method_to_reqwest(request.method);

        // What `EventSource` sends; the user's own headers win.
        let mut headers = request.headers.clone();
        default_header(&mut headers, "Accept", "text/event-stream");
        default_header(&mut headers, "Cache-Control", "no-cache");

        let mut builder = client.request(method.clone(), &request.url);
        for header in headers.iter().filter(|h| h.enabled) {
            builder = builder.header(&header.key, &header.value);
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        trace.info(format!("{} {}", method, request.url));
        if !ctx.settings.verify_tls {
            trace.warn(messages::tls_verification_disabled());
        }

        let fail = |trace: &mut TraceRecorder, message: String| {
            trace.error(message.clone());
            ExecutorError::Failed(message)
        };

        // The read timeout means "silence for this long is a failure". Between
        // events silence is normal, but waiting for the head is still waiting
        // for an answer, so the setting applies there.
        let sent_at = Instant::now();
        let head = async {
            match optional_duration(ctx.settings.read_timeout_ms) {
                Some(limit) => tokio::time::timeout(limit, builder.send()).await.ok(),
                None => Some(builder.send().await),
            }
        };
        let head = tokio::select! {
            head = head => head,
            _ = &mut cancelled => {
                trace.phase(Phase::Wait, sent_at.elapsed());
                return Err(fail(trace, messages::request_cancelled()));
            }
        };
        trace.phase(Phase::Wait, sent_at.elapsed());
        let mut response = match head {
            Some(Ok(response)) => response,
            Some(Err(e)) => return Err(fail(trace, describe_error(&e, &request.url))),
            None => return Err(fail(trace, messages::response_timeout(&host_of(&request.url)))),
        };

        let status = response.status();
        let status_text = status.canonical_reason().unwrap_or("").to_string();
        let headers = header_pairs(response.headers());
        if let Some(addr) = response.remote_addr() {
            trace.set_remote_addr(addr.to_string());
        }
        trace.info(messages::response_received(
            status.as_u16(),
            &status_text,
            &format!("{:?}", response.version()),
        ));
        emit(SseMessage::Open {
            status: status.as_u16(),
            status_text: status_text.clone(),
            headers: headers.clone(),
        });
        let outcome = |events: u64, end: StreamEnd| SseOutcome {
            status: status.as_u16(),
            status_text: status_text.clone(),
            headers: headers.clone(),
            events,
            end,
        };

        let download_at = Instant::now();

        // `EventSource` refuses anything but a successful `text/event-stream`.
        // What came instead is usually the explanation (a 401 with a JSON
        // error, an HTML login page), so it is read and handed over whole.
        if !status.is_success() || !is_event_stream(response.headers()) {
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-")
                .to_string();
            trace.warn(messages::stream_not_an_event_stream(status.as_u16(), &content_type));
            let body = tokio::select! {
                body = response.bytes() => body,
                _ = &mut cancelled => return Ok(outcome(0, StreamEnd::Cancelled)),
            };
            trace.phase(Phase::Download, download_at.elapsed());
            let body = body.map_err(|e| fail(trace, describe_error(&e, &request.url)))?;
            trace.info(messages::body_received(body.len()));
            return Ok(outcome(
                0,
                StreamEnd::NotAStream {
                    body_base64: base64::engine::general_purpose::STANDARD.encode(&body),
                },
            ));
        }

        trace.info(messages::stream_opened());
        let mut parser = SseParser::new();
        let mut events = 0u64;
        let end = loop {
            let chunk = tokio::select! {
                chunk = response.chunk() => chunk,
                _ = &mut cancelled => break StreamEnd::Cancelled,
            };
            let bytes = match chunk {
                Ok(Some(bytes)) => bytes,
                Ok(None) => break StreamEnd::Closed,
                Err(e) => {
                    let message = describe_error(&e, &request.url);
                    trace.error(message.clone());
                    break StreamEnd::Failed { message };
                }
            };
            let at_ms = trace.elapsed().as_millis() as u64;
            for frame in parser.feed(&bytes) {
                emit(match frame {
                    SseFrame::Event {
                        id,
                        event,
                        data,
                        retry_ms,
                    } => {
                        events += 1;
                        SseMessage::Event {
                            at_ms,
                            id,
                            event,
                            data,
                            retry_ms,
                        }
                    }
                    SseFrame::Comment(text) => SseMessage::Comment { at_ms, text },
                });
            }
        };
        // For a stream "download" is how long it stayed open.
        trace.phase(Phase::Download, download_at.elapsed());

        match &end {
            StreamEnd::Closed => {
                if parser.has_incomplete_event() {
                    trace.warn(messages::stream_incomplete_event());
                }
                trace.info(messages::stream_closed_by_server(events));
            }
            StreamEnd::Cancelled => trace.info(messages::stream_closed_by_client(events)),
            StreamEnd::Failed { .. } | StreamEnd::NotAStream { .. } => {}
        }
        Ok(outcome(events, end))
    }
}

impl Default for SseExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HttpMethod;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn event(id: Option<&str>, event: &str, data: &str) -> SseFrame {
        SseFrame::Event {
            id: id.map(str::to_string),
            event: event.to_string(),
            data: data.to_string(),
            retry_ms: None,
        }
    }

    #[test]
    fn parses_events_comments_and_multiline_data() {
        let mut parser = SseParser::new();
        let frames = parser.feed(b": ping\n\nid: 7\nevent: update\ndata: first\ndata: second\n\ndata\n\n");
        assert_eq!(
            frames,
            vec![
                SseFrame::Comment("ping".to_string()),
                event(Some("7"), "update", "first\nsecond"),
                // A bare field name is a field with an empty value.
                event(None, "message", ""),
            ]
        );
        assert!(!parser.has_incomplete_event());
    }

    #[test]
    fn a_block_without_data_dispatches_nothing() {
        let mut parser = SseParser::new();
        let frames = parser.feed(b"event: lonely\nid: 3\n\ndata: next\n\n");
        // The type and id of the empty block must not leak into the next one.
        assert_eq!(frames, vec![event(None, "message", "next")]);
    }

    #[test]
    fn chunk_boundaries_do_not_matter() {
        let stream = "\u{FEFF}data: привет\r\n\r\nevent: x\rdata: a:b\r\r".as_bytes();
        let whole = SseParser::new().feed(stream);
        assert_eq!(whole, vec![event(None, "message", "привет"), event(None, "x", "a:b")]);

        // Byte by byte splits every UTF-8 sequence and every CRLF.
        let mut parser = SseParser::new();
        let split: Vec<SseFrame> = stream.iter().flat_map(|b| parser.feed(&[*b])).collect();
        assert_eq!(split, whole);
    }

    #[test]
    fn retry_takes_digits_only_and_ids_with_nul_are_ignored() {
        let mut parser = SseParser::new();
        let frames = parser.feed(b"retry: 1500\nid: a\0b\ndata: x\n\nretry: soon\ndata: y\n\n");
        assert_eq!(
            frames,
            vec![
                SseFrame::Event {
                    id: None,
                    event: "message".to_string(),
                    data: "x".to_string(),
                    retry_ms: Some(1500),
                },
                event(None, "message", "y"),
            ]
        );
    }

    #[test]
    fn an_unfinished_event_is_reported_as_incomplete() {
        let mut parser = SseParser::new();
        assert!(parser.feed(b"data: half").is_empty());
        assert!(parser.has_incomplete_event());
    }

    #[test]
    fn only_a_successful_event_stream_counts_as_one() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, "Text/Event-Stream; charset=utf-8".parse().unwrap());
        assert!(is_event_stream(&headers));
        headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
        assert!(!is_event_stream(&headers));
        assert!(!is_event_stream(&reqwest::header::HeaderMap::new()));
    }

    /// Serves one canned HTTP response on a loopback port and returns its
    /// URL. Loopback only - the suite never reaches the network.
    fn serve_once(response: &'static [u8]) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(response);
        });
        format!("http://{addr}/events")
    }

    fn request(url: String) -> ResolvedHttpRequest {
        ResolvedHttpRequest {
            method: HttpMethod::Get,
            url,
            headers: vec![],
            body: None,
        }
    }

    #[tokio::test]
    async fn streams_events_until_the_server_closes() {
        let url = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
              : hi\n\ndata: one\n\nevent: tick\ndata: two\n\n",
        );
        let mut messages = Vec::new();
        let mut recorder = TraceRecorder::start();
        let outcome = SseExecutor::new()
            .stream(
                &request(url),
                &ExecutionContext::default(),
                &mut recorder,
                std::future::pending(),
                &mut |m| messages.push(m),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, 200);
        assert_eq!(outcome.events, 2);
        assert_eq!(outcome.end, StreamEnd::Closed);
        assert!(matches!(messages[0], SseMessage::Open { status: 200, .. }));
        let data: Vec<&str> = messages
            .iter()
            .filter_map(|m| match m {
                SseMessage::Event { data, .. } => Some(data.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(data, ["one", "two"]);
        assert!(messages.iter().any(|m| matches!(m, SseMessage::Comment { text, .. } if text == "hi")));
    }

    #[tokio::test]
    async fn an_ordinary_response_is_handed_over_whole() {
        let url = serve_once(
            b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 14\r\nConnection: close\r\n\r\n\
              {\"error\":\"no\"}",
        );
        let mut recorder = TraceRecorder::start();
        let outcome = SseExecutor::new()
            .stream(
                &request(url),
                &ExecutionContext::default(),
                &mut recorder,
                std::future::pending(),
                &mut |_| {},
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, 401);
        let StreamEnd::NotAStream { body_base64 } = outcome.end else {
            panic!("expected the body back, got {:?}", outcome.end);
        };
        let body = base64::engine::general_purpose::STANDARD.decode(body_base64).unwrap();
        assert_eq!(body, b"{\"error\":\"no\"}");
    }

    #[tokio::test]
    async fn disconnecting_an_open_stream_is_an_ordinary_end() {
        // Never closes: no Content-Length, no chunk terminator, and the
        // thread keeps the socket alive by sleeping on it.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/events", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\ndata: first\n\n");
            std::thread::sleep(std::time::Duration::from_secs(5));
        });

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let mut tx = Some(tx);
        let mut recorder = TraceRecorder::start();
        let outcome = SseExecutor::new()
            .stream(
                &request(url),
                &ExecutionContext::default(),
                &mut recorder,
                async {
                    let _ = rx.await;
                },
                // Disconnect as soon as the first event lands.
                &mut |m| {
                    if matches!(m, SseMessage::Event { .. }) {
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(());
                        }
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.events, 1);
        assert_eq!(outcome.end, StreamEnd::Cancelled);
        assert!(recorder.elapsed() < std::time::Duration::from_secs(4));
    }
}
