//! WebSocket: a connection both sides talk over for as long as it stays open.
//!
//! The handshake is an HTTP GET with an `Upgrade`, so the request that opens
//! a socket is resolved exactly like any other (`resolve_http_request`) -
//! URL, query, headers and auth. After that nothing is request/response any
//! more: messages go out whenever the user sends one and come in whenever
//! the server does, which is why this is an executor of its own beside
//! `ProtocolExecutor`, the way `sse::SseExecutor` is.
//!
//! The connection is opened here step by step - DNS, TCP, TLS, then the
//! handshake through tungstenite - rather than by tungstenite's own
//! `connect`. That is what puts each step in the trace on its own, and what
//! lets the TLS setting apply the same way it does to HTTP.

use super::{ExecutionContext, ExecutionTrace, ExecutorError, Phase, ResolvedHttpRequest, TraceRecorder};
use crate::domain::{effective_user_agent, optional_duration, KeyValue};
use crate::i18n::messages;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::rustls::{self, ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

/// After the user disconnects, how long the server gets to answer the close
/// frame before the connection is dropped anyway. A well-behaved server
/// replies at once; one that doesn't must not keep the button spinning.
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// Headers the handshake is made of. tungstenite writes them itself and
/// checks the server's answer against the key it generated, so a value from
/// the request would at best be ignored and at worst break the handshake.
/// `Host` is not among them: overriding it to reach a virtual host is a
/// legitimate thing to test.
const HANDSHAKE_HEADERS: [&str; 4] = ["connection", "upgrade", "sec-websocket-version", "sec-websocket-key"];

/// What the connection sends to the UI, in order. The executor emits `Open`
/// and every `Frame`; the command sends `End` last, once the outcome and the
/// trace are both known - on the same channel, so it can't overtake the
/// final frames (see `sse::SseMessage` for the same arrangement).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WsMessage {
    /// The server answered the handshake - with 101, or with something else
    /// that `End` will then carry as `Rejected`.
    Open {
        status: u16,
        status_text: String,
        headers: Vec<KeyValue>,
    },
    /// One message, either way, or a control frame. Fragmented messages
    /// arrive already reassembled.
    Frame {
        /// How far into the connection it went out or came in, on the same
        /// clock as the trace log.
        at_ms: u64,
        direction: Direction,
        opcode: Opcode,
        /// The text of a text message; the bytes of anything else, base64
        /// encoded - a binary payload is not text and must not be decoded as
        /// such on the way to the UI.
        data: String,
        /// In bytes, as it went over the wire.
        size: u64,
        /// For a sent message: names in it that stayed `{{literal}}`.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        unresolved_variables: Vec<String>,
    },
    End {
        outcome: WsOutcome,
        trace: ExecutionTrace,
        unresolved_variables: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Sent,
    Received,
}

/// Pings and pongs are shown alongside the messages: a heartbeat that stops
/// is often exactly what is being debugged. Close frames are not - how the
/// connection ended is what `SocketEnd` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Opcode {
    Text,
    Binary,
    Ping,
    Pong,
}

/// Why the connection is over.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SocketEnd {
    /// The server closed it. `code` is absent when its close frame carried
    /// none, which the protocol allows.
    Closed { code: Option<u16>, reason: String },
    /// The user disconnected - the normal way a socket ends, not a failure.
    Cancelled,
    /// The connection broke after it had opened. What was exchanged before
    /// still stands.
    Failed { message: String },
    /// The server refused to switch protocols and answered with an ordinary
    /// HTTP response. That answer is usually the explanation (a 401, a 404
    /// on the wrong path), so its body is handed over whole.
    Rejected { body_base64: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct WsOutcome {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    /// Data messages only - pings and pongs are not counted.
    pub sent: u64,
    pub received: u64,
    pub end: SocketEnd,
}

/// A message the user asked to send, its variables already substituted.
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub text: String,
    pub unresolved_variables: Vec<String>,
}

/// The address to connect to, as a `ws://` or `wss://` URL. `http(s)://` is
/// taken as well and mapped across: APIs often document their socket
/// endpoints that way, and the handshake is an HTTP request to that very
/// address.
fn websocket_url(raw: &str) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(raw).map_err(|_| messages::invalid_url(raw))?;
    let scheme = match url.scheme() {
        "ws" | "http" => "ws",
        "wss" | "https" => "wss",
        other => return Err(messages::ws_unsupported_scheme(other)),
    };
    if url.scheme() != scheme {
        url.set_scheme(scheme).map_err(|_| messages::invalid_url(raw))?;
    }
    if url.host().is_none() {
        return Err(messages::invalid_url(raw));
    }
    // A fragment never leaves the client, and the handshake's request line
    // has no room for one.
    url.set_fragment(None);
    Ok(url)
}

