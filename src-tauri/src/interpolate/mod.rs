use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::OnceLock;

fn var_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}").unwrap())
}

#[derive(Debug, Clone, Default)]
pub struct VariableScope(pub HashMap<String, String>);

/// Resolves `{{var}}` references. Precedence: active collection-scoped
/// environment, then active global environment. No per-request local
/// variables in MVP — an easy additive tier later.
pub struct Resolver {
    global: VariableScope,
    collection: Option<VariableScope>,
}

impl Resolver {
    pub fn new(global: VariableScope, collection: Option<VariableScope>) -> Self {
        Resolver { global, collection }
    }

    pub fn resolve(&self, key: &str) -> Option<&str> {
        self.collection
            .as_ref()
            .and_then(|s| s.0.get(key))
            .or_else(|| self.global.0.get(key))
            .map(String::as_str)
    }

    /// Returns the substituted string and the names of any variables that
    /// couldn't be resolved (left verbatim as `{{name}}` in the output).
    pub fn interpolate(&self, input: &str) -> (String, Vec<String>) {
        let mut unresolved = Vec::new();
        let output = var_regex().replace_all(input, |caps: &Captures| match self.resolve(&caps[1]) {
            Some(value) => value.to_string(),
            None => {
                unresolved.push(caps[1].to_string());
                caps[0].to_string()
            }
        });
        (output.into_owned(), unresolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(pairs: &[(&str, &str)]) -> VariableScope {
        VariableScope(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    #[test]
    fn collection_scope_overrides_global() {
        let resolver = Resolver::new(
            scope(&[("baseUrl", "https://global.example.com")]),
            Some(scope(&[("baseUrl", "https://collection.example.com")])),
        );
        let (out, unresolved) = resolver.interpolate("{{baseUrl}}/pets");
        assert_eq!(out, "https://collection.example.com/pets");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn falls_back_to_global_when_collection_has_no_value() {
        let resolver = Resolver::new(scope(&[("token", "abc")]), Some(scope(&[])));
        let (out, _) = resolver.interpolate("Bearer {{ token }}");
        assert_eq!(out, "Bearer abc");
    }

    #[test]
    fn reports_unresolved_variables_and_leaves_them_verbatim() {
        let resolver = Resolver::new(scope(&[]), None);
        let (out, unresolved) = resolver.interpolate("{{missing}}");
        assert_eq!(out, "{{missing}}");
        assert_eq!(unresolved, vec!["missing".to_string()]);
    }

    #[test]
    fn handles_no_collection_scope_and_multiple_vars() {
        let resolver = Resolver::new(scope(&[("a", "1"), ("b", "2")]), None);
        let (out, unresolved) = resolver.interpolate("{{a}}-{{b}}-{{c}}");
        assert_eq!(out, "1-2-{{c}}");
        assert_eq!(unresolved, vec!["c".to_string()]);
    }

    #[test]
    fn ignores_malformed_braces() {
        let resolver = Resolver::new(scope(&[("x", "1")]), None);
        let (out, unresolved) = resolver.interpolate("{{}} {x}} {{ x }}");
        assert_eq!(out, "{{}} {x}} 1");
        assert!(unresolved.is_empty());
    }
}
