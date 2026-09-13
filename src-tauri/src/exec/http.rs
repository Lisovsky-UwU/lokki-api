use super::{
    ExecutionContext, ExecutionOutcome, ExecutorError, Phase, ProtocolExecutor, ResolvedHttpRequest, TraceRecorder,
};
use crate::domain::{optional_duration, HttpMethod, KeyValue, RequestSettings};
use async_trait::async_trait;
use base64::Engine;
use std::sync::Mutex;
use std::time::Instant;

pub struct HttpExecutor {
    /// The client is cached, not rebuilt per request: building one sets up
    /// the whole rustls root store and throws away connection pooling. It is
    /// keyed by the settings that shape it, so changing a timeout or turning
    /// TLS checking off takes effect on the next request without a restart.
    cached: Mutex<Option<(RequestSettings, reqwest::Client)>>,
}

impl HttpExecutor {
    pub fn new() -> Self {
        HttpExecutor {
            cached: Mutex::new(None),
        }
    }

    fn client_for(&self, settings: &RequestSettings) -> Result<reqwest::Client, ExecutorError> {
        let mut cached = self.cached.lock().expect("http client cache poisoned");
        if let Some((for_settings, client)) = cached.as_ref() {
            if for_settings == settings {
                return Ok(client.clone());
            }
        }
        let client = build_client(settings)?;
        *cached = Some((settings.clone(), client.clone()));
        Ok(client)
    }
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn build_client(settings: &RequestSettings) -> Result<reqwest::Client, ExecutorError> {
    let mut builder = reqwest::Client::builder()
        // Testing against a staging box with a self-signed certificate is a
        // normal thing to do, so this is a setting rather than a hard rule —
        // on by default, and the settings dialog spells out what turning it
        // off means.
        .danger_accept_invalid_certs(!settings.verify_tls)
        .redirect(if settings.follow_redirects {
            reqwest::redirect::Policy::limited(settings.max_redirects as usize)
        } else {
            reqwest::redirect::Policy::none()
        });

    if let Some(connect) = optional_duration(settings.connect_timeout_ms) {
        builder = builder.connect_timeout(connect);
    }
    if let Some(read) = optional_duration(settings.read_timeout_ms) {
        builder = builder.read_timeout(read);
    }
    if !settings.user_agent.trim().is_empty() {
        builder = builder.user_agent(settings.user_agent.trim());
    }

    builder
        .build()
        .map_err(|e| ExecutorError::Failed(format!("Не удалось настроить HTTP-клиент: {e}")))
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

/// The whole `source()` chain flattened. reqwest keeps the interesting part
/// (the DNS or TLS failure) several levels below the top-level message.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(current) = source {
        parts.push(current.to_string());
        source = current.source();
    }
    parts.join(": ")
}

fn host_of(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string())
}

/// Turns a reqwest failure into a sentence the person testing an API can act
/// on. Classification leans on the typed predicates first and falls back to
/// matching the flattened chain, because the layers underneath reqwest
/// (hyper, rustls, the OS resolver) only report these as text.
pub fn describe_error(error: &reqwest::Error, url: &str) -> String {
    let host = host_of(url);
    let chain = error_chain(error).to_lowercase();

    if error.is_timeout() {
        return if error.is_connect() {
            format!("Не удалось подключиться к {host}: превышено время ожидания подключения. Увеличьте таймаут в настройках или проверьте доступность хоста.")
        } else {
            format!("Превышено время ожидания ответа от {host}. Увеличьте таймаут в настройках или проверьте, отвечает ли сервис.")
        };
    }
    if error.is_redirect() {
        return format!("Слишком много перенаправлений при обращении к {host}. Ограничение задаётся в настройках.");
    }
    if error.is_builder() || chain.contains("relative url without a base") {
        return format!("Некорректный адрес запроса: {url}");
    }
    // DNS: the OS resolver's own wording differs per platform and locale, so
    // both the generic and the Windows phrasings are matched.
    if chain.contains("dns error") || chain.contains("lookup address") || chain.contains("failed to lookup") {
        return format!("Не удалось определить адрес хоста {host}. Проверьте имя хоста и подключение к сети.");
    }
    if chain.contains("certificate") || chain.contains("invalid peer certificate") || chain.contains("certexpired") {
        return format!(
            "Не удалось проверить TLS-сертификат {host}. Если это тестовый сервер с самоподписанным сертификатом, отключите проверку TLS в настройках."
        );
    }
    if chain.contains("connection refused") || chain.contains("10061") {
        return format!("Хост {host} отклонил подключение. Проверьте порт и что сервис запущен.");
    }
    if chain.contains("connection reset") || chain.contains("10054") {
        return format!("Соединение с {host} разорвано сервером.");
    }
    if chain.contains("network is unreachable") || chain.contains("10051") || chain.contains("os error 10065") {
        return format!("Сеть недоступна: не удалось добраться до {host}.");
    }
    if error.is_connect() {
        return format!("Не удалось подключиться к {host}. {}", error_chain(error));
    }
    if error.is_body() || error.is_decode() {
        return format!("Не удалось прочитать ответ от {host}. {}", error_chain(error));
    }
    format!("Запрос к {host} не выполнен. {}", error_chain(error))
}

