//! Every sentence the core shows the user, in one place.
//!
//! They are functions rather than a key/template table so each one states
//! what it needs: a message that talks about a host cannot be built without
//! one. The two wordings sit side by side in `tr!`, which is what keeps a
//! translation from drifting away from the text it mirrors.
//!
//! House style, in both languages: a sentence, and where there is a fix,
//! name it.

use crate::tr;

// --- sending -------------------------------------------------------------

/// The URL itself still has `{{variables}}` in it. Refused before sending,
/// because reqwest would otherwise report it as "relative URL without a
/// base" - true, and useless.
pub fn unresolved_url_variables(names: &str) -> String {
    tr!(
        "The request URL has unresolved variables: {names}. Check the active environment.",
        "В адресе запроса не подставлены переменные: {names}. Проверьте активное окружение.",
    )
}

/// A name that stayed a literal placeholder outside the URL. A warning on
/// the trace, not a failure: sending the placeholder through may well be
/// what the API is being tested with.
pub fn unresolved_variable(name: &str) -> String {
    tr!(
        "Unresolved variable {{{{{name}}}}}",
        "Не подставлена переменная {{{{{name}}}}}",
    )
}

pub fn request_cancelled() -> String {
    tr!("Request cancelled", "Запрос отменён")
}

pub fn decode_body_failed(error: &str) -> String {
    tr!(
        "Could not decode the response body: {error}",
        "Не удалось раскодировать тело ответа: {error}",
    )
}

pub fn read_body_file_failed(path: &str, error: &str) -> String {
    tr!(
        "Could not read file {path}: {error}",
        "Не удалось прочитать файл {path}: {error}",
    )
}

// --- HTTP transport ------------------------------------------------------

pub fn http_client_setup_failed(error: &str) -> String {
    tr!(
        "Could not set up the HTTP client: {error}",
        "Не удалось настроить HTTP-клиент: {error}",
    )
}

pub fn connect_timeout(host: &str) -> String {
    tr!(
        "Could not connect to {host}: the connect timeout ran out. Raise the timeout in settings, or check that the host is reachable.",
        "Не удалось подключиться к {host}: превышено время ожидания подключения. Увеличьте таймаут в настройках или проверьте доступность хоста.",
    )
}

pub fn response_timeout(host: &str) -> String {
    tr!(
        "Timed out waiting for a response from {host}. Raise the timeout in settings, or check that the service is answering.",
        "Превышено время ожидания ответа от {host}. Увеличьте таймаут в настройках или проверьте, отвечает ли сервис.",
    )
}

pub fn too_many_redirects(host: &str) -> String {
    tr!(
        "Too many redirects while calling {host}. The limit is set in settings.",
        "Слишком много перенаправлений при обращении к {host}. Ограничение задаётся в настройках.",
    )
}

pub fn invalid_url(url: &str) -> String {
    tr!("Malformed request URL: {url}", "Некорректный адрес запроса: {url}")
}

pub fn dns_failed(host: &str) -> String {
    tr!(
        "Could not resolve host {host}. Check the host name and your network connection.",
        "Не удалось определить адрес хоста {host}. Проверьте имя хоста и подключение к сети.",
    )
}

pub fn tls_failed(host: &str) -> String {
    tr!(
        "Could not verify the TLS certificate of {host}. If this is a test server with a self-signed certificate, turn TLS verification off in settings.",
        "Не удалось проверить TLS-сертификат {host}. Если это тестовый сервер с самоподписанным сертификатом, отключите проверку TLS в настройках.",
    )
}

pub fn connection_refused(host: &str) -> String {
    tr!(
        "Host {host} refused the connection. Check the port and that the service is running.",
        "Хост {host} отклонил подключение. Проверьте порт и что сервис запущен.",
    )
}

pub fn connection_reset(host: &str) -> String {
    tr!(
        "The server at {host} closed the connection.",
        "Соединение с {host} разорвано сервером.",
    )
}

pub fn network_unreachable(host: &str) -> String {
    tr!(
        "The network is unreachable: could not get to {host}.",
        "Сеть недоступна: не удалось добраться до {host}.",
    )
}

pub fn connect_failed(host: &str, detail: &str) -> String {
    tr!(
        "Could not connect to {host}. {detail}",
        "Не удалось подключиться к {host}. {detail}",
    )
}

pub fn read_response_failed(host: &str, detail: &str) -> String {
    tr!(
        "Could not read the response from {host}. {detail}",
        "Не удалось прочитать ответ от {host}. {detail}",
    )
}

pub fn request_failed(host: &str, detail: &str) -> String {
    tr!(
        "The request to {host} failed. {detail}",
        "Запрос к {host} не выполнен. {detail}",
    )
}

// --- trace log -----------------------------------------------------------

pub fn tls_verification_disabled() -> String {
    tr!(
        "TLS certificate verification is turned off in settings",
        "Проверка TLS-сертификата отключена в настройках",
    )
}

pub fn response_received(status: u16, reason: &str, version: &str) -> String {
    tr!(
        "Response received: {status} {reason} ({version})",
        "Получен ответ {status} {reason} ({version})",
    )
}

pub fn body_received(bytes: usize) -> String {
    tr!("Response body received, {bytes} bytes", "Тело ответа получено, {bytes} байт")
}

// --- workspace and collections -------------------------------------------

pub fn not_a_collection(path: &str) -> String {
    tr!(
        "{path} is not a LokkiAPI collection, deletion cancelled.",
        "{path} - не коллекция LokkiAPI, удаление отменено.",
    )
}

pub fn cannot_move_folder_into_itself() -> String {
    tr!(
        "A folder cannot be moved inside itself.",
        "Нельзя переместить папку внутрь самой себя.",
    )
}

