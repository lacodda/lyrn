//! Placeholder substitution.
//!
//! The syntax is `{{ name }}`: braces, optional spaces, a key. It was chosen
//! over `${name}` because a generated project is full of shell and JavaScript
//! that uses `${...}` for its own purposes, and over bare `{name}` because
//! Rust format strings and CSS both use single braces.

use std::fmt;

use crate::model::Context;

/// A placeholder the template asked for but the context cannot fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPlaceholder {
    pub key: String,
}

impl fmt::Display for UnknownPlaceholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the template asks for `{{{{ {} }}}}`, which nothing defines", self.key)
    }
}

impl std::error::Error for UnknownPlaceholder {}

/// Replace every `{{ key }}` in `input` with its value from `context`.
///
/// An unknown key is an error rather than an empty string: a template that
/// silently renders half a value produces a project that looks fine and is
/// wrong, which is the expensive kind of wrong.
pub fn render(input: &str, context: &Context) -> Result<String, UnknownPlaceholder> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        let (before, from_open) = rest.split_at(start);
        out.push_str(before);

        // `${{ ... }}` belongs to GitHub Actions, which uses the same braces
        // for its own expressions. The dollar tells them apart, so a workflow
        // can carry both without being excluded from substitution entirely.
        if before.ends_with('$') {
            let end = from_open.find("}}").map(|e| e + 2).unwrap_or(from_open.len());
            out.push_str(&from_open[..end]);
            rest = &from_open[end..];
            continue;
        }

        let Some(end) = from_open.find("}}") else {
            // An unpaired `{{` is literal text, not a broken placeholder.
            out.push_str(from_open);
            return Ok(out);
        };

        let key = from_open[2..end].trim();
        match context.get(key) {
            Some(value) => out.push_str(value),
            None => return Err(UnknownPlaceholder { key: key.to_string() }),
        }
        rest = &from_open[end + 2..];
    }

    out.push_str(rest);
    Ok(out)
}

/// Every placeholder key a template file mentions, in order of appearance.
///
/// This exists for the gate that checks no template asks for a variable the
/// generator cannot provide, which is a compile-time-only concern.
#[cfg(test)]
pub fn placeholders(input: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        let from_open = &rest[start..];
        let Some(end) = from_open.find("}}") else { break };
        // `${{ ... }}` is a GitHub Actions expression, not a placeholder; and
        // `{{#addon}}` / `{{^addon}}` / `{{/addon}}` are section markers the
        // generator strips before rendering.
        let is_actions = rest[..start].ends_with('$');
        let key = from_open[2..end].trim();
        if !is_actions && !key.starts_with(['#', '^', '/']) {
            found.push(key.to_string());
        }
        rest = &from_open[end + 2..];
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        let mut c = Context::new();
        c.set("name", "demo-app").set("accent", "#6D7BF2");
        c
    }

    #[test]
    fn substitutes_a_placeholder() {
        assert_eq!(render("hello {{ name }}", &ctx()).unwrap(), "hello demo-app");
    }

    #[test]
    fn tolerates_missing_spaces() {
        assert_eq!(render("{{name}}", &ctx()).unwrap(), "demo-app");
    }

    #[test]
    fn substitutes_every_occurrence() {
        let out = render("{{ name }}/{{ name }}", &ctx()).unwrap();
        assert_eq!(out, "demo-app/demo-app");
    }

    #[test]
    fn leaves_single_braces_alone() {
        // CSS and Rust format strings must survive untouched.
        let input = ":root { --accent-base: red }";
        assert_eq!(render(input, &ctx()).unwrap(), input);
    }

    #[test]
    fn leaves_shell_and_js_interpolation_alone() {
        let input = "echo ${HOME} and `${count}`";
        assert_eq!(render(input, &ctx()).unwrap(), input);
    }

    #[test]
    fn an_unpaired_open_brace_is_literal() {
        assert_eq!(render("a {{ b", &ctx()).unwrap(), "a {{ b");
    }

    #[test]
    fn an_unknown_key_is_an_error_not_an_empty_string() {
        let err = render("{{ nope }}", &ctx()).unwrap_err();
        assert_eq!(err.key, "nope");
    }

    #[test]
    fn reports_the_placeholders_a_file_uses() {
        assert_eq!(placeholders("{{ a }} x {{ b }}"), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn an_empty_input_renders_empty() {
        assert_eq!(render("", &ctx()).unwrap(), "");
    }
}