/// The host as DNS and TLS want it: an IPv6 address without its brackets.
fn host_name(url: &reqwest::Url) -> String {
    let host = url.host_str().unwrap_or("");
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
        .to_string()
}

/// The handshake request: the headers tungstenite needs, then the request's
/// own. A header the request sets replaces the generated one of that name,
/// and a repeated header is kept repeated.
fn handshake_request(
    url: &reqwest::Url,
    headers: &[KeyValue],
    user_agent: &str,
    trace: &mut TraceRecorder,
) -> Result<Request, String> {
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| messages::invalid_url(url.as_str()))?;
    let target = request.headers_mut();
    let mut seen: Vec<HeaderName> = Vec::new();
    for header in headers.iter().filter(|h| h.enabled) {
        let name = HeaderName::from_bytes(header.key.trim().as_bytes())
            .map_err(|_| messages::invalid_header_name(&header.key))?;
        if HANDSHAKE_HEADERS.contains(&name.as_str()) {
            trace.warn(messages::ws_header_ignored(&header.key));
            continue;
        }
        let value =
            HeaderValue::from_str(&header.value).map_err(|_| messages::invalid_header_value(&header.key))?;
        if seen.contains(&name) {
            target.append(name, value);
        } else {
            target.insert(name.clone(), value);
            seen.push(name);
        }
    }
    if !target.contains_key("user-agent") {
        if let Ok(value) = HeaderValue::from_str(user_agent) {
            target.insert("user-agent", value);
        }
    }
    Ok(request)
}

/// Resolves to `None` if `cancelled` fires first. Each step of opening the
/// connection goes through this rather than one `select!` around all of
/// them, so the steps can write to the trace as they finish.
async fn unless_cancelled<F: Future>(cancelled: Pin<&mut impl Future<Output = ()>>, step: F) -> Option<F::Output> {
    tokio::select! {
        output = step => Some(output),
        _ = cancelled => None,
    }
}

async fn within<F: Future>(
    deadline: Option<tokio::time::Instant>,
    step: F,
) -> Result<F::Output, tokio::time::error::Elapsed> {
    match deadline {
        Some(deadline) => tokio::time::timeout_at(deadline, step).await,
        None => Ok(step.await),
    }
}

fn describe_io_error(error: &std::io::Error, host: &str) -> String {
    use std::io::ErrorKind;
    if let Some(tls) = error.get_ref().and_then(|inner| inner.downcast_ref::<rustls::Error>()) {
        return match tls {
            rustls::Error::InvalidCertificate(_) => messages::tls_failed(host),
            other => messages::connect_failed(host, &other.to_string()),
        };
    }
    match error.kind() {
        ErrorKind::ConnectionRefused => messages::connection_refused(host),
        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe | ErrorKind::UnexpectedEof => {
            messages::connection_reset(host)
        }
        ErrorKind::NetworkUnreachable | ErrorKind::HostUnreachable => messages::network_unreachable(host),
        _ => messages::connect_failed(host, &error.to_string()),
    }
}

fn describe_ws_error(error: &WsError, host: &str) -> String {
    match error {
        WsError::Io(io) => describe_io_error(io, host),
        WsError::Tls(_) => messages::tls_failed(host),
        WsError::Capacity(detail) => messages::ws_message_too_large(&detail.to_string()),
        // What a server that simply drops the socket looks like from here.
        WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => messages::connection_reset(host),
        WsError::Protocol(detail) => messages::ws_protocol_error(host, &detail.to_string()),
        other => messages::ws_failed(host, &other.to_string()),
    }
}

fn status_text(status: u16) -> String {
    reqwest::StatusCode::from_u16(status)
        .ok()
        .and_then(|s| s.canonical_reason())
        .unwrap_or("")
        .to_string()
}

fn header_pairs(headers: &tokio_tungstenite::tungstenite::http::HeaderMap) -> Vec<KeyValue> {
    headers
        .iter()
        .map(|(k, v)| KeyValue {
            key: k.to_string(),
            value: v.to_str().unwrap_or("").to_string(),
            enabled: true,
        })
        .collect()
}

