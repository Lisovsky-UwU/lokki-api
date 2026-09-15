//! Reading an OpenAPI 3.x or Swagger 2.0 document into an `ImportPlan`.
//!
//! The document is walked as a generic `serde_json::Value` rather than
//! deserialized into typed structs: specifications in the wild routinely
//! carry vendor extensions, half-filled objects and fields from a
//! neighbouring version, and a typed model would refuse the whole file over
//! any one of them. Everything here is best-effort instead - what cannot be
//! read is skipped with a warning, and what is left still becomes a working
//! collection.
//!
//! The two dialects differ in a handful of places (where the servers are,
//! where the schemas are, how a body is declared), which is little enough
//! to be one walk with a `Dialect` switch rather than two parsers.

use super::{ImportPlan, ImportedEnvironment, ImportedFolder};
use crate::domain::{
    AuthSpec, BodySpec, HttpMethod, HttpRequestSpec, Id, KeyValue, RequestFile, TextFormat, Variable,
};
use crate::error::{AppError, AppResult};
use crate::i18n::messages;
use serde_json::{Map, Value};

/// How deep a schema is unfolded when building an example body. Deep enough
/// for real payloads, shallow enough that a mutually recursive pair of
/// schemas cannot make the example enormous.
const MAX_SCHEMA_DEPTH: usize = 12;

/// A request name comes from a free-text `summary`, which is sometimes a
/// paragraph - and which also becomes a file name.
const MAX_NAME_LEN: usize = 80;

/// The variable every imported request builds its URL on.
const BASE_URL_VARIABLE: &str = "baseUrl";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialect {
    /// OpenAPI 3.0 and 3.1.
    V3,
    /// Swagger 2.0.
    V2,
}

pub fn parse(text: &str) -> AppResult<ImportPlan> {
    parse_with_origin(text, None)
}

/// `origin` is where the document itself came from. A specification is
/// allowed to give a relative server address (`/api/v3`), which means "the
/// host that served this document" - so it can only be resolved when the
/// document arrived over the network.
pub fn parse_with_origin(text: &str, origin: Option<&str>) -> AppResult<ImportPlan> {
    let root = parse_document(text)?;
    let dialect = detect_dialect(&root)?;
    Ok(Importer {
        root: &root,
        dialect,
        origin: origin.and_then(|url| reqwest::Url::parse(url).ok()),
        warnings: Vec::new(),
        auth_variables: Vec::new(),
        path_parameters_seen: false,
    }
    .run())
}

/// JSON is valid YAML, so one parser could read both - but serde_json says
/// far more about a broken JSON file, and JSON is the half people hand-edit.
fn parse_document(text: &str) -> AppResult<Value> {
    if text.trim_start().starts_with('{') {
        return serde_json::from_str::<Value>(text)
            .map_err(|e| AppError::Message(messages::spec_parse_failed(&e.to_string())));
    }
    serde_yaml_ng::from_str::<Value>(text)
        .map_err(|e| AppError::Message(messages::spec_parse_failed(&e.to_string())))
}

fn detect_dialect(root: &Value) -> AppResult<Dialect> {
    if str_at(root, "openapi").is_some() {
        return Ok(Dialect::V3);
    }
    if str_at(root, "swagger").is_some() {
        return Ok(Dialect::V2);
    }
    Err(AppError::Message(messages::not_an_openapi_spec()))
}

// --- reading a generic document ------------------------------------------

fn str_at<'v>(value: &'v Value, key: &str) -> Option<&'v str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn bool_at(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn array_at<'v>(value: &'v Value, key: &str) -> &'v [Value] {
    value.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn object_at<'v>(value: &'v Value, key: &str) -> Option<&'v Map<String, Value>> {
    value.get(key).and_then(Value::as_object)
}

/// A value that can stand in a header, a query parameter or a form field.
/// Objects and arrays have no one obvious spelling there, so they are left
/// out rather than guessed at.
fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// First line, trimmed and capped: `summary` is free text and also ends up
/// as a file name.
fn short_name(raw: &str) -> String {
    let first_line = raw.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= MAX_NAME_LEN {
        return first_line.to_string();
    }
    first_line
        .chars()
        .take(MAX_NAME_LEN)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn method_label(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

/// `None` marks a method the app has no `HttpMethod` for: it is listed so
/// an operation written with it is reported rather than silently dropped.
const METHODS: &[(&str, Option<HttpMethod>)] = &[
    ("get", Some(HttpMethod::Get)),
    ("post", Some(HttpMethod::Post)),
    ("put", Some(HttpMethod::Put)),
    ("patch", Some(HttpMethod::Patch)),
    ("delete", Some(HttpMethod::Delete)),
    ("head", Some(HttpMethod::Head)),
    ("options", Some(HttpMethod::Options)),
    ("trace", None),
];

// --- authorization --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiKeyIn {
    Header,
    Query,
    Cookie,
}

/// What an operation's security requirement turns into. The token, user
/// name and key never come from the document - they are variables the user
/// fills in once, in the environment the import creates.
#[derive(Debug, Clone)]
enum AuthKind {
    Bearer,
    Basic,
    ApiKey {
        location: ApiKeyIn,
        /// The header, query or cookie name the API expects.
        name: String,
        /// The environment variable holding the key.
        variable: String,
    },
}

const TOKEN_VARIABLE: &str = "token";
const USERNAME_VARIABLE: &str = "username";
const PASSWORD_VARIABLE: &str = "password";

/// Variable names are referenced as `{{name}}`, which only accepts these
/// characters (see `interpolate`).
fn variable_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "apiKey".to_string()
    } else {
        cleaned
    }
}

