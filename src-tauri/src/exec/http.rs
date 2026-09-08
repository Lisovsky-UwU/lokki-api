use super::{ExecutionContext, ExecutionOutcome, ExecutorError, ProtocolExecutor, ResolvedHttpRequest};
use crate::domain::{HttpMethod, KeyValue};
use async_trait::async_trait;
use base64::Engine;
use std::time::Instant;

pub struct HttpExecutor {
    client: reqwest::Client,
}

impl HttpExecutor {
    pub fn new() -> Self {
        HttpExecutor {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn method_to_reqwest(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    }
}

#[async_trait]
impl ProtocolExecutor for HttpExecutor {
    async fn execute(
        &self,
        request: &ResolvedHttpRequest,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutcome, ExecutorError> {
        let mut builder = self
            .client
            .request(method_to_reqwest(request.method), &request.url)
            .timeout(ctx.timeout);

        for header in &request.headers {
            if header.enabled {
                builder = builder.header(&header.key, &header.value);
            }
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        let start = Instant::now();
        let response = builder
            .send()
            .await
            .map_err(|e| ExecutorError::Failed(e.to_string()))?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| KeyValue {
                key: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
                enabled: true,
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|e| ExecutorError::Failed(e.to_string()))?;

        Ok(ExecutionOutcome {
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            headers,
            body_base64: base64::engine::general_purpose::STANDARD.encode(&body),
            duration_ms,
        })
    }
}
