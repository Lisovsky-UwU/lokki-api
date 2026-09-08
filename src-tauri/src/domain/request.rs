use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Protocol this request executes over. Only `Http` has a matching spec
/// implemented in MVP; the others exist so `RequestFile` can grow sibling
/// `Option` fields (e.g. `ws: Option<WsRequestSpec>`) later without a
/// breaking schema migration.
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

/// One request file on disk. `http` is the only populated variant in MVP;
/// future protocols add their own `Option<...Spec>` sibling field here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestFile {
    pub meta: RequestMeta,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BodySpec {
    None,
    Raw { content: String },
    Json { content: String },
    Form { fields: Vec<KeyValue> },
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
        RequestFile {
            meta: RequestMeta {
                sync: SyncMeta::new(),
                name: name.into(),
                seq,
                protocol: Protocol::Http,
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
