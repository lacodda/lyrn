//! Guards the facts that must agree before a version is published.
//!
//! lyrn ships to three places - GitHub, crates.io and npm - each of which
//! renders its own copy of the metadata. They drift silently: nothing fails
//! when `npm/package.json` still says 1.3.0, or when the npm page describes
//! the product differently from the crate. The drift is only visible after
//! publishing, when it is too late to take back.
//!
//! What every product's shopfronts owe each other - one version, one
//! description, the commands the README shows, the site's address - is held
//! by `lyrn check`, which `tests/check.rs` runs on this repository. What is
//! left here is lyrn's own: the forms, the reference pages that name
//! commands, the templates that have to survive packaging.
//!
//! These checks run in CI, so a mismatch fails the build instead of shipping.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn every_command_the_docs_reference_exists_in_the_cli() {
    // The rule of the line: a command without a page does not exist. This is
    // the other direction - a page without a command.
    let help = String::from_utf8(common::lyrn().arg("--help").output().unwrap().stdout).unwrap();
    let reference = repo_root().join("docs/src/content/docs/reference");

    for entry in fs::read_dir(&reference).expect("the reference directory is missing") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        assert!(
            help.contains(&stem),
            "docs/reference/{stem}.md documents `lyrn {stem}`, which the CLI does not have"
        );
    }
}

#[test]
fn every_embedded_template_file_survives_packaging() {
    // `include_str!` reads from the source tree, so a template file the crate
    // does not ship still compiles here and fails to compile from the tarball
    // - the crate publishes, and then nobody can install it. This happened:
    // a bare `docs/` in `exclude` is a glob matched anywhere, and it took
    // `src/templates/*/docs` with it.
    let manifest = read("Cargo.toml");
    let excluded: Vec<&str> = manifest
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("exclude"))
        .skip(1)
        .take_while(|l| !l.trim().starts_with(']'))
        .filter_map(|l| l.trim().trim_end_matches(',').strip_prefix('"'))
        .filter_map(|l| l.strip_suffix('"'))
        .collect();

    for pattern in &excluded {
        assert!(
            pattern.starts_with('/'),
            "`{pattern}` in exclude has no leading slash, so it matches at every depth -              anchor it to the root or it will silently drop files under src/"
        );
    }

    // And the templates really are in the package - asked of cargo itself.
    // This half used to imitate cargo's matching, and the imitation was
    // wrong the other way: it read the anchored `/docs/` as matching
    // `src/templates/docs`, which cargo ships, and so refused the docs form.
    // A second implementation of someone else's rule is a second truth.
    let listed = std::process::Command::new(env!("CARGO"))
        .args(["package", "--list", "--allow-dirty"])
        .current_dir(repo_root())
        .output()
        .expect("cargo package --list did not run");
    assert!(
        listed.status.success(),
        "cargo package --list failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    let packaged: std::collections::BTreeSet<String> = String::from_utf8_lossy(&listed.stdout).lines().map(|l| l.trim().replace('\\', "/")).collect();

    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { walk(&path, out) } else { out.push(path) }
        }
    }
    let mut files = Vec::new();
    walk(&repo_root().join("src/templates"), &mut files);
    assert!(
        files.len() > 100,
        "found only {} template files - the walk is not looking where they are",
        files.len()
    );
    for file in files {
        let relative = file.strip_prefix(repo_root()).unwrap().display().to_string().replace('\\', "/");
        assert!(
            packaged.contains(&relative),
            "`{relative}` is embedded by the binary but left out of the package"
        );
    }
}

