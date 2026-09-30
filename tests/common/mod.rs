//! What more than one test file needs to agree on.

// Each test file is its own crate and uses a different part of this module.
#![allow(dead_code)]

/// Every subset of `addons`, in the order given, the empty one first.
fn subsets(addons: &[&'static str]) -> Vec<Vec<&'static str>> {
    (0..1u32 << addons.len())
        .map(|mask| addons.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, a)| *a).collect())
        .collect()
}

/// Every form, as `lyrn forms` names it.
pub const FORMS: &[&str] = &["spa", "cli", "desktop", "service", "workspace", "mono", "plugin", "tauri-plugin", "docs"];

/// The add-on sets each form has to survive.
///
/// Every subset: two add-ons is four combinations and three is eight, which
/// is cheap enough to check exhaustively; if a form ever grows enough of them
/// for that to hurt, the answer is fewer add-ons rather than less checking.
pub fn combinations(form: &str) -> Vec<Vec<&'static str>> {
    match form {
        // Four add-ons, sixteen sets: they share main.tsx, App.tsx, its test
        // and index.html, so every pairing is a different file.
        "spa" => subsets(&["router", "tanstack-query", "auth", "pwa"]),
        "cli" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "desktop" => vec![vec![], vec!["i18n"]],
        "service" => subsets(&["spa", "demo"]),
        "workspace" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "mono" => vec![vec![], vec!["stand"]],
        _ => vec![vec![]],
    }
}

/// The lyrn binary, with a home of its own.
///
/// `~/.lyrn/templates/spa` on the machine running the tests would stand in for
/// the built-in spa form, and every test of it would then measure somebody's
/// local template. `LYRN_HOME` points at a directory that stays empty.
pub fn lyrn() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("lyrn").unwrap();
    cmd.env("LYRN_HOME", concat!(env!("CARGO_TARGET_TMPDIR"), "/empty-lyrn-home"));
    cmd
}
