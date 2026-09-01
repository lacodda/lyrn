//! Project names and the line's accents.

use std::fmt;

/// The line's registry of marks: product, two-letter code, accent.
///
/// dowel derives a product's whole palette from this one colour, so `lyrn new`
/// only has to write a single line into the generated theme.
pub const LINE_ACCENTS: &[(&str, &str)] = &[
    ("atlas", "#8A7DF5"),
    ("austeris", "#C25BD9"),
    ("dowel", "#E8862D"),
    ("efema", "#5470E8"),
    ("kasl", "#A9C23F"),
    ("kasl-server", "#D9A82E"),
    ("kilna", "#D9569E"),
    ("lyrid", "#4A8FE8"),
    ("lyrn", "#6D7BF2"),
    ("midda", "#A46BE8"),
    ("nitid", "#3FA9D9"),
    ("nooma", "#3FA873"),
    ("sefy", "#35A8A0"),
    ("turnout", "#E85B72"),
];

/// The accent a product without its own mark starts with: neutral graphite,
/// a placeholder that reads as "the mark has not been drawn yet".
pub const PLACEHOLDER_ACCENT: &str = "#6E7079";

/// Why a name cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong(usize),
    BadStart(char),
    BadChar(char),
    BadEnd(char),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::Empty => write!(f, "the name is empty"),
            NameError::TooLong(n) => {
                write!(f, "the name is {n} characters; npm stops at 214")
            }
            NameError::BadStart(c) => {
                write!(f, "the name starts with `{c}`; it has to start with a lowercase letter")
            }
            NameError::BadChar(c) => write!(f, "the name contains `{c}`; only lowercase letters, digits and hyphens are allowed"),
            NameError::BadEnd(c) => write!(f, "the name ends with `{c}`; it has to end with a letter or a digit"),
        }
    }
}

impl std::error::Error for NameError {}

/// Check a project name against what a directory, a crate and an npm package
/// can all be called at once. The strictest of the three wins.
pub fn validate_name(name: &str) -> Result<(), NameError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(NameError::Empty);
    };
    if name.len() > 214 {
        return Err(NameError::TooLong(name.len()));
    }
    if !first.is_ascii_lowercase() {
        return Err(NameError::BadStart(first));
    }
    if let Some(bad) = name.chars().find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')) {
        return Err(NameError::BadChar(bad));
    }
    let last = name.chars().next_back().expect("name is not empty");
    if last == '-' {
        return Err(NameError::BadEnd(last));
    }
    Ok(())
}

/// Resolve `--accent`: either a product of the line, or a literal hex colour.
pub fn resolve_accent(input: &str) -> Result<String, String> {
    if let Some((_, hex)) = LINE_ACCENTS.iter().find(|(product, _)| *product == input) {
        return Ok((*hex).to_string());
    }
    if is_hex_colour(input) {
        return Ok(input.to_uppercase());
    }
    Err(format!(
        "`{input}` is neither a product of the line nor a `#rrggbb` colour (products: {})",
        LINE_ACCENTS.iter().map(|(p, _)| *p).collect::<Vec<_>>().join(", ")
    ))
}

fn is_hex_colour(s: &str) -> bool {
    let Some(digits) = s.strip_prefix('#') else { return false };
    digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit())
}

/// Turn a project name into the title a README and an `<h1>` show.
pub fn title_from_name(name: &str) -> String {
    name.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_name() {
        assert!(validate_name("demo-app").is_ok());
    }

    #[test]
    fn accepts_digits_inside() {
        assert!(validate_name("app2").is_ok());
    }

    #[test]
    fn rejects_an_empty_name() {
        assert_eq!(validate_name(""), Err(NameError::Empty));
    }

    #[test]
    fn rejects_uppercase() {
        // npm refuses uppercase in a package name, so the line never uses it.
        assert_eq!(validate_name("DemoApp"), Err(NameError::BadStart('D')));
    }

    #[test]
    fn rejects_a_leading_digit() {
        assert_eq!(validate_name("2fast"), Err(NameError::BadStart('2')));
    }

    #[test]
    fn rejects_underscores_and_spaces() {
        assert_eq!(validate_name("demo_app"), Err(NameError::BadChar('_')));
        assert_eq!(validate_name("demo app"), Err(NameError::BadChar(' ')));
    }

    #[test]
    fn rejects_a_trailing_hyphen() {
        assert_eq!(validate_name("demo-"), Err(NameError::BadEnd('-')));
    }

    #[test]
    fn resolves_a_product_of_the_line_to_its_mark() {
        assert_eq!(resolve_accent("kilna").unwrap(), "#D9569E");
    }

    #[test]
    fn resolves_a_literal_colour() {
        assert_eq!(resolve_accent("#a1b2c3").unwrap(), "#A1B2C3");
    }

    #[test]
    fn rejects_a_colour_without_a_hash() {
        assert!(resolve_accent("a1b2c3").is_err());
    }

    #[test]
    fn rejects_a_short_colour() {
        // `#abc` is valid CSS but dowel's derivation reads six digits.
        assert!(resolve_accent("#abc").is_err());
    }

    #[test]
    fn every_accent_in_the_registry_is_a_six_digit_colour() {
        for (product, hex) in LINE_ACCENTS {
            assert!(is_hex_colour(hex), "{product} has a malformed accent: {hex}");
        }
        assert!(is_hex_colour(PLACEHOLDER_ACCENT));
    }

    #[test]
    fn builds_a_title_from_a_hyphenated_name() {
        assert_eq!(title_from_name("demo-app"), "Demo App");
        assert_eq!(title_from_name("lyrn"), "Lyrn");
    }
}