pub fn no_workspace_in_folder(path: &str) -> String {
    tr!(
        "Folder {path} holds no LokkiAPI workspace. Create a new one.",
        "В папке {path} нет пространства LokkiAPI. Создайте новое.",
    )
}

pub fn workspace_already_exists(path: &str) -> String {
    tr!(
        "Folder {path} already holds a workspace - open it instead.",
        "В папке {path} уже есть пространство - откройте его.",
    )
}

pub fn workspace_folder_not_empty(path: &str) -> String {
    tr!(
        "Folder {path} is not empty. Pick an empty one - a workspace fills the folder with its own files.",
        "Папка {path} не пуста. Выберите пустую - пространство размещает в папке свои файлы.",
    )
}

// --- importing a collection ----------------------------------------------

pub fn read_spec_failed(path: &str, error: &str) -> String {
    tr!(
        "Could not read the specification file {path}: {error}",
        "Не удалось прочитать файл спецификации {path}: {error}",
    )
}

pub fn download_spec_failed(url: &str, detail: &str) -> String {
    tr!(
        "Could not download the specification from {url}. {detail}",
        "Не удалось скачать спецификацию с {url}. {detail}",
    )
}

pub fn download_spec_status(url: &str, status: u16) -> String {
    tr!(
        "{url} answered {status}, so the specification could not be downloaded.",
        "{url} ответил {status}, спецификацию скачать не удалось.",
    )
}

pub fn spec_not_text() -> String {
    tr!(
        "What was downloaded is not text: a specification is JSON or YAML.",
        "Скачано не текстовое содержимое: спецификация - это JSON или YAML.",
    )
}

pub fn spec_parse_failed(error: &str) -> String {
    tr!(
        "Could not parse the specification: {error}",
        "Не удалось разобрать спецификацию: {error}",
    )
}

pub fn not_an_openapi_spec() -> String {
    tr!(
        "This is not an OpenAPI specification: it has neither an `openapi` field nor a `swagger` one.",
        "Это не спецификация OpenAPI: в файле нет ни поля `openapi`, ни поля `swagger`.",
    )
}

pub fn spec_has_no_operations() -> String {
    tr!(
        "The specification describes no operations, so there is nothing to import.",
        "В спецификации нет ни одной операции, импортировать нечего.",
    )
}

/// Used when the specification has no `info.title` to name the collection
/// after.
pub fn imported_collection_name() -> String {
    tr!("Imported API", "Импортированный API")
}

/// Used when a server entry has no description and no host to name its
/// environment after.
pub fn imported_environment_name() -> String {
    tr!("Imported", "Импорт")
}

// --- import warnings -----------------------------------------------------
//
// Each one is something the user may have to finish by hand, so they are
// collected during the import and shown when it is done.

pub fn import_skipped_method(method: &str, path: &str) -> String {
    tr!(
        "{method} {path}: this HTTP method is not supported, the operation was skipped.",
        "{method} {path}: этот HTTP-метод не поддерживается, операция пропущена.",
    )
}

pub fn import_skipped_body(name: &str, media_type: &str) -> String {
    tr!(
        "“{name}”: a {media_type} body cannot be built, the request was imported without one.",
        "«{name}»: тело {media_type} собрать нельзя, запрос импортирован без тела.",
    )
}

pub fn import_multipart_body(name: &str) -> String {
    tr!(
        "“{name}”: a multipart body was imported as form fields - attach the files by hand.",
        "«{name}»: multipart-тело импортировано как поля формы - файлы приложите вручную.",
    )
}

pub fn import_oauth_as_bearer(scheme: &str) -> String {
    tr!(
        "Authorization scheme “{scheme}” is OAuth2: requests are sent with a bearer token, put it in the {{{{token}}}} variable.",
        "Схема авторизации «{scheme}» - OAuth2: запросы отправляются с bearer-токеном, впишите его в переменную {{{{token}}}}.",
    )
}

pub fn import_server_variable_without_default(name: &str) -> String {
    tr!(
        "Server variable {{{name}}} has no default value and was left in the address as it is.",
        "У серверной переменной {{{name}}} нет значения по умолчанию, она осталась в адресе как есть.",
    )
}

pub fn import_relative_server(url: &str) -> String {
    tr!(
        "The specification gives a relative server address ({url}) - write the full one into the baseUrl variable.",
        "В спецификации указан относительный адрес сервера ({url}) - впишите полный в переменную baseUrl.",
    )
}

pub fn import_no_server() -> String {
    tr!(
        "The specification does not say where the API lives - write its address into the baseUrl variable.",
        "В спецификации не указан адрес API - впишите его в переменную baseUrl.",
    )
}

pub fn import_path_parameters_kept() -> String {
    tr!(
        "Path parameters are left as the specification writes them ({{petId}} and the like) - substitute values before sending.",
        "Параметры пути оставлены как в спецификации (вида {{petId}}) - подставьте значения перед отправкой.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Language;
    use crate::i18n::with_language;

    #[test]
    fn every_message_speaks_the_active_language() {
        assert!(with_language(Language::En, || dns_failed("example.com")).starts_with("Could not resolve"));
        assert!(with_language(Language::Ru, || dns_failed("example.com")).starts_with("Не удалось"));
    }

    /// The variable name is wrapped in the app's own `{{…}}` syntax, which
    /// is four escaped braces inside a format string - easy to get wrong in
    /// one language and not the other.
    #[test]
    fn unresolved_variable_keeps_the_placeholder_syntax() {
        assert_eq!(with_language(Language::En, || unresolved_variable("baseUrl")), "Unresolved variable {{baseUrl}}");
        assert_eq!(
            with_language(Language::Ru, || unresolved_variable("baseUrl")),
            "Не подставлена переменная {{baseUrl}}"
        );
    }
}