fn reference(name: &str) -> String {
    format!("{{{{{name}}}}}")
}

// --- the walk -------------------------------------------------------------

struct Importer<'a> {
    root: &'a Value,
    dialect: Dialect,
    /// Where the document was fetched from, when it was fetched at all.
    origin: Option<reqwest::Url>,
    warnings: Vec<String>,
    /// Variables the operations' authorization referred to, in the order
    /// they were met, paired with whether the value is a secret. Every
    /// environment gets its own copy: a `Variable`'s id is the key its
    /// secret is stored under, so two environments sharing one id would
    /// share whatever value was typed into either.
    auth_variables: Vec<(String, bool)>,
    path_parameters_seen: bool,
}

impl<'a> Importer<'a> {
    fn run(mut self) -> ImportPlan {
        let root = self.root;
        let mut folders: Vec<ImportedFolder> = self
            .declared_tags()
            .into_iter()
            .map(|name| ImportedFolder {
                name,
                requests: Vec::new(),
            })
            .collect();
        let mut root_requests = Vec::new();

        let paths: Vec<String> = object_at(root, "paths")
            .map(|paths| paths.keys().cloned().collect())
            .unwrap_or_default();
        for path in &paths {
            let Some(path_item) = root.get("paths").and_then(|paths| paths.get(path)) else {
                continue;
            };
            let path_item = self.deref(path_item);
            for (key, method) in METHODS {
                let Some(operation) = path_item.get(*key) else { continue };
                if !operation.is_object() {
                    continue;
                }
                let Some(method) = *method else {
                    self.warnings
                        .push(messages::import_skipped_method(&key.to_uppercase(), path));
                    continue;
                };
                let request = self.operation(path, method, path_item, operation);
                match self.tag_of(operation) {
                    Some(tag) => match folders.iter_mut().find(|folder| folder.name == tag) {
                        Some(folder) => folder.requests.push(request),
                        // A tag an operation uses but the document never
                        // declared in its own `tags` list.
                        None => folders.push(ImportedFolder {
                            name: tag,
                            requests: vec![request],
                        }),
                    },
                    None => root_requests.push(request),
                }
            }
        }

        // Declared tags are documentation and are routinely wider than what
        // the paths actually use; an empty folder would only be noise.
        folders.retain(|folder| !folder.requests.is_empty());

        if self.path_parameters_seen {
            self.warnings.push(messages::import_path_parameters_kept());
        }

        let collection_name = self.collection_name();
        // After the walk, not before: the environments carry the variables
        // the operations' authorization turned out to need.
        let environments = self.environments();
        ImportPlan {
            collection_name,
            environments,
            folders,
            requests: root_requests,
            warnings: self.warnings,
        }
    }