/// Accepts whatever certificate the server shows, for the "verify TLS" switch
/// in settings turned off - the same thing reqwest's
/// `danger_accept_invalid_certs` does for HTTP. Signatures are still checked:
/// the handshake has to be with whoever holds the key of the certificate it
/// presented, trusted or not.
#[derive(Debug)]
struct AcceptAnyCertificate(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn build_tls_config(verify: bool) -> Arc<ClientConfig> {
    // The provider is named rather than left to rustls' process default,
    // which is only picked automatically while exactly one is compiled in.
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("ring supports the default protocol versions");
    // No ALPN: a socket has to be opened over HTTP/1.1, and offering h2
    // would let a server pick a protocol the handshake can't run on.
    let config = if verify {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyCertificate(provider)))
            .with_no_client_auth()
    };
    Arc::new(config)
}

pub struct WsExecutor {
    /// Built on first use and kept: setting up the root store is the
    /// expensive part of a TLS connection, and the two variants are all the
    /// TLS setting can ask for.
    verified: OnceLock<Arc<ClientConfig>>,
    unverified: OnceLock<Arc<ClientConfig>>,
}

impl WsExecutor {
    pub fn new() -> Self {
        WsExecutor {
            verified: OnceLock::new(),
            unverified: OnceLock::new(),
        }
    }

    fn tls_config(&self, verify: bool) -> Arc<ClientConfig> {
        let slot = if verify { &self.verified } else { &self.unverified };
        slot.get_or_init(|| build_tls_config(verify)).clone()
    }

    /// Opens the socket and keeps it open until the server closes it, the
    /// connection breaks, or `cancelled` resolves. Messages from `outgoing`
    /// are sent as they come; everything that crosses the socket goes out
    /// through `emit`.
    ///
    /// As with a stream, cancelling is passed in rather than done by
    /// dropping this future: once the socket is open, disconnecting is the
    /// ordinary end, and it owes the server a close frame and the UI an
    /// outcome. Before the handshake completes there is nothing to report,
    /// and cancelling fails the attempt like a cancelled HTTP send.
    pub async fn connect(
        &self,
        request: &ResolvedHttpRequest,
        ctx: &ExecutionContext,
        trace: &mut TraceRecorder,
        cancelled: impl Future<Output = ()> + Send,
        outgoing: &mut mpsc::UnboundedReceiver<OutgoingMessage>,
        emit: &mut (dyn FnMut(WsMessage) + Send),
    ) -> Result<WsOutcome, ExecutorError> {
        tokio::pin!(cancelled);
        let fail = |trace: &mut TraceRecorder, message: String| {
            trace.error(message.clone());
            ExecutorError::Failed(message)
        };

        let url = websocket_url(&request.url).map_err(|message| fail(trace, message))?;
        let host = host_name(&url);
        let port = url.port_or_known_default().unwrap_or(80);
        let secure = url.scheme() == "wss";
        trace.info(format!("GET {url}"));
        if secure && !ctx.settings.verify_tls {
            trace.warn(messages::tls_verification_disabled());
        }
        let handshake = handshake_request(&url, &request.headers, &effective_user_agent(&ctx.settings), trace)
            .map_err(|message| fail(trace, message))?;

        // One deadline for DNS, TCP and TLS together: the connect timeout is
        // "how long until there is a connection", however many steps that
        // takes, which is also how reqwest applies it.
        let deadline = optional_duration(ctx.settings.connect_timeout_ms).map(|limit| tokio::time::Instant::now() + limit);

        let dns_at = Instant::now();
        let addresses = match unless_cancelled(cancelled.as_mut(), within(deadline, tokio::net::lookup_host((host.as_str(), port)))).await {
            None => return Err(fail(trace, messages::request_cancelled())),
            Some(Err(_)) => return Err(fail(trace, messages::connect_timeout(&host))),
            Some(Ok(Err(_))) => return Err(fail(trace, messages::dns_failed(&host))),
            Some(Ok(Ok(addresses))) => addresses.collect::<Vec<_>>(),
        };
        trace.phase(Phase::Dns, dns_at.elapsed());

        // Every address in turn, as a browser would: a host that publishes
        // an IPv6 address it doesn't listen on is common enough.
        let connect_at = Instant::now();
        let tcp = async {
            let mut last_error = std::io::Error::new(std::io::ErrorKind::NotFound, messages::dns_failed(&host));
            for address in &addresses {
                match TcpStream::connect(address).await {
                    Ok(stream) => return Ok(stream),
                    Err(e) => last_error = e,
                }
            }
            Err(last_error)
        };
        let tcp = match unless_cancelled(cancelled.as_mut(), within(deadline, tcp)).await {
            None => return Err(fail(trace, messages::request_cancelled())),
            Some(Err(_)) => return Err(fail(trace, messages::connect_timeout(&host))),
            Some(Ok(Err(e))) => return Err(fail(trace, describe_io_error(&e, &host))),
            Some(Ok(Ok(tcp))) => tcp,
        };
        trace.phase(Phase::Connect, connect_at.elapsed());
        trace.set_reused_connection(false);
        if let Ok(address) = tcp.peer_addr() {
            trace.set_remote_addr(address.to_string());
        }
        // Messages are small and interactive; batching them up would only
        // add latency to what the user is watching.
        let _ = tcp.set_nodelay(true);

        if !secure {
            return self
                .session(tcp, handshake, &host, ctx, trace, cancelled.as_mut(), outgoing, emit)
                .await;
        }

        let server_name = rustls::pki_types::ServerName::try_from(host.clone())
            .map_err(|_| fail(trace, messages::invalid_url(&request.url)))?;
        let connector = TlsConnector::from(self.tls_config(ctx.settings.verify_tls));
        let tls_at = Instant::now();
        let tls = match unless_cancelled(cancelled.as_mut(), within(deadline, connector.connect(server_name, tcp))).await {
            None => return Err(fail(trace, messages::request_cancelled())),
            Some(Err(_)) => return Err(fail(trace, messages::connect_timeout(&host))),
            Some(Ok(Err(e))) => return Err(fail(trace, describe_io_error(&e, &host))),
            Some(Ok(Ok(tls))) => tls,
        };
        trace.phase(Phase::Tls, tls_at.elapsed());
        self.session(tls, handshake, &host, ctx, trace, cancelled.as_mut(), outgoing, emit)
            .await
    }

