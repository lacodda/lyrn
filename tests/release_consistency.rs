//! Guards the facts that must agree before a version is published.
//!
//! lyrn ships to three places - GitHub, crates.io and npm - each of which
//! renders its own copy of the metadata. They drift silently: nothing fails
//! when `npm/package.json` still says 1.3.0, or when the npm page describes
//! the product differently from the crate. The drift is only visible after
//! publishing, when it is too late to take back.
//!
//! These checks run in CI, so a mismatch fails the build instead of shipping.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Extracts a top-level `key = "value"` from Cargo.toml.
///
/// Deliberately naive: it reads only the `[package]` block, which is all these
/// checks need, and avoids adding a TOML parser as a dev-dependency.
fn cargo_field(key: &str) -> String {
    let manifest = read("Cargo.toml");
    for line in manifest.lines() {
        let line = line.trim();
        // Stop at the next section: `version` also appears under [dependencies]
        // and in every dependency entry.
        if line.starts_with('[') && line != "[package]" {
            break;
        }
        let Some((name, value)) = line.split_once('=') else { continue };
        // Exact match, so `rust-version` cannot answer a lookup for `version`.
        if name.trim() != key {
            continue;
        }
        return value.trim().trim_matches('"').to_string();
    }
    panic!("`{key}` not found in the [package] block of Cargo.toml");
}

/// Extracts a `"key": "value"` from a JSON file, without a JSON dependency.
fn json_field(file: &str, key: &str) -> String {
    let text = read(file);
    let needle = format!("\"{key}\"");
    let start = text.find(&needle).unwrap_or_else(|| panic!("`{key}` not found in {file}"));
    let after = &text[start + needle.len()..];
    let after = after.trim_start().trim_start_matches(':').trim_start();
    let after = after.strip_prefix('"').unwrap_or_else(|| panic!("`{key}` in {file} is not a string"));
    after[..after.find('"').expect("unterminated string")].to_string()
}

#[test]
fn the_npm_package_version_matches_the_crate() {
    let crate_version = cargo_field("version");
    let npm_version = json_field("npm/package.json", "version");

    assert_eq!(
        npm_version, crate_version,
        "npm/package.json version ({npm_version}) differs from Cargo.toml ({crate_version}); \
         the npm page would advertise a version that was never released"
    );
}

#[test]
fn the_npm_wrapper_points_at_the_matching_release() {
    // `lyrn.binary` pins the tag the wrapper downloads. If it lags behind, npm
    // installs an older binary than the page advertises.
    let crate_version = cargo_field("version");
    let binary_tag = json_field("npm/package.json", "binary");

    assert_eq!(
        binary_tag,
        format!("v{crate_version}"),
        "npm/package.json pins the binary at {binary_tag}, but the crate is {crate_version}"
    );
}

#[test]
fn the_three_storefronts_describe_the_same_product() {
    let crate_description = cargo_field("description");
    let npm_description = json_field("npm/package.json", "description");

    assert_eq!(npm_description, crate_description, "the npm page and the crate describe lyrn differently");
}

#[test]
fn every_command_the_readme_shows_exists_in_the_cli() {
    // A README that documents a command the binary does not have is worse than
    // one that documents nothing: it is confidently wrong.
    let readme = read("README.md");
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();

    for line in readme.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("$ lyrn ") else { continue };
        let Some(command) = rest.split_whitespace().next() else { continue };
        // Placeholders and flags are not subcommands.
        if command.starts_with('-') || command.starts_with('<') {
            continue;
        }
        assert!(help.contains(command), "README shows `lyrn {command}`, which `lyrn --help` does not list");
    }
}

#[test]
fn every_command_the_docs_reference_exists_in_the_cli() {
    // The rule of the line: a command without a page does not exist. This is
    // the other direction - a page without a command.
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();
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
fn every_command_has_a_reference_page() {
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();
    let reference = repo_root().join("docs/src/content/docs/reference");

    // The subcommand list sits between the `Commands:` header and the next
    // blank line; `help` is clap's own and needs no page.
    let commands: Vec<String> = help
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|c| c != "help")
        .collect();

    assert!(!commands.is_empty(), "no commands parsed out of `lyrn --help`");

    for command in commands {
        let page = reference.join(format!("{command}.md"));
        assert!(page.is_file(), "`lyrn {command}` has no page at docs/reference/{command}.md");
    }
}

#[test]
fn the_docs_site_and_the_crate_describe_the_same_product() {
    let crate_description = cargo_field("description");
    let astro = read("docs/astro.config.mjs");

    // Starlight's description ends up in the site's meta tags, where a stale
    // sentence is invisible until someone shares a link.
    assert!(
        astro.contains(crate_description.trim_end_matches('.')),
        "docs/astro.config.mjs describes lyrn differently from Cargo.toml"
    );
}