    fn collection_name(&self) -> String {
        self.root
            .get("info")
            .and_then(|info| str_at(info, "title"))
            .map(short_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(messages::imported_collection_name)
    }

    /// The document's own `tags` list, which is the order the API's author
    /// chose - worth keeping, since it decides the order of the folders.
    fn declared_tags(&self) -> Vec<String> {
        array_at(self.root, "tags")
            .iter()
            .filter_map(|tag| str_at(tag, "name"))
            .map(short_name)
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn tag_of(&self, operation: &Value) -> Option<String> {
        array_at(operation, "tags")
            .iter()
            .filter_map(Value::as_str)
            .map(short_name)
            .find(|name| !name.is_empty())
    }

    /// Follows a `$ref` to the object it names. External references (another
    /// file, a URL) are out of scope - an import reads one document - and
    /// are left as they are, which every caller reads as an empty object.
    fn deref(&self, value: &'a Value) -> &'a Value {
        let mut current = value;
        // A `$ref` may point at another `$ref`; the cap is what keeps a
        // cycle from hanging the import.
        for _ in 0..8 {
            let Some(reference) = str_at(current, "$ref") else { return current };
            let Some(target) = reference.strip_prefix('#').and_then(|pointer| self.root.pointer(pointer)) else {
                return current;
            };
            current = target;
        }
        current
    }

    // --- one operation ----------------------------------------------------

    fn operation(&mut self, path: &str, method: HttpMethod, path_item: &'a Value, operation: &'a Value) -> RequestFile {
        let name = self.operation_name(path, method, operation);
        let parameters = self.parameters(path_item, operation);
        let mut query_params = Vec::new();
        let mut headers = Vec::new();
        let mut cookies: Vec<(String, String)> = Vec::new();
        let mut body = BodySpec::None;
        let mut has_form_data = false;

        if path.contains('{') {
            self.path_parameters_seen = true;
        }

        for parameter in &parameters {
            let Some(parameter_name) = str_at(parameter, "name") else { continue };
            let required = bool_at(parameter, "required");
            let value = self.parameter_value(parameter);
            match str_at(parameter, "in").unwrap_or("query") {
                // Already spelled out in the URL, the way the specification
                // writes it.
                "path" => {}
                "header" => headers.push(KeyValue {
                    key: parameter_name.to_string(),
                    value,
                    enabled: required,
                }),
                "cookie" => cookies.push((parameter_name.to_string(), value)),
                // Swagger 2.0 declares the body among the parameters.
                "body" => body = self.v2_body(parameter, operation, &mut headers),
                "formData" => has_form_data = true,
                _ => query_params.push(KeyValue {
                    key: parameter_name.to_string(),
                    value,
                    enabled: required,
                }),
            }
        }

        if has_form_data {
            body = self.v2_form_body(operation, &mut headers, &parameters);
        }
        if self.dialect == Dialect::V3 {
            body = self.v3_body(operation, &name, &mut headers);
        }

        let auth = match self.operation_auth(operation) {
            Some(AuthKind::Bearer) => {
                self.register_variable(TOKEN_VARIABLE, true);
                AuthSpec::Bearer {
                    token: reference(TOKEN_VARIABLE),
                }
            }
            Some(AuthKind::Basic) => {
                self.register_variable(USERNAME_VARIABLE, false);
                self.register_variable(PASSWORD_VARIABLE, true);
                AuthSpec::Basic {
                    username: reference(USERNAME_VARIABLE),
                    password: reference(PASSWORD_VARIABLE),
                }
            }
            Some(AuthKind::ApiKey {
                location,
                name,
                variable,
            }) => {
                self.register_variable(&variable, true);
                let value = reference(&variable);
                match location {
                    ApiKeyIn::Header => headers.push(KeyValue {
                        key: name,
                        value,
                        enabled: true,
                    }),
                    ApiKeyIn::Query => query_params.push(KeyValue {
                        key: name,
                        value,
                        enabled: true,
                    }),
                    ApiKeyIn::Cookie => cookies.push((name, value)),
                }
                AuthSpec::None
            }
            None => AuthSpec::None,
        };

        // Cookies have no table of their own in the request editor, and on
        // the wire they are one header anyway.
        if !cookies.is_empty() {
            let value = cookies
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("; ");
            headers.push(KeyValue {
                key: "Cookie".to_string(),
                value,
                enabled: true,
            });
        }

        let mut request = RequestFile::new_http(name, 0, method);
        request.http = Some(HttpRequestSpec {
            method,
            url: format!("{}{}", reference(BASE_URL_VARIABLE), path),
            query_params,
            headers,
            auth,
            body,
        });
        request
    }

    fn operation_name(&self, path: &str, method: HttpMethod, operation: &Value) -> String {
        str_at(operation, "summary")
            .or_else(|| str_at(operation, "operationId"))
            .map(short_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("{} {}", method_label(method), path))
    }

    /// The parameters declared for the whole path plus the operation's own,
    /// the operation winning where both describe the same one - which is
    /// what the specification means by overriding.
    fn parameters(&self, path_item: &'a Value, operation: &'a Value) -> Vec<&'a Value> {
        let mut out: Vec<&'a Value> = Vec::new();
        for source in [path_item, operation] {
            for parameter in array_at(source, "parameters") {
                let parameter = self.deref(parameter);
                let identity = (str_at(parameter, "name"), str_at(parameter, "in"));
                match out
                    .iter_mut()
                    .find(|existing| (str_at(existing, "name"), str_at(existing, "in")) == identity)
                {
                    Some(slot) => *slot = parameter,
                    None => out.push(parameter),
                }
            }
        }
        out
    }

    /// A value to start from, never an invented one: an example the author
    /// wrote, a default, or the first of an enum. Anything else is left
    /// empty for the user to fill in.
    fn parameter_value(&self, parameter: &'a Value) -> String {
        // OpenAPI 3 keeps the type under `schema`; Swagger 2.0 puts it on
        // the parameter itself.
        let schema = parameter.get("schema").map(|schema| self.deref(schema));
        let first_example = object_at(parameter, "examples")
            .and_then(|examples| examples.values().next())
            .and_then(|example| example.get("value"));

        [
            parameter.get("example"),
            first_example,
            parameter.get("default"),
            schema.and_then(|schema| schema.get("example")),
            schema.and_then(|schema| schema.get("default")),
            array_at(parameter, "enum").first(),
            schema.map(|schema| array_at(schema, "enum")).unwrap_or(&[]).first(),
        ]
        .into_iter()
        .flatten()
        .find_map(scalar_text)
        .unwrap_or_default()
    }

    // --- request bodies ---------------------------------------------------

    fn v3_body(&mut self, operation: &'a Value, name: &str, headers: &mut Vec<KeyValue>) -> BodySpec {
        let Some(request_body) = operation.get("requestBody").map(|body| self.deref(body)) else {
            return BodySpec::None;
        };
        let Some(content) = object_at(request_body, "content") else {
            return BodySpec::None;
        };
        let Some((media_type, media)) = pick_media_type(content) else {
            return BodySpec::None;
        };
        let media_type = media_type.to_string();
        let schema = media.get("schema").map(|schema| self.deref(schema));
        let example = media
            .get("example")
            .cloned()
            .or_else(|| {
                object_at(media, "examples")
                    .and_then(|examples| examples.values().next())
                    .and_then(|example| example.get("value").cloned())
            })
            .or_else(|| schema.map(|schema| self.schema_example(schema, 0, &mut Vec::new())));

        self.body_from(&media_type, schema, example, name, headers)
    }

    /// Swagger 2.0: the body is a parameter carrying a schema, and its type
    /// comes from `consumes`.
    fn v2_body(&mut self, parameter: &'a Value, operation: &'a Value, headers: &mut Vec<KeyValue>) -> BodySpec {
        let media_type = self.v2_consumes(operation, "application/json");
        let schema = parameter.get("schema").map(|schema| self.deref(schema));
        let example = schema.map(|schema| self.schema_example(schema, 0, &mut Vec::new()));
        let name = str_at(parameter, "name").unwrap_or_default().to_string();
        self.body_from(&media_type, schema, example, &name, headers)
    }

    /// Swagger 2.0: `formData` parameters are the fields of a form body.
    fn v2_form_body(
        &mut self,
        operation: &'a Value,
        headers: &mut Vec<KeyValue>,
        parameters: &[&'a Value],
    ) -> BodySpec {
        let media_type = self.v2_consumes(operation, "application/x-www-form-urlencoded");
        let fields: Vec<KeyValue> = parameters
            .iter()
            .filter(|parameter| str_at(parameter, "in") == Some("formData"))
            .filter_map(|parameter| {
                str_at(parameter, "name").map(|name| KeyValue {
                    key: name.to_string(),
                    value: self.parameter_value(parameter),
                    enabled: true,
                })
            })
            .collect();
        set_content_type(headers, &media_type);
        BodySpec::Form { fields }
    }

    fn v2_consumes(&self, operation: &'a Value, fallback: &str) -> String {
        array_at(operation, "consumes")
            .first()
            .or_else(|| array_at(self.root, "consumes").first())
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    }

    fn body_from(
        &mut self,
        media_type: &str,
        schema: Option<&'a Value>,
        example: Option<Value>,
        name: &str,
        headers: &mut Vec<KeyValue>,
    ) -> BodySpec {
        match classify_media_type(media_type) {
            MediaKind::Text(format) => {
                set_content_type(headers, media_type);
                BodySpec::Text {
                    content: render_body(example.unwrap_or(Value::Null), format),
                    format,
                }
            }
            MediaKind::Multipart => {
                // Sent as an ordinary form: a multipart part can be a file,
                // and which file that is only the user knows.
                self.warnings.push(messages::import_multipart_body(name));
                set_content_type(headers, media_type);
                BodySpec::Form {
                    fields: self.form_fields(schema),
                }
            }
            MediaKind::Form => {
                set_content_type(headers, media_type);
                BodySpec::Form {
                    fields: self.form_fields(schema),
                }
            }
            MediaKind::Unsupported => {
                self.warnings.push(messages::import_skipped_body(name, media_type));
                BodySpec::None
            }
        }
    }

    fn form_fields(&self, schema: Option<&'a Value>) -> Vec<KeyValue> {
        let Some(properties) = schema.and_then(|schema| object_at(schema, "properties")) else {
            return Vec::new();
        };
        properties
            .iter()
            .map(|(key, property)| KeyValue {
                key: key.clone(),
                value: scalar_text(&self.schema_example(property, 0, &mut Vec::new())).unwrap_or_default(),
                enabled: true,
            })
            .collect()
    }

    /// An example payload built from a schema: the author's own `example`
    /// wherever there is one, and an empty value of the right type
    /// everywhere else. Nothing is invented - a made-up id in a body is
    /// worse than a blank one, because it looks like a real value.
    fn schema_example(&self, schema: &'a Value, depth: usize, seen: &mut Vec<&'a str>) -> Value {
        if depth >= MAX_SCHEMA_DEPTH {
            return Value::Null;
        }
        if let Some(reference) = str_at(schema, "$ref") {
            // A schema that refers to itself (a tree node, a linked list)
            // would otherwise unfold all the way to the depth cap for no
            // gain.
            if seen.contains(&reference) {
                return Value::Null;
            }
            seen.push(reference);
            let example = self.schema_example(self.deref(schema), depth + 1, seen);
            seen.pop();
            return example;
        }
        if let Some(example) = schema.get("example") {
            return example.clone();
        }
        if let Some(default) = schema.get("default") {
            return default.clone();
        }
        if let Some(first) = array_at(schema, "enum").first() {
            return first.clone();
        }
        if !array_at(schema, "allOf").is_empty() {
            let mut merged = Map::new();
            for part in array_at(schema, "allOf") {
                if let Value::Object(fields) = self.schema_example(part, depth + 1, seen) {
                    merged.extend(fields);
                }
            }
            return Value::Object(merged);
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(first) = array_at(schema, key).first() {
                return self.schema_example(first, depth + 1, seen);
            }
        }
        if let Some(properties) = object_at(schema, "properties") {
            let fields = properties
                .iter()
                .map(|(key, property)| (key.clone(), self.schema_example(property, depth + 1, seen)))
                .collect();
            return Value::Object(fields);
        }
        match schema_type(schema) {
            Some("object") => Value::Object(Map::new()),
            Some("array") => match schema.get("items") {
                Some(items) => Value::Array(vec![self.schema_example(items, depth + 1, seen)]),
                None => Value::Array(Vec::new()),
            },
            Some("string") => Value::String(String::new()),
            Some("integer") | Some("number") => Value::Number(0.into()),
            Some("boolean") => Value::Bool(false),
            _ => Value::Null,
        }
    }

    // --- authorization ----------------------------------------------------

    fn security_schemes(&self) -> Option<&'a Map<String, Value>> {
        match self.dialect {
            Dialect::V3 => self
                .root
                .get("components")
                .and_then(|components| components.get("securitySchemes"))
                .and_then(Value::as_object),
            Dialect::V2 => object_at(self.root, "securityDefinitions"),
        }
    }

    /// The first scheme of the first requirement: a request carries one set
    /// of credentials, and choosing between several alternatives is the
    /// user's call, not the importer's.
    fn operation_auth(&mut self, operation: &'a Value) -> Option<AuthKind> {
        let requirements = match operation.get("security").and_then(Value::as_array) {
            // An empty `security` on an operation means "nothing is needed
            // here", and must not fall back to the document's default.
            Some(own) => own.as_slice(),
            None => array_at(self.root, "security"),
        };
        let scheme_name = requirements
            .iter()
            .filter_map(Value::as_object)
            .flat_map(|requirement| requirement.keys())
            .next()?;
        let definition = self.security_schemes()?.get(scheme_name)?;

        let kind = match str_at(definition, "type")?.to_ascii_lowercase().as_str() {
            "http" => match str_at(definition, "scheme")
                .unwrap_or("bearer")
                .to_ascii_lowercase()
                .as_str()
            {
                "basic" => AuthKind::Basic,
                _ => AuthKind::Bearer,
            },
            // Swagger 2.0 spells these out as types of their own.
            "basic" => AuthKind::Basic,
            "apikey" => AuthKind::ApiKey {
                location: match str_at(definition, "in").unwrap_or("header").to_ascii_lowercase().as_str() {
                    "query" => ApiKeyIn::Query,
                    "cookie" => ApiKeyIn::Cookie,
                    _ => ApiKeyIn::Header,
                },
                name: str_at(definition, "name").unwrap_or("X-API-Key").to_string(),
                variable: variable_name(scheme_name),
            },
            // The app runs no OAuth2 flow: the request carries whatever
            // token the user pastes in, which is exactly a bearer.
            "oauth2" | "openidconnect" => {
                let warning = messages::import_oauth_as_bearer(scheme_name);
                if !self.warnings.contains(&warning) {
                    self.warnings.push(warning);
                }
                AuthKind::Bearer
            }
            _ => return None,
        };
        Some(kind)
    }

    fn register_variable(&mut self, key: &str, secret: bool) {
        if !self.auth_variables.iter().any(|(existing, _)| existing == key) {
            self.auth_variables.push((key.to_string(), secret));
        }
    }

    // --- environments -----------------------------------------------------

    /// One environment per server the document lists, so switching between
    /// staging and production is the switch the app already has. They carry
    /// the same variable names, which is what makes that switch work.
    fn environments(&mut self) -> Vec<ImportedEnvironment> {
        let mut servers = match self.dialect {
            Dialect::V3 => self.v3_servers(),
            Dialect::V2 => self.v2_servers(),
        };
        if servers.is_empty() {
            // Nothing said where the API lives. The environment is still
            // created, with a blank `baseUrl` row waiting to be filled in -
            // otherwise every imported request would point at nothing and
            // there would be nowhere obvious to say where it should point.
            self.warnings.push(messages::import_no_server());
            servers.push((messages::imported_environment_name(), String::new()));
        }

        servers
            .into_iter()
            .map(|(name, base_url)| {
                let mut variables = vec![variable(BASE_URL_VARIABLE, &base_url, false)];
                for (key, secret) in &self.auth_variables {
                    // A secret's value never goes into the environment file
                    // (see domain::environment): it is typed in once and
                    // kept in the workspace's local secret store.
                    variables.push(variable(key, "", *secret));
                }
                ImportedEnvironment { name, variables }
            })
            .collect()
    }

    fn v3_servers(&mut self) -> Vec<(String, String)> {
        let servers = array_at(self.root, "servers").to_vec();
        servers
            .iter()
            .filter_map(|server| {
                let url = str_at(server, "url")?.to_string();
                let url = self.expand_server_variables(&url, server.get("variables"));
                let url = self.absolute_url(&url);
                let name = str_at(server, "description")
                    .map(short_name)
                    .filter(|name| !name.is_empty())
                    .or_else(|| host_of(&url))
                    .unwrap_or_else(messages::imported_environment_name);
                Some((name, trim_base_url(&url)))
            })
            .collect()
    }

    /// A server address may be relative (`/api/v3`), which resolves only
    /// against wherever the document came from - and stays as it is when it
    /// came from a file, since nothing else says what host it means.
    fn absolute_url(&mut self, url: &str) -> String {
        if reqwest::Url::parse(url).is_ok() {
            return url.to_string();
        }
        match self.origin.as_ref().and_then(|origin| origin.join(url).ok()) {
            Some(resolved) => resolved.to_string(),
            None => {
                self.warnings.push(messages::import_relative_server(url));
                url.to_string()
            }
        }
    }

    /// Swagger 2.0 spells the server out in three fields instead of a list.
    fn v2_servers(&self) -> Vec<(String, String)> {
        // The document may leave the host out, which means the one that
        // served it.
        let host = str_at(self.root, "host")
            .map(str::to_string)
            .or_else(|| self.origin.as_ref().and_then(|origin| origin.host_str().map(str::to_string)));
        let Some(host) = host else {
            return Vec::new();
        };
        let base_path = str_at(self.root, "basePath").unwrap_or("");
        let schemes: Vec<&str> = array_at(self.root, "schemes").iter().filter_map(Value::as_str).collect();
        // HTTPS whenever the API offers it at all: an imported request
        // should not be the one that sends a token in the clear.
        let scheme = if schemes.iter().any(|s| s.eq_ignore_ascii_case("https")) {
            "https"
        } else {
            schemes.first().copied().unwrap_or("https")
        };
        vec![(host.to_string(), trim_base_url(&format!("{scheme}://{host}{base_path}")))]
    }

    /// Substitutes an OpenAPI 3 server template (`https://{region}.api.com`)
    /// with the defaults the document gives. A variable without one is left
    /// in place: it is a real choice the user has to make, and a guess would
    /// send the request somewhere they never picked.
    fn expand_server_variables(&mut self, url: &str, variables: Option<&Value>) -> String {
        if !url.contains('{') {
            return url.to_string();
        }
        let mut out = url.to_string();
        for name in placeholder_names(url) {
            let value = variables
                .and_then(|variables| variables.get(&name))
                .and_then(|definition| {
                    definition
                        .get("default")
                        .or_else(|| array_at(definition, "enum").first())
                })
                .and_then(scalar_text);
            match value {
                Some(value) => out = out.replace(&format!("{{{name}}}"), &value),
                None => self
                    .warnings
                    .push(messages::import_server_variable_without_default(&name)),
            }
        }
        out
    }
}

fn variable(key: &str, value: &str, secret: bool) -> Variable {
    Variable {
        id: Id::new(),
        key: key.to_string(),
        value: value.to_string(),
        enabled: true,
        secret,
    }
}

/// The `{name}` placeholders a server URL still carries.
fn placeholder_names(url: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut rest = url;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else { break };
        let name = &rest[start + 1..start + end];
        if !name.is_empty() && !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
        rest = &rest[start + end + 1..];
    }
    names
}

/// The request URL is `{{baseUrl}}` followed by a path that starts with
/// `/`, so a base that ends with one would double it.
fn trim_base_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        url.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn host_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
}