    /// The handshake and the conversation that follows, over a transport
    /// that is already connected - plain TCP or TLS alike.
    #[allow(clippy::too_many_arguments)]
    async fn session<S>(
        &self,
        stream: S,
        handshake: Request,
        host: &str,
        ctx: &ExecutionContext,
        trace: &mut TraceRecorder,
        mut cancelled: Pin<&mut impl Future<Output = ()>>,
        outgoing: &mut mpsc::UnboundedReceiver<OutgoingMessage>,
        emit: &mut (dyn FnMut(WsMessage) + Send),
    ) -> Result<WsOutcome, ExecutorError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let fail = |trace: &mut TraceRecorder, message: String| {
            trace.error(message.clone());
            ExecutorError::Failed(message)
        };

        // The read timeout bounds the wait for the server's answer, as it
        // does for a stream's head. Once the socket is open, silence is
        // normal and nothing times it out.
        let wait_at = Instant::now();
        let deadline = optional_duration(ctx.settings.read_timeout_ms).map(|limit| tokio::time::Instant::now() + limit);
        let answer = tokio_tungstenite::client_async_with_config(handshake, stream, Some(WebSocketConfig::default()));
        let answer = unless_cancelled(cancelled.as_mut(), within(deadline, answer)).await;
        trace.phase(Phase::Wait, wait_at.elapsed());
        let (socket, response) = match answer {
            None => return Err(fail(trace, messages::request_cancelled())),
            Some(Err(_)) => return Err(fail(trace, messages::response_timeout(host))),
            Some(Ok(Err(WsError::Http(response)))) => {
                let status = response.status().as_u16();
                let status_text = status_text(status);
                let headers = header_pairs(response.headers());
                trace.info(messages::response_received(status, &status_text, &format!("{:?}", response.version())));
                trace.warn(messages::ws_rejected(status));
                emit(WsMessage::Open {
                    status,
                    status_text: status_text.clone(),
                    headers: headers.clone(),
                });
                let body = response.into_body().unwrap_or_default();
                return Ok(WsOutcome {
                    status,
                    status_text,
                    headers,
                    sent: 0,
                    received: 0,
                    end: SocketEnd::Rejected {
                        body_base64: base64::engine::general_purpose::STANDARD.encode(body),
                    },
                });
            }
            Some(Ok(Err(e))) => return Err(fail(trace, describe_ws_error(&e, host))),
            Some(Ok(Ok(opened))) => opened,
        };