/// Every form the binary carries is named in both shopfronts, and neither
/// names one it does not carry.
///
/// The drift this catches is the quiet kind: a form is added, the code and its
/// tests are right, and the README goes on describing the set that existed
/// before. It happened on this very version - the README said "all six forms"
/// while the binary carried eight - and nothing was red.
#[test]
fn both_shopfronts_list_exactly_the_forms_the_binary_carries() {
    let listing = String::from_utf8(common::lyrn().arg("forms").output().unwrap().stdout).unwrap();

    // The forms are the lines that do not begin with an option's indent.
    let forms: Vec<String> = listing
        .lines()
        .filter(|line| !line.starts_with(' ') && !line.trim().is_empty())
        .filter_map(|line| line.split_whitespace().next().map(str::to_string))
        .collect();

    assert!(forms.len() >= 2, "no forms parsed out of `lyrn forms`");

    let readme = read("README.md");
    let docs = read("docs/src/content/docs/reference/forms.md");

    for form in &forms {
        assert!(readme.contains(&format!("`{form}`")), "README does not name the `{form}` form");
        assert!(docs.contains(&format!("## {form}")), "docs/reference/forms.md has no `## {form}` section");
    }

    // The other direction: a section left behind after a form is renamed
    // documents something nobody can ask for.
    for line in docs.lines() {
        let Some(heading) = line.strip_prefix("## ") else { continue };
        let heading = heading.trim();
        // The page carries prose sections too; only the ones that look like a
        // form name are claims about the binary.
        if heading.contains(' ') || heading.chars().any(|c| c.is_uppercase()) {
            continue;
        }
        assert!(
            forms.iter().any(|f| f == heading),
            "docs/reference/forms.md documents the `{heading}` form, which the binary does not carry"
        );
    }
}

/// The transcript in the docs is what `lyrn forms` actually prints.
///
/// A hand-edited transcript is a screenshot of a version that no longer
/// exists, and it is the first thing a reader trusts.
#[test]
fn the_forms_transcript_is_the_one_the_binary_prints() {
    let listing = String::from_utf8(common::lyrn().arg("forms").output().unwrap().stdout).unwrap();
    let docs = read("docs/src/content/docs/reference/forms.md");

    for line in listing.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            docs.contains(line.trim_end()),
            "docs/reference/forms.md no longer shows what `lyrn forms` prints; this line is missing:\n  {line}"
        );
    }
}

/// The checks the `docs` form ships are the ones this site builds with.
///
/// Two copies of the same integration drift the day one is fixed. Holding them
/// byte for byte means every build of lyrn's own site is a live run of what
/// the form hands out, on the custom-domain layout the generated projects do
/// not start with.
#[test]
fn the_docs_form_ships_the_checks_this_site_builds_with() {
    for file in ["address.mjs", "llms.mjs"] {
        let shipped = read(format!("src/templates/docs/integrations/{file}"));
        let used = read(format!("docs/src/integrations/{file}"));
        assert!(shipped == used, "docs/src/integrations/{file} differs from the copy the docs form ships");
    }
    let astro = read("docs/astro.config.mjs");
    for integration in ["address()", "llms({ title, description })"] {
        assert!(astro.contains(integration), "docs/astro.config.mjs does not build with `{integration}`");
    }
}

/// `lyrn doctor --explain <rule>` sends the reader to the rule's section of
/// the reference page, by the rule's name. A rule added without a section
/// would send them to the top of a page that does not mention it.
#[test]
fn every_doctor_rule_has_a_section_on_its_page() {
    let refusal = common::lyrn().args(["doctor", "--explain", "no-such-rule"]).output().unwrap();
    let refusal = String::from_utf8(refusal.stderr).unwrap();
    let listed = refusal
        .split("[possible values: ")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("clap lists the rules it accepts");
    let rules: Vec<&str> = listed.split(", ").collect();
    assert!(rules.len() >= 10, "{rules:?}");

    let page = read("docs/src/content/docs/reference/doctor.md");
    for rule in rules {
        assert!(
            page.contains(&format!("\n### {rule}\n")),
            "docs/reference/doctor.md has no `### {rule}` section"
        );
    }
}

/// lyrn is installed through the same npm launcher the cli form writes, so it
/// is that launcher, word for word: a fix made in one place - waiting out
/// Ctrl+C instead of dying under the binary - reaches both.
#[test]
fn lyrn_launches_the_way_the_cli_form_does() {
    let template = read("src/templates/cli/npm/run.js.tmpl").replace("{{ name }}", "lyrn");
    assert!(read("npm/run.js") == template, "npm/run.js differs from the launcher the cli form writes");
}
