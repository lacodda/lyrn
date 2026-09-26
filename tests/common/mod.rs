//! What more than one test file needs to agree on.

// Each test file is its own crate and uses a different part of this module.
#![allow(dead_code)]

/// Every form, as `lyrn forms` names it.
pub const FORMS: &[&str] = &["spa", "cli", "desktop", "service", "workspace", "mono", "plugin", "tauri-plugin", "docs"];

/// The add-on sets each form has to survive.
///
/// Two add-ons is four combinations, which is cheap enough to check
/// exhaustively; if a form ever grows enough of them for that to hurt, the
/// answer is fewer add-ons rather than less checking.
pub fn combinations(form: &str) -> Vec<Vec<&'static str>> {
    match form {
        "cli" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "desktop" => vec![vec![], vec!["i18n"]],
        "service" => vec![vec![], vec!["spa"]],
        "workspace" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "mono" => vec![vec![], vec!["stand"]],
        _ => vec![vec![]],
    }
}