        let status = response.status().as_u16();
        let status_text = status_text(status);
        let headers = header_pairs(response.headers());
        trace.info(messages::response_received(status, &status_text, &format!("{:?}", response.version())));
        let subprotocol = response
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        trace.info(messages::ws_opened(subprotocol.as_deref()));
        emit(WsMessage::Open {
            status,
            status_text: status_text.clone(),
            headers: headers.clone(),
        });

        let open_at = Instant::now();
        let (mut sink, mut source) = socket.split();
        let (mut sent, mut received) = (0u64, 0u64);
        let mut close: Option<CloseFrame> = None;
        let base64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);

        let end = loop {
            tokio::select! {
                _ = cancelled.as_mut() => {
                    let goodbye = CloseFrame { code: CloseCode::Normal, reason: "".into() };
                    if sink.send(Message::Close(Some(goodbye))).await.is_ok() {
                        // Read until the server's reply closes the stream, so
                        // the close handshake completes rather than the
                        // socket simply being dropped on the server.
                        let _ = tokio::time::timeout(CLOSE_GRACE, async {
                            while let Some(Ok(_)) = source.next().await {}
                        })
                        .await;
                    }
                    break SocketEnd::Cancelled;
                }
                Some(message) = outgoing.recv() => {
                    let at_ms = trace.elapsed().as_millis() as u64;
                    let size = message.text.len() as u64;
                    if let Err(e) = sink.send(Message::text(message.text.clone())).await {
                        let message = describe_ws_error(&e, host);
                        trace.error(message.clone());
                        break SocketEnd::Failed { message };
                    }
                    sent += 1;
                    emit(WsMessage::Frame {
                        at_ms,
                        direction: Direction::Sent,
                        opcode: Opcode::Text,
                        data: message.text,
                        size,
                        unresolved_variables: message.unresolved_variables,
                    });
                }
                frame = source.next() => {
                    let at_ms = trace.elapsed().as_millis() as u64;
                    let (opcode, data, size) = match frame {
                        Some(Ok(Message::Text(text))) => {
                            received += 1;
                            (Opcode::Text, text.to_string(), text.len())
                        }
                        Some(Ok(Message::Binary(bytes))) => {
                            received += 1;
                            (Opcode::Binary, base64(&bytes), bytes.len())
                        }
                        Some(Ok(Message::Ping(bytes))) => (Opcode::Ping, base64(&bytes), bytes.len()),
                        Some(Ok(Message::Pong(bytes))) => (Opcode::Pong, base64(&bytes), bytes.len()),
                        // tungstenite answers it by itself; the stream ends
                        // once the server then drops the connection.
                        Some(Ok(Message::Close(frame))) => {
                            close = frame;
                            continue;
                        }
                        Some(Ok(Message::Frame(_))) => continue,
                        None | Some(Err(WsError::ConnectionClosed)) => break SocketEnd::Closed {
                            code: close.as_ref().map(|f| u16::from(f.code)),
                            reason: close.as_ref().map(|f| f.reason.to_string()).unwrap_or_default(),
                        },
                        Some(Err(e)) => {
                            let message = describe_ws_error(&e, host);
                            trace.error(message.clone());
                            break SocketEnd::Failed { message };
                        }
                    };
                    emit(WsMessage::Frame {
                        at_ms,
                        direction: Direction::Received,
                        opcode,
                        data,
                        size: size as u64,
                        unresolved_variables: Vec::new(),
                    });
                }
            }
        };
        // As for a stream, "download" is how long the socket stayed open.
        trace.phase(Phase::Download, open_at.elapsed());

        match &end {
            SocketEnd::Closed { code, reason } => {
                let code = code.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string());
                trace.info(messages::ws_closed_by_server(&code, reason, sent, received));
            }
            SocketEnd::Cancelled => trace.info(messages::ws_closed_by_client(sent, received)),
            SocketEnd::Failed { .. } | SocketEnd::Rejected { .. } => {}
        }
        Ok(WsOutcome {
            status,
            status_text,
            headers,
            sent,
            received,
            end,
        })
    }
}