/// OpenAPI 3.1 allows `type` to be a list (`["string", "null"]`); the
/// nullable half says nothing about what an example should look like.
fn schema_type(schema: &Value) -> Option<&str> {
    match schema.get("type") {
        Some(Value::String(name)) => Some(name.as_str()),
        Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).find(|name| *name != "null"),
        _ => None,
    }
}

// --- media types ----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Text(TextFormat),
    Form,
    Multipart,
    Unsupported,
}

fn classify_media_type(media_type: &str) -> MediaKind {
    let base = media_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    match base.as_str() {
        "application/x-www-form-urlencoded" => MediaKind::Form,
        "application/json" | "text/json" => MediaKind::Text(TextFormat::Json),
        "application/xml" | "text/xml" => MediaKind::Text(TextFormat::Xml),
        "application/yaml" | "text/yaml" | "application/x-yaml" => MediaKind::Text(TextFormat::Yaml),
        "application/edn" => MediaKind::Text(TextFormat::Edn),
        "text/html" => MediaKind::Text(TextFormat::Html),
        "text/css" => MediaKind::Text(TextFormat::Css),
        "application/javascript" | "text/javascript" => MediaKind::Text(TextFormat::Javascript),
        other if other.starts_with("multipart/") => MediaKind::Multipart,
        // Vendor types (`application/vnd.api+json`) are read by their
        // suffix, which is what the suffix is there for.
        other if other.ends_with("+json") => MediaKind::Text(TextFormat::Json),
        other if other.ends_with("+xml") => MediaKind::Text(TextFormat::Xml),
        other if other.ends_with("+yaml") => MediaKind::Text(TextFormat::Yaml),
        other if other.starts_with("text/") => MediaKind::Text(TextFormat::Plain),
        _ => MediaKind::Unsupported,
    }
}