#[async_trait]
impl ProtocolExecutor for HttpExecutor {
    async fn execute(
        &self,
        request: &ResolvedHttpRequest,
        ctx: &ExecutionContext,
        trace: &mut TraceRecorder,
    ) -> Result<ExecutionOutcome, ExecutorError> {
        let client = self.client_for(&ctx.settings)?;
        let mut builder = client.request(method_to_reqwest(request.method), &request.url);
        if let Some(total) = optional_duration(ctx.settings.total_timeout_ms) {
            builder = builder.timeout(total);
        }

        for header in &request.headers {
            if header.enabled {
                builder = builder.header(&header.key, &header.value);
            }
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        trace.info(format!("{} {}", method_to_reqwest(request.method), request.url));
        if !ctx.settings.verify_tls {
            trace.warn("Проверка TLS-сертификата отключена в настройках");
        }

        // reqwest drives connecting itself, so from out here the request is
        // two observable spans: waiting for the response head, then reading
        // the body. Splitting out DNS, TCP and TLS needs hooks this client
        // doesn't expose — those phases stay unset until an executor that
        // drives them itself fills them in.
        let sent_at = Instant::now();
        let response = builder.send().await.map_err(|e| {
            let described = describe_error(&e, &request.url);
            trace.phase(Phase::Wait, sent_at.elapsed());
            trace.error(described.clone());
            ExecutorError::Failed(described)
        })?;
        trace.phase(Phase::Wait, sent_at.elapsed());

        let status = response.status();
        if let Some(addr) = response.remote_addr() {
            trace.set_remote_addr(addr.to_string());
        }
        trace.info(format!(
            "Получен ответ {} {} ({:?})",
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
            response.version()
        ));

        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| KeyValue {
                key: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
                enabled: true,
            })
            .collect();

        let download_at = Instant::now();
        let body = response.bytes().await.map_err(|e| {
            let described = describe_error(&e, &request.url);
            trace.phase(Phase::Download, download_at.elapsed());
            trace.error(described.clone());
            ExecutorError::Failed(described)
        })?;
        trace.phase(Phase::Download, download_at.elapsed());
        trace.info(format!("Тело ответа получено, {} байт", body.len()));

        Ok(ExecutionOutcome {
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            headers,
            body_base64: base64::engine::general_purpose::STANDARD.encode(&body),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cached_settings(executor: &HttpExecutor) -> Option<RequestSettings> {
        executor.cached.lock().unwrap().as_ref().map(|(settings, _)| settings.clone())
    }

    #[test]
    fn the_client_is_cached_per_settings_and_rebuilt_when_they_change() {
        let executor = HttpExecutor::new();
        // Every "off" value at once: 0 timeouts, no redirects, no TLS check.
        let relaxed = RequestSettings {
            verify_tls: false,
            connect_timeout_ms: 0,
            read_timeout_ms: 0,
            total_timeout_ms: 0,
            follow_redirects: false,
            max_redirects: 0,
            user_agent: "LokkiAPI test".to_string(),
        };

        executor.client_for(&relaxed).unwrap();
        assert_eq!(cached_settings(&executor).as_ref(), Some(&relaxed));

        executor.client_for(&relaxed).unwrap();
        assert_eq!(cached_settings(&executor).as_ref(), Some(&relaxed));

        let defaults = RequestSettings::default();
        executor.client_for(&defaults).unwrap();
        assert_eq!(cached_settings(&executor).as_ref(), Some(&defaults));
    }

    #[test]
    fn a_bad_url_is_reported_as_a_bad_url() {
        let error = reqwest::Client::new()
            .get("not-a-url")
            .build()
            .expect_err("relative url must not build");
        let described = describe_error(&error, "not-a-url");
        assert!(described.contains("Некорректный адрес запроса"), "{described}");
    }
}