impl Default for WsExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HttpMethod;
    use std::io::{Read, Write};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::handshake::server::{Request as ServerRequest, Response as ServerResponse};

    fn request(url: String, headers: Vec<KeyValue>) -> ResolvedHttpRequest {
        ResolvedHttpRequest {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
        }
    }

    fn header(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            enabled: true,
        }
    }

    #[test]
    fn http_addresses_are_taken_as_their_websocket_counterparts() {
        assert_eq!(websocket_url("https://example.com/socket#x").unwrap().as_str(), "wss://example.com/socket");
        assert_eq!(websocket_url("http://example.com:8080/").unwrap().as_str(), "ws://example.com:8080/");
        assert_eq!(websocket_url("wss://[::1]:9000/ws").unwrap().as_str(), "wss://[::1]:9000/ws");
        assert!(websocket_url("ftp://example.com").is_err());
        assert!(websocket_url("not a url").is_err());
    }

    #[test]
    fn ipv6_hosts_lose_their_brackets_for_dns_and_tls() {
        assert_eq!(host_name(&websocket_url("ws://[::1]:9000/").unwrap()), "::1");
        assert_eq!(host_name(&websocket_url("ws://example.com/").unwrap()), "example.com");
    }

    #[test]
    fn the_request_headers_go_into_the_handshake_but_cannot_break_it() {
        let url = websocket_url("ws://example.com/chat").unwrap();
        let mut trace = TraceRecorder::start();
        let built = handshake_request(
            &url,
            &[
                header("Sec-WebSocket-Protocol", "chat"),
                header("X-Tag", "a"),
                header("X-Tag", "b"),
                header("Sec-WebSocket-Key", "mine"),
                header("Host", "virtual.example"),
            ],
            "LokkiAPI/test",
            &mut trace,
        )
        .unwrap();
        let headers = built.headers();
        assert_eq!(headers["sec-websocket-protocol"], "chat");
        assert_eq!(headers.get_all("x-tag").iter().count(), 2);
        assert_ne!(headers["sec-websocket-key"], "mine");
        assert_eq!(headers["host"], "virtual.example");
        assert_eq!(headers["user-agent"], "LokkiAPI/test");
        assert_eq!(trace.finish().events.len(), 1, "the ignored key header is reported");
    }

    #[test]
    fn an_invalid_header_name_is_refused_before_connecting() {
        let url = websocket_url("ws://example.com/").unwrap();
        let mut trace = TraceRecorder::start();
        assert!(handshake_request(&url, &[header("Bad Name", "x")], "ua", &mut trace).is_err());
    }

    /// An echo server on a loopback port that closes with 1000 "bye" after
    /// the first message. Hands back the URL and the headers the client's
    /// handshake carried.
    async fn echo_once() -> (String, tokio::sync::oneshot::Receiver<Vec<(String, String)>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/echo", listener.local_addr().unwrap());
        let (headers_tx, headers_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut seen = Vec::new();
            let callback = |request: &ServerRequest, response: ServerResponse| {
                seen = request
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                Ok(response)
            };
            let mut socket = tokio_tungstenite::accept_hdr_async(tcp, callback).await.unwrap();
            let _ = headers_tx.send(seen);
            socket.send(Message::Ping("beat".into())).await.unwrap();
            while let Some(Ok(message)) = socket.next().await {
                if let Message::Text(text) = message {
                    socket.send(Message::text(format!("echo: {text}"))).await.unwrap();
                    let bye = CloseFrame {
                        code: CloseCode::Normal,
                        reason: "bye".into(),
                    };
                    socket.send(Message::Close(Some(bye))).await.unwrap();
                }
            }
        });
        (url, headers_rx)
    }

    #[tokio::test]
    async fn sends_and_receives_until_the_server_closes() {
        let (url, headers) = echo_once().await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut messages = Vec::new();
        let mut recorder = TraceRecorder::start();
        let outcome = WsExecutor::new()
            .connect(
                &request(url, vec![header("X-Token", "secret")]),
                &ExecutionContext::default(),
                &mut recorder,
                std::future::pending(),
                &mut rx,
                &mut |message| {
                    // Say something as soon as the socket is open.
                    if matches!(message, WsMessage::Open { .. }) {
                        tx.send(OutgoingMessage {
                            text: "hello".to_string(),
                            unresolved_variables: vec!["missing".to_string()],
                        })
                        .unwrap();
                    }
                    messages.push(message);
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, 101);
        assert_eq!((outcome.sent, outcome.received), (1, 1));
        assert_eq!(
            outcome.end,
            SocketEnd::Closed {
                code: Some(1000),
                reason: "bye".to_string()
            }
        );

        let frames: Vec<(Direction, Opcode, &str)> = messages
            .iter()
            .filter_map(|m| match m {
                WsMessage::Frame {
                    direction, opcode, data, ..
                } => Some((*direction, *opcode, data.as_str())),
                _ => None,
            })
            .collect();
        let beat = base64::engine::general_purpose::STANDARD.encode("beat");
        assert!(frames.contains(&(Direction::Received, Opcode::Ping, beat.as_str())));
        assert!(frames.contains(&(Direction::Sent, Opcode::Text, "hello")));
        assert!(frames.contains(&(Direction::Received, Opcode::Text, "echo: hello")));
        assert!(messages.iter().any(|m| matches!(
            m,
            WsMessage::Frame { direction: Direction::Sent, unresolved_variables, .. } if unresolved_variables == &["missing"]
        )));

        let headers = headers.await.unwrap();
        assert!(headers.contains(&("x-token".to_string(), "secret".to_string())));
        assert!(headers.iter().any(|(k, _)| k == "user-agent"));

        let trace = recorder.finish();
        assert!(trace.dns_ms.is_some() && trace.connect_ms.is_some() && trace.wait_ms.is_some());
        assert!(trace.tls_ms.is_none(), "ws:// has no TLS phase");
    }

    #[tokio::test]
    async fn a_refused_upgrade_is_handed_over_as_an_ordinary_response() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("ws://{}/socket", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer);
            let _ = socket.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 14\r\nConnection: close\r\n\r\n\
                  {\"error\":\"no\"}",
            );
        });

        let (_tx, mut rx) = mpsc::unbounded_channel();
        let mut recorder = TraceRecorder::start();
        let outcome = WsExecutor::new()
            .connect(
                &request(url, vec![]),
                &ExecutionContext::default(),
                &mut recorder,
                std::future::pending(),
                &mut rx,
                &mut |_| {},
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, 401);
        let SocketEnd::Rejected { body_base64 } = outcome.end else {
            panic!("expected the body back, got {:?}", outcome.end);
        };
        let body = base64::engine::general_purpose::STANDARD.decode(body_base64).unwrap();
        assert_eq!(body, b"{\"error\":\"no\"}");
    }

    #[tokio::test]
    async fn disconnecting_sends_a_close_frame_and_ends_normally() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/", listener.local_addr().unwrap());
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            // Never says a word; waits for the client to leave, and keeps
            // reading after that so its reply to the close goes out.
            let mut close_tx = Some(close_tx);
            while let Some(Ok(message)) = socket.next().await {
                if let (Message::Close(frame), Some(tx)) = (message, close_tx.take()) {
                    let _ = tx.send(frame.map(|f| u16::from(f.code)));
                }
            }
        });

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        let mut cancel_tx = Some(cancel_tx);
        let (_tx, mut rx) = mpsc::unbounded_channel();
        let mut recorder = TraceRecorder::start();
        let outcome = WsExecutor::new()
            .connect(
                &request(url, vec![]),
                &ExecutionContext::default(),
                &mut recorder,
                async {
                    let _ = cancel_rx.await;
                },
                &mut rx,
                &mut |message| {
                    if matches!(message, WsMessage::Open { .. }) {
                        if let Some(tx) = cancel_tx.take() {
                            let _ = tx.send(());
                        }
                    }
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.end, SocketEnd::Cancelled);
        assert_eq!(close_rx.await.unwrap(), Some(1000));
        // The server answered the close, so the grace period was not needed.
        assert!(recorder.elapsed() < CLOSE_GRACE);
    }

    #[tokio::test]
    async fn a_refused_connection_fails_before_opening() {
        // Bound and dropped at once, so nothing listens on the port.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let (_tx, mut rx) = mpsc::unbounded_channel();
        let mut recorder = TraceRecorder::start();
        let result = WsExecutor::new()
            .connect(
                &request(format!("ws://127.0.0.1:{port}/"), vec![]),
                &ExecutionContext::default(),
                &mut recorder,
                std::future::pending(),
                &mut rx,
                &mut |_| {},
            )
            .await;
        assert!(result.is_err());
        assert!(recorder.finish().events.iter().any(|e| e.level == super::super::TraceLevel::Error));
    }
}
