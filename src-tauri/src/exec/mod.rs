pub mod http;
pub mod trace;

pub use http::HttpExecutor;
pub use trace::{ExecutionTrace, Phase, TraceEvent, TraceLevel, TraceRecorder};

use crate::domain::{AuthSpec, BodySpec, HttpMethod, HttpRequestSpec, KeyValue, RequestSettings};
use crate::interpolate::Resolver;
use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A fully-resolved HTTP request: every `{{variable}}` has already been
/// substituted by interpolate::Resolver before this reaches an executor.
#[derive(Debug, Clone)]
pub struct ResolvedHttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub settings: RequestSettings,
}

/// The response itself. How long it took lives in the `ExecutionTrace` the
/// caller passes into `execute`, not here: timings exist for a failed
/// attempt too, and that one produces no outcome at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    /// Base64-encoded response body — cheaper over IPC/JSON than a raw
    /// byte-array, decoded client-side for display.
    pub body_base64: String,
}

/// Carries a message written for the user: the raw reqwest error text
/// ("error sending request for url (...): dns error") says nothing to
/// someone testing an API, so http::describe_error turns it into a sentence
/// naming what went wrong and, where there is one, the fix.
#[derive(Debug, Error, Serialize)]
pub enum ExecutorError {
    #[error("{0}")]
    Failed(String),
}

/// Future protocols (WebSocket, SSE, GraphQL) implement this trait as
/// siblings to HttpExecutor and get their own Tauri commands (e.g. a
/// streamed `send_ws_connect`) rather than overloading `send_request`.
///
/// `trace` is where the implementation reports where the time went and what
/// happened. Filling it is best-effort and transport-specific: whatever a
/// given client can't see stays `None`, which the UI shows as "not
/// measured" rather than as zero.
#[async_trait]
pub trait ProtocolExecutor: Send + Sync {
    async fn execute(
        &self,
        request: &ResolvedHttpRequest,
        ctx: &ExecutionContext,
        trace: &mut TraceRecorder,
    ) -> Result<ExecutionOutcome, ExecutorError>;
}

fn form_urlencode_pairs(pairs: &[(String, String)]) -> String {
    // Reuses url::Url's WWW-form-urlencoded query serializer rather than
    // hand-rolling percent-encoding.
    let mut url = reqwest::Url::parse("http://x/").expect("static base url");
    {
        let mut qp = url.query_pairs_mut();
        for (k, v) in pairs {
            qp.append_pair(k, v);
        }
    }
    url.query().unwrap_or("").to_string()
}

