use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Protocol this request executes over. `Http` and `Sse` are implemented;
/// the others exist so `RequestFile` can grow sibling `Option` fields (e.g.
/// `ws: Option<WsRequestSpec>`) later without a breaking schema migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    WebSocket,
    Sse,
    GraphQl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    #[serde(flatten)]
    pub sync: SyncMeta,
    pub name: String,
    pub seq: u32,
    pub protocol: Protocol,
}

/// One request file on disk. Protocols that need more than an HTTP request
/// can describe add their own `Option<...Spec>` sibling field here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestFile {
    pub meta: RequestMeta,
    /// The HTTP request to send. An SSE request keeps its spec here as well:
    /// what opens a stream *is* an HTTP request (method, URL, headers, auth,
    /// body), and only the way the response is read differs. That is also
    /// what makes switching a request between the two lossless.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpRequestSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthSpec {
    None,
    Basic { username: String, password: String },
    Bearer { token: String },
}

impl Default for AuthSpec {
    fn default() -> Self {
        AuthSpec::None
    }
}

/// Text body formats. Each one only decides the default `Content-Type` and
/// which highlighting the editor uses - the payload is always the text the
/// user typed, sent as-is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Plain,
    Json,
    Xml,
    Yaml,
    Edn,
    Html,
    Css,
    Javascript,
}

impl TextFormat {
    pub fn content_type(self) -> &'static str {
        match self {
            TextFormat::Plain => "text/plain",
            TextFormat::Json => "application/json",
            TextFormat::Xml => "application/xml",
            TextFormat::Yaml => "application/yaml",
            TextFormat::Edn => "application/edn",
            TextFormat::Html => "text/html",
            TextFormat::Css => "text/css",
            TextFormat::Javascript => "application/javascript",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BodySpec {
    None,
    /// Kept for files written before formats existed: a raw body is plain
    /// text with no `Content-Type` of its own.
    Raw {
        content: String,
    },
    /// Also predates `Text`; same thing with `format = "json"`.
    Json {
        content: String,
    },
    Text {
        content: String,
        format: TextFormat,
    },
    Form {
        fields: Vec<KeyValue>,
    },
    /// Sends a file from disk verbatim. The path is absolute and local, so a
    /// request shared through a synced workspace will point at nothing on
    /// someone else's machine - deliberately, since copying the file into
    /// the workspace is the user's decision to make.
    File {
        path: String,
    },
}

impl Default for BodySpec {
    fn default() -> Self {
        BodySpec::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestSpec {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, rename = "query")]
    pub query_params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub auth: AuthSpec,
    #[serde(default)]
    pub body: BodySpec,
}

impl RequestFile {
    pub fn new_http(name: impl Into<String>, seq: u32, method: HttpMethod) -> Self {
        Self::new(name, seq, Protocol::Http, method)
    }

    /// A blank request over one of the protocols that are described by an
    /// HTTP request (`Http`, `Sse`).
    pub fn new(name: impl Into<String>, seq: u32, protocol: Protocol, method: HttpMethod) -> Self {
        RequestFile {
            meta: RequestMeta {
                sync: SyncMeta::new(),
                name: name.into(),
                seq,
                protocol,
            },
            http: Some(HttpRequestSpec {
                method,
                url: String::new(),
                query_params: Vec::new(),
                headers: Vec::new(),
                auth: AuthSpec::None,
                body: BodySpec::None,
            }),
        }
    }
}