/// Where the docs site is served from has to be said the same way in every
/// place that says it.
///
/// This is the check that was missing. The site moved to `lyrn.lacodda.com`,
/// `docs/astro.config.mjs` kept `site: 'https://lacodda.github.io'` with
/// `base: '/lyrn'`, and every page then asked for `/lyrn/_astro/…` on a domain
/// that serves those files from the root. Every stylesheet 404'd, the site
/// rendered as unstyled HTML, and nothing here noticed - the pages built, the
/// links resolved, the descriptions matched. The owner saw it in a browser.
///
/// A project site on `lacodda.github.io/<name>` and a site on its own domain
/// are two consistent arrangements; the failure is a mixture of the two. So
/// the rule is stated as a choice between them rather than as one fixed
/// address, and the other storefronts are held to whichever one is in force.
/// Every docs source, with absolute URLs stripped out.
///
/// `/lyrn/` is a legitimate part of `github.com/lacodda/lyrn/...` and of the
/// raw-content URLs the install snippets use, so those are removed before the
/// text is searched; what is left is site-relative links, where the prefix is
/// either required or forbidden depending on how the site is served.
fn prefixed_link_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            // Built output and dependencies are not sources, and walking them
            // turns a millisecond test into a slow one.
            if path
                .file_name()
                .is_some_and(|name| name == "node_modules" || name == "dist" || name == ".astro")
            {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "md" || e == "mdx" || e == "mjs") {
                let relative = path.strip_prefix(repo_root()).unwrap_or(&path).display().to_string().replace('\\', "/");
                let text = fs::read_to_string(&path).unwrap_or_default();
                // Drop every absolute URL; only site-relative paths remain.
                let stripped = text.split_whitespace().filter(|word| !word.contains("://")).collect::<Vec<_>>().join(" ");
                out.push((relative, stripped));
            }
        }
    }

    let mut out = Vec::new();
    walk(&repo_root().join("docs"), &mut out);
    out
}

#[test]
fn the_docs_site_agrees_with_itself_about_where_it_lives() {
    let astro = read("docs/astro.config.mjs");
    let site = astro
        .lines()
        .find_map(|line| line.trim().strip_prefix("site:"))
        .map(|value| value.trim().trim_matches(|c| c == '\'' || c == ',' || c == '"').to_string())
        .expect("`site` not found in docs/astro.config.mjs");

    // `base` is only meaningful when it is not commented out.
    let has_base = astro
        .lines()
        .any(|line| !line.trim_start().starts_with("//") && line.trim().starts_with("base:"));

    let cname_path = repo_root().join("docs/public/CNAME");
    let cname = fs::read_to_string(&cname_path).ok().map(|text| text.trim().to_string());

    match &cname {
        Some(domain) => {
            // A custom domain serves from the root. A `base` here is what
            // makes the pages ask for assets under a path that does not exist.
            assert_eq!(
                site,
                format!("https://{domain}"),
                "docs/public/CNAME says `{domain}` but astro.config.mjs builds for `{site}`"
            );
            assert!(
                !has_base,
                "astro.config.mjs keeps a `base` while docs/public/CNAME serves `{domain}` from the root - \
                 every asset URL will carry a prefix the domain does not have"
            );
            // The config is not the only place the old prefix hides. Content
            // links are written by hand, and three of them survived the move
            // to the domain - including the front page's "Get started"
            // button, which 404'd while every asset around it loaded. Found by
            // walking the live site, not by reading the config.
            for (file, text) in prefixed_link_sources() {
                assert!(
                    !text.contains("/lyrn/"),
                    "{file} links to `/lyrn/...`, a path the custom domain does not serve - \
                     with no `base`, an internal link is written from the root"
                );
            }
        }
        None => {
            // A github.io project site: the base path is what makes it work.
            assert_eq!(
                site, "https://lacodda.github.io",
                "no docs/public/CNAME, so the site must build for lacodda.github.io"
            );
            assert!(has_base, "a github.io project site needs `base`, or every asset resolves to the wrong path");
        }
    }

    // npm renders `homepage` as the package's link; a stale one sends readers
    // to a redirect at best.
    let homepage = json_field("npm/package.json", "homepage");
    let expected = match &cname {
        Some(domain) => format!("https://{domain}"),
        None => "https://lacodda.github.io/lyrn/".to_string(),
    };
    assert_eq!(
        homepage.trim_end_matches('/'),
        expected.trim_end_matches('/'),
        "npm/package.json points readers somewhere other than where the docs are served"
    );
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

    // And the templates really are in the package: walk what the sources
    // reference and check none of it sits under an excluded path.
    for entry in fs::read_dir(repo_root().join("src/templates")).expect("no templates directory") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let relative = path.strip_prefix(repo_root()).unwrap().display().to_string().replace('\\', "/");
        for pattern in &excluded {
            let bare = pattern.trim_start_matches('/').trim_end_matches('/');
            assert!(
                !relative.split('/').any(|part| part == bare),
                "the template directory `{relative}` matches the exclude pattern `{pattern}`"
            );
        }
    }
}

#[test]
fn the_installers_agree_on_the_repository() {
    for installer in ["tools/install.sh", "tools/install.ps1"] {
        let text = read(installer);
        assert!(text.contains("lacodda/lyrn"), "{installer} does not point at lacodda/lyrn");
        // The tag must come from the redirect, never from the rate-limited API.
        assert!(
            !text.contains("api.github.com"),
            "{installer} asks api.github.com for the latest tag; unauthenticated calls are \
             capped at 60/hour per IP and the installer would fail for reasons of someone else's making"
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
    let listing = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("forms").output().unwrap().stdout).unwrap();

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
    let listing = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("forms").output().unwrap().stdout).unwrap();
    let docs = read("docs/src/content/docs/reference/forms.md");

    for line in listing.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            docs.contains(line.trim_end()),
            "docs/reference/forms.md no longer shows what `lyrn forms` prints; this line is missing:\n  {line}"
        );
    }
}