/// An endpoint that accepts several types gets the one that is easiest to
/// edit by hand, JSON first.
fn pick_media_type(content: &Map<String, Value>) -> Option<(&str, &Value)> {
    let rank = |media_type: &str| match classify_media_type(media_type) {
        MediaKind::Text(TextFormat::Json) => 0,
        MediaKind::Form => 1,
        MediaKind::Text(TextFormat::Xml) => 2,
        MediaKind::Text(TextFormat::Yaml) => 3,
        MediaKind::Text(_) => 4,
        MediaKind::Multipart => 5,
        MediaKind::Unsupported => 6,
    };
    content
        .iter()
        .min_by_key(|(media_type, _)| rank(media_type))
        .map(|(media_type, media)| (media_type.as_str(), media))
}

/// The type the specification declared, written out as a header rather than
/// left to the body format's default: the endpoint named it, and a vendor
/// type (`application/vnd.api+json`) is not something the default knows.
fn set_content_type(headers: &mut Vec<KeyValue>, media_type: &str) {
    if headers.iter().any(|header| header.key.eq_ignore_ascii_case("content-type")) {
        return;
    }
    headers.push(KeyValue {
        key: "Content-Type".to_string(),
        value: media_type.to_string(),
        enabled: true,
    });
}

fn render_body(example: Value, format: TextFormat) -> String {
    // An example that is already text (an XML document, a plain-text
    // payload) is used as it stands; anything else is a structure, and is
    // written out in the format the endpoint asked for.
    if let Value::String(text) = &example {
        if format != TextFormat::Json {
            return text.clone();
        }
    }
    match format {
        TextFormat::Yaml => serde_yaml_ng::to_string(&example).unwrap_or_default(),
        _ => serde_json::to_string_pretty(&example).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Language;
    use crate::i18n::with_language;

    const PETSTORE_V3: &str = r##"{
      "openapi": "3.0.3",
      "info": { "title": "Petstore" },
      "servers": [
        { "url": "https://api.example.com/v1/", "description": "Production" },
        { "url": "https://{env}.example.com", "variables": { "env": { "default": "staging" } } }
      ],
      "tags": [{ "name": "pets" }],
      "security": [{ "bearerAuth": [] }],
      "components": {
        "securitySchemes": { "bearerAuth": { "type": "http", "scheme": "bearer" } },
        "schemas": {
          "Pet": {
            "type": "object",
            "properties": {
              "id": { "type": "integer" },
              "name": { "type": "string", "example": "Rex" },
              "tags": { "type": "array", "items": { "type": "string" } }
            }
          }
        }
      },
      "paths": {
        "/pets": {
          "get": {
            "tags": ["pets"],
            "summary": "List pets",
            "parameters": [
              { "name": "limit", "in": "query", "required": true, "schema": { "type": "integer", "default": 20 } },
              { "name": "X-Trace", "in": "header", "schema": { "type": "string" } }
            ]
          },
          "post": {
            "tags": ["pets"],
            "operationId": "createPet",
            "requestBody": { "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Pet" } } } }
          }
        },
        "/pets/{petId}": {
          "parameters": [{ "name": "petId", "in": "path", "required": true, "schema": { "type": "string" } }],
          "get": { "summary": "Get a pet", "security": [] },
          "trace": { "summary": "Trace a pet" }
        }
      }
    }"##;

    fn parse_en(text: &str) -> ImportPlan {
        with_language(Language::En, || parse(text).unwrap())
    }

    fn http(request: &RequestFile) -> &HttpRequestSpec {
        request.http.as_ref().unwrap()
    }

    #[test]
    fn groups_operations_by_tag_and_leaves_untagged_ones_at_the_root() {
        let plan = parse_en(PETSTORE_V3);

        assert_eq!(plan.collection_name, "Petstore");
        assert_eq!(plan.folders.len(), 1);
        assert_eq!(plan.folders[0].name, "pets");
        let names: Vec<&str> = plan.folders[0].requests.iter().map(|r| r.meta.name.as_str()).collect();
        // `summary` names a request, `operationId` is the fallback.
        assert_eq!(names, vec!["List pets", "createPet"]);
        assert_eq!(plan.requests.len(), 1);
        assert_eq!(plan.requests[0].meta.name, "Get a pet");
    }

    #[test]
    fn a_request_carries_the_url_parameters_and_authorization_of_its_operation() {
        let plan = parse_en(PETSTORE_V3);
        let list = http(&plan.folders[0].requests[0]);

        assert_eq!(list.url, "{{baseUrl}}/pets");
        assert_eq!(list.method, HttpMethod::Get);
        // A required parameter is on, an optional one is a row waiting to be
        // switched on, and the value is the default the document gives.
        assert_eq!(list.query_params.len(), 1);
        assert_eq!(list.query_params[0].key, "limit");
        assert_eq!(list.query_params[0].value, "20");
        assert!(list.query_params[0].enabled);
        let header = list.headers.iter().find(|h| h.key == "X-Trace").unwrap();
        assert!(!header.enabled);
        assert!(matches!(&list.auth, AuthSpec::Bearer { token } if token == "{{token}}"));
    }

    #[test]
    fn path_parameters_stay_as_the_specification_writes_them() {
        let plan = parse_en(PETSTORE_V3);
        let get_one = http(&plan.requests[0]);

        assert_eq!(get_one.url, "{{baseUrl}}/pets/{petId}");
        // An empty `security` on an operation overrides the document default
        // rather than inheriting it.
        assert!(matches!(get_one.auth, AuthSpec::None));
        assert!(plan.warnings.iter().any(|w| w.contains("Path parameters")));
    }

    #[test]
    fn a_json_body_is_built_from_the_schema_with_the_examples_it_carries() {
        let plan = parse_en(PETSTORE_V3);
        let create = http(&plan.folders[0].requests[1]);

        let BodySpec::Text { content, format } = &create.body else {
            panic!("expected a text body, got {:?}", create.body)
        };
        assert_eq!(*format, TextFormat::Json);
        assert!(content.contains("\"name\": \"Rex\""), "{content}");
        // Nothing invented: a field without an example is an empty value of
        // its own type.
        assert!(content.contains("\"id\": 0"), "{content}");
        assert!(content.contains("\"tags\": [\n    \"\"\n  ]"), "{content}");
        assert!(create
            .headers
            .iter()
            .any(|h| h.key == "Content-Type" && h.value == "application/json"));
    }

    #[test]
    fn every_server_becomes_an_environment_carrying_the_same_variables() {
        let plan = parse_en(PETSTORE_V3);

        assert_eq!(plan.environments.len(), 2);
        assert_eq!(plan.environments[0].name, "Production");
        // The trailing slash goes: the request path brings its own.
        assert_eq!(plan.environments[0].variables[0].key, "baseUrl");
        assert_eq!(plan.environments[0].variables[0].value, "https://api.example.com/v1");
        // A templated server URL is filled in from the defaults it declares.
        assert_eq!(plan.environments[1].variables[0].value, "https://staging.example.com");
        assert_eq!(plan.environments[1].name, "staging.example.com");

        for environment in &plan.environments {
            let token = environment.variables.iter().find(|v| v.key == "token").unwrap();
            assert!(token.secret);
            assert!(token.value.is_empty(), "a secret value never goes into the environment file");
        }
        // Each environment owns its variables: the id is the key a secret is
        // stored under, so a shared one would carry the value across them.
        assert_ne!(plan.environments[0].variables[1].id, plan.environments[1].variables[1].id);
    }

    #[test]
    fn an_unsupported_method_is_reported_rather_than_dropped_silently() {
        let plan = parse_en(PETSTORE_V3);
        assert!(
            plan.warnings.iter().any(|w| w.contains("TRACE /pets/{petId}")),
            "{:?}",
            plan.warnings
        );
    }

    const LEGACY_V2: &str = r#"