/// Substitutes `{{variables}}` throughout an `HttpRequestSpec` (url, query,
/// headers, auth, body) via `resolver`, folding auth into an `Authorization`
/// header and body-type-appropriate `Content-Type`. Returns the resolved
/// request plus any variable names that couldn't be found (left verbatim in
/// the output rather than failing the whole request).
pub fn resolve_http_request(spec: &HttpRequestSpec, resolver: &Resolver) -> (ResolvedHttpRequest, Vec<String>) {
    let mut unresolved = Vec::new();
    let mut interp = |s: &str| -> String {
        let (out, missing) = resolver.interpolate(s);
        unresolved.extend(missing);
        out
    };

    let mut url = interp(&spec.url);
    // Blank-key rows are UI scaffolding (the always-present trailing empty
    // row), never real params/headers — sending them would make reqwest
    // reject the whole request as an invalid header name.
    let enabled_query: Vec<(String, String)> = spec
        .query_params
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .map(|q| (interp(&q.key), interp(&q.value)))
        .collect();
    if !enabled_query.is_empty() {
        if let Ok(mut parsed) = reqwest::Url::parse(&url) {
            {
                let mut pairs = parsed.query_pairs_mut();
                for (k, v) in &enabled_query {
                    pairs.append_pair(k, v);
                }
            }
            url = parsed.to_string();
        }
    }

    let mut headers: Vec<KeyValue> = spec
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .map(|h| KeyValue {
            key: interp(&h.key),
            value: interp(&h.value),
            enabled: true,
        })
        .collect();

    match &spec.auth {
        AuthSpec::None => {}
        AuthSpec::Bearer { token } => {
            headers.push(KeyValue {
                key: "Authorization".to_string(),
                value: format!("Bearer {}", interp(token)),
                enabled: true,
            });
        }
        AuthSpec::Basic { username, password } => {
            let credentials = format!("{}:{}", interp(username), interp(password));
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
            headers.push(KeyValue {
                key: "Authorization".to_string(),
                value: format!("Basic {}", encoded),
                enabled: true,
            });
        }
    }

    let has_content_type = |headers: &[KeyValue]| headers.iter().any(|h| h.key.eq_ignore_ascii_case("content-type"));

    let body = match &spec.body {
        BodySpec::None => None,
        BodySpec::Raw { content } => Some(interp(content).into_bytes()),
        BodySpec::Json { content } => {
            if !has_content_type(&headers) {
                headers.push(KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                });
            }
            Some(interp(content).into_bytes())
        }
        BodySpec::Form { fields } => {
            if !has_content_type(&headers) {
                headers.push(KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/x-www-form-urlencoded".to_string(),
                    enabled: true,
                });
            }
            let pairs: Vec<(String, String)> = fields
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| (interp(&f.key), interp(&f.value)))
                .collect();
            Some(form_urlencode_pairs(&pairs).into_bytes())
        }
    };

    (
        ResolvedHttpRequest {
            method: spec.method,
            url,
            headers,
            body,
        },
        unresolved,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::KeyValue as KV;
    use crate::interpolate::VariableScope;
    use std::collections::HashMap;

    fn scope(pairs: &[(&str, &str)]) -> VariableScope {
        VariableScope(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    #[test]
    fn interpolates_url_query_headers_and_bearer_auth() {
        let spec = HttpRequestSpec {
            method: HttpMethod::Get,
            url: "{{baseUrl}}/pets".to_string(),
            query_params: vec![KV { key: "limit".into(), value: "{{limit}}".into(), enabled: true }],
            headers: vec![KV { key: "Accept".into(), value: "application/json".into(), enabled: true }],
            auth: AuthSpec::Bearer { token: "{{token}}".into() },
            body: BodySpec::None,
        };
        let resolver = Resolver::new(
            scope(&[("baseUrl", "https://api.example.com"), ("limit", "20"), ("token", "abc123")]),
            None,
        );
        let (resolved, unresolved) = resolve_http_request(&spec, &resolver);
        assert!(unresolved.is_empty());
        assert_eq!(resolved.url, "https://api.example.com/pets?limit=20");
        assert!(resolved.headers.iter().any(|h| h.key == "Authorization" && h.value == "Bearer abc123"));
    }

    #[test]
    fn json_body_gets_default_content_type() {
        let spec = HttpRequestSpec {
            method: HttpMethod::Post,
            url: "https://api.example.com/pets".to_string(),
            query_params: vec![],
            headers: vec![],
            auth: AuthSpec::None,
            body: BodySpec::Json { content: "{\"name\":\"Rex\"}".to_string() },
        };
        let resolver = Resolver::new(VariableScope(HashMap::new()), None);
        let (resolved, _) = resolve_http_request(&spec, &resolver);
        assert!(resolved.headers.iter().any(|h| h.key == "Content-Type" && h.value == "application/json"));
        assert_eq!(resolved.body.unwrap(), b"{\"name\":\"Rex\"}".to_vec());
    }

    #[test]
    fn blank_key_rows_are_excluded() {
        let spec = HttpRequestSpec {
            method: HttpMethod::Get,
            url: "https://api.example.com/pets".to_string(),
            // The UI always keeps one trailing blank row; it must never
            // reach the wire (reqwest rejects an empty header name).
            query_params: vec![KV { key: "".into(), value: "".into(), enabled: true }],
            headers: vec![KV { key: "  ".into(), value: "x".into(), enabled: true }],
            auth: AuthSpec::None,
            body: BodySpec::None,
        };
        let resolver = Resolver::new(VariableScope(HashMap::new()), None);
        let (resolved, _) = resolve_http_request(&spec, &resolver);
        assert_eq!(resolved.url, "https://api.example.com/pets");
        assert!(resolved.headers.is_empty());
    }

    #[test]
    fn disabled_query_params_and_headers_are_excluded() {
        let spec = HttpRequestSpec {
            method: HttpMethod::Get,
            url: "https://api.example.com/pets".to_string(),
            query_params: vec![KV { key: "limit".into(), value: "20".into(), enabled: false }],
            headers: vec![KV { key: "X-Debug".into(), value: "1".into(), enabled: false }],
            auth: AuthSpec::None,
            body: BodySpec::None,
        };
        let resolver = Resolver::new(VariableScope(HashMap::new()), None);
        let (resolved, _) = resolve_http_request(&spec, &resolver);
        assert_eq!(resolved.url, "https://api.example.com/pets");
        assert!(resolved.headers.is_empty());
    }
}