swagger: "2.0"
info:
  title: Legacy API
host: legacy.example.com
basePath: /api
schemes: [http, https]
securityDefinitions:
  api_key:
    type: apiKey
    name: X-API-Key
    in: header
security:
  - api_key: []
paths:
  /login:
    post:
      summary: Log in
      consumes: [application/x-www-form-urlencoded]
      parameters:
        - { name: user, in: formData, type: string }
        - { name: password, in: formData, type: string }
  /items:
    post:
      summary: Create item
      parameters:
        - name: body
          in: body
          schema:
            type: object
            properties:
              title: { type: string }
"#;

    #[test]
    fn swagger_2_is_read_from_yaml_with_its_own_spelling_of_everything() {
        let plan = parse_en(LEGACY_V2);

        assert_eq!(plan.collection_name, "Legacy API");
        // Three fields instead of a server list, and HTTPS wins wherever the
        // API offers it at all.
        assert_eq!(plan.environments.len(), 1);
        assert_eq!(plan.environments[0].variables[0].value, "https://legacy.example.com/api");

        let login = plan.requests.iter().find(|r| r.meta.name == "Log in").unwrap();
        let login = http(login);
        let BodySpec::Form { fields } = &login.body else {
            panic!("expected a form body, got {:?}", login.body)
        };
        let keys: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, vec!["user", "password"]);
        // An API key is a header the request carries, not one of the auth
        // modes the app has.
        assert!(login
            .headers
            .iter()
            .any(|h| h.key == "X-API-Key" && h.value == "{{api_key}}"));
        assert!(plan.environments[0].variables.iter().any(|v| v.key == "api_key" && v.secret));

        let create = plan.requests.iter().find(|r| r.meta.name == "Create item").unwrap();
        let BodySpec::Text { content, .. } = &http(create).body else {
            panic!("expected a text body")
        };
        assert!(content.contains("\"title\": \"\""), "{content}");
    }

    #[test]
    fn a_document_that_is_neither_dialect_is_refused() {
        let error = with_language(Language::En, || parse(r#"{"info": {"title": "x"}}"#).unwrap_err());
        assert!(error.to_string().contains("not an OpenAPI specification"), "{error}");
    }

    #[test]
    fn a_schema_that_refers_to_itself_does_not_unfold_forever() {
        let spec = r##"{
          "openapi": "3.0.0",
          "info": { "title": "Tree" },
          "components": { "schemas": { "Node": { "type": "object", "properties": {
            "name": { "type": "string" },
            "child": { "$ref": "#/components/schemas/Node" }
          } } } },
          "paths": { "/nodes": { "post": { "summary": "Add node", "requestBody": { "content": {
            "application/json": { "schema": { "$ref": "#/components/schemas/Node" } }
          } } } } }
        }"##;

        let plan = parse_en(spec);
        let BodySpec::Text { content, .. } = &http(&plan.requests[0]).body else {
            panic!("expected a text body")
        };
        assert!(content.contains("\"child\": null"), "{content}");
    }

    /// A specification served over HTTP routinely gives its server address
    /// relative to itself, which is resolvable only against where it came
    /// from - the real Petstore document is written that way.
    #[test]
    fn a_relative_server_address_is_resolved_against_where_the_document_came_from() {
        let spec = r#"{
          "openapi": "3.0.0",
          "info": { "title": "Relative" },
          "servers": [{ "url": "/api/v3" }],
          "paths": { "/pets": { "get": { "summary": "List pets" } } }
        }"#;

        let downloaded = with_language(Language::En, || {
            parse_with_origin(spec, Some("https://petstore3.swagger.io/api/v3/openapi.json")).unwrap()
        });
        assert_eq!(
            downloaded.environments[0].variables[0].value,
            "https://petstore3.swagger.io/api/v3"
        );
        assert_eq!(downloaded.environments[0].name, "petstore3.swagger.io");

        // Imported from a file there is nothing to resolve it against, so it
        // is left alone and said out loud.
        let from_file = parse_en(spec);
        assert_eq!(from_file.environments[0].variables[0].value, "/api/v3");
        assert!(
            from_file.warnings.iter().any(|w| w.contains("relative server address")),
            "{:?}",
            from_file.warnings
        );
    }
}
