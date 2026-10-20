//! `node`: pnpm and Node at the majors the line builds with.
//!
//! The majors are read from the templates lyrn writes - `packageManager` in a
//! generated `package.json`, `node-version` in its workflows - so the rule
//! and the projects lyrn starts can never disagree about what current is.

use std::collections::BTreeMap;

use regex::Regex;

use super::{Finding, Outcome, Repair, Repo, newline_of, plural, some_of};
use crate::generate::Contents;
use crate::model::Form;
use crate::templates;

/// What the standard builds JavaScript with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    /// `pnpm@10.34.5`, exactly as the templates pin it.
    pub package_manager: String,
    pub pnpm_major: u32,
    /// The oldest Node a template's workflow runs on.
    pub node_major: u32,
}

pub fn standard() -> Toolchain {
    let pinned = Regex::new(r#""packageManager":\s*"(pnpm@(\d+)[^"]*)""#).expect("a valid pattern");
    let node = Regex::new(r#"node-version:\s*['"]?(\d+)"#).expect("a valid pattern");
    let mut package_manager: Option<(String, u32)> = None;
    let mut node_major: Option<u32> = None;
    for &form in Form::ALL {
        for source in templates::sources_for(form) {
            let Contents::Text(text) = source.contents else { continue };
            if source.path.ends_with("package.json")
                && let Some(c) = pinned.captures(text)
                && let Ok(major) = c[2].parse::<u32>()
                && package_manager.as_ref().is_none_or(|(_, m)| major > *m)
            {
                package_manager = Some((c[1].to_string(), major));
            }
            if source.path.starts_with(".github/workflows/") {
                for c in node.captures_iter(text) {
                    if let Ok(major) = c[1].parse::<u32>() {
                        node_major = Some(node_major.map_or(major, |m| m.min(major)));
                    }
                }
            }
        }
    }
    let (package_manager, pnpm_major) = package_manager.expect("a template pins pnpm with packageManager");
    Toolchain {
        package_manager,
        pnpm_major,
        node_major: node_major.expect("a template's workflow sets up Node"),
    }
}

pub fn check(repo: &Repo) -> Outcome {
    let lockfiles: Vec<&str> = repo.files_named("pnpm-lock.yaml").collect();
    let sets_up_node = repo.workflows.iter().flat_map(|w| w.jobs()).flat_map(|j| j.steps).any(|s| {
        s.uses
            .is_some_and(|u| u.starts_with("actions/setup-node@") || u.starts_with("pnpm/action-setup@"))
    });
    if lockfiles.is_empty() && !sets_up_node {
        return Outcome::Skip("no pnpm project and no Node in CI".to_string());
    }

    let standard = standard();
    let mut findings = Vec::new();

    for lockfile in &lockfiles {
        let dir = lockfile.strip_suffix("pnpm-lock.yaml").unwrap_or_default();
        let manifest = format!("{dir}package.json");
        let Some(text) = repo.read_text(&manifest) else { continue };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            findings.push(Finding::fail(format!("{manifest} does not parse")));
            continue;
        };
        match json.get("packageManager").and_then(|v| v.as_str()) {
            None => {
                let mut finding = Finding::warn(format!("{manifest} names no packageManager: CI and every machine use whatever pnpm they have"))
                    .fix(format!("add `\"packageManager\": \"{}\"`", standard.package_manager));
                if let Some(pinned) = with_package_manager(&text, &standard.package_manager) {
                    finding = finding.repair(Repair::new(manifest.clone(), pinned, format!("pins {}", standard.package_manager)));
                }
                findings.push(finding);
            }
            Some(pinned) => {
                let major = pinned
                    .strip_prefix("pnpm@")
                    .and_then(|v| v.split('.').next())
                    .and_then(|m| m.parse::<u32>().ok());
                match major {
                    Some(m) if m < standard.pnpm_major => findings.push(
                        Finding::fail(format!("{manifest} pins {pinned}: the line builds with pnpm {}", standard.pnpm_major))
                            .fix(format!("pin `{}` and run `pnpm install`", standard.package_manager)),
                    ),
                    Some(m) if m > standard.pnpm_major => findings.push(
                        Finding::warn(format!("{manifest} pins {pinned}, ahead of the line's pnpm {}", standard.pnpm_major))
                            .fix(format!("pin `{}` until the line moves", standard.package_manager)),
                    ),
                    Some(_) => {}
                    None => findings
                        .push(Finding::fail(format!("{manifest} pins {pinned} beside a pnpm lockfile")).fix(format!("pin `{}`", standard.package_manager))),
                }
            }
        }
    }

    // A gate run against node_modules older than the lockfile checks a
    // different program from the one that ships.
    for lockfile in &lockfiles {
        let dir = lockfile.strip_suffix("pnpm-lock.yaml").unwrap_or_default();
        let settings = format!("{dir}pnpm-workspace.yaml");
        let workspace = repo.read_text(&settings);
        let npmrc = repo.read_text(&format!("{dir}.npmrc"));
        if verifies_before_run(workspace.as_deref(), npmrc.as_deref()) {
            continue;
        }
        findings.push(
            Finding::fail(format!("{settings} lets a script run against node_modules older than {lockfile}"))
                .fix("`verifyDepsBeforeRun: install`: pnpm then installs what fell behind the lockfile before any `pnpm run`")
                .repair(Repair::new(
                    settings,
                    with_verify(workspace.as_deref().unwrap_or_default()),
                    "has pnpm install what fell behind the lockfile before a script runs",
                )),
        );
    }

    // One finding per version, naming the files it is in.
    let mut old_node: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut old_pnpm: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unpinned: Vec<String> = Vec::new();
    for workflow in &repo.workflows {
        for job in workflow.jobs() {
            for step in &job.steps {
                let Some(uses) = step.uses else { continue };
                if uses.starts_with("actions/setup-node@") {
                    match step.input("node-version") {
                        Some(version) => {
                            if major(&version).is_some_and(|m| m < standard.node_major) {
                                push_once(old_node.entry(version).or_default(), workflow.name());
                            }
                        }
                        None if step.input("node-version-file").is_none() => push_once(&mut unpinned, workflow.name()),
                        None => {}
                    }
                }
                if uses.starts_with("pnpm/action-setup@")
                    && let Some(version) = step.input("version")
                    && major(&version).is_some_and(|m| m < standard.pnpm_major)
                {
                    push_once(old_pnpm.entry(version).or_default(), workflow.name());
                }
            }
        }
    }
    for (version, files) in &old_node {
        findings.push(
            Finding::fail(format!(
                "Node {version} in {}: the line builds on Node {} or later",
                some_of(files, 3),
                standard.node_major
            ))
            .fix(format!("`node-version: {}`", standard.node_major)),
        );
    }
    for (version, files) in &old_pnpm {
        findings.push(
            Finding::fail(format!(
                "pnpm {version} in {}: the line builds with pnpm {}",
                some_of(files, 3),
                standard.pnpm_major
            ))
            .fix("drop `version:` and let pnpm/action-setup read packageManager"),
        );
    }
    if !unpinned.is_empty() {
        findings.push(
            Finding::warn(format!(
                "setup-node without a node-version in {}: the job runs on whatever the runner has",
                some_of(&unpinned, 3)
            ))
            .fix(format!("`node-version: {}`", standard.node_major)),
        );
    }

    Outcome::from_findings(findings, || {
        format!(
            "pnpm {} and Node {}+ across {}",
            standard.pnpm_major,
            standard.node_major,
            plural(lockfiles.len().max(1), "project", "projects")
        )
    })
}

/// The manifest with `packageManager` as its last field, written the way the
/// file writes its others - its indentation, its line endings - so the diff
/// is the one line. `None` if the result would not parse.
fn with_package_manager(text: &str, package_manager: &str) -> Option<String> {
    let end = text.rfind('}')?;
    let before = text[..end].trim_end();
    let newline = newline_of(text);
    let indent: String = text
        .lines()
        .nth(1)
        .map(|line| line.chars().take_while(|c| c.is_whitespace()).collect())
        .filter(|indent: &String| !indent.is_empty())
        .unwrap_or_else(|| "  ".to_string());
    let comma = if before.ends_with('{') { "" } else { "," };
    let pinned = format!(
        "{before}{comma}{newline}{indent}\"packageManager\": \"{package_manager}\"{newline}{}",
        &text[end..]
    );
    let json: serde_json::Value = serde_json::from_str(&pinned).ok()?;
    (json.get("packageManager")?.as_str()? == package_manager).then_some(pinned)
}

/// The setting, with the reason the standard's templates give it.
const VERIFY: &str = "# Before a script runs, pnpm holds node_modules to the lockfile and installs
# what has fallen behind it: a gate run against older packages than the lock
# names passes for something other than what ships.
verifyDepsBeforeRun: install
";

/// Whether pnpm checks node_modules against the lockfile before a script,
/// and stops or installs when they differ: in pnpm-workspace.yaml, where the
/// line keeps pnpm's settings, or in .npmrc, where pnpm 10 also reads them.
fn verifies_before_run(workspace: Option<&str>, npmrc: Option<&str>) -> bool {
    let yaml = Regex::new(r#"(?m)^verifyDepsBeforeRun:\s*['"]?(install|error)['"]?\s*(#.*)?$"#).expect("a valid pattern");
    let ini = Regex::new(r"(?m)^\s*verify-deps-before-run\s*=\s*(install|error)\s*$").expect("a valid pattern");
    workspace.is_some_and(|t| yaml.is_match(t)) || npmrc.is_some_and(|t| ini.is_match(t))
}

/// pnpm-workspace.yaml with the setting: a value that is there and says
/// something else is replaced, otherwise the standard's lines go at the end.
fn with_verify(text: &str) -> String {
    let newline = newline_of(text);
    let existing = Regex::new(r"(?m)^verifyDepsBeforeRun:.*$").expect("a valid pattern");
    if existing.is_match(text) {
        return existing.replace(text, "verifyDepsBeforeRun: install").into_owned();
    }
    let block = VERIFY.replace('\n', newline);
    match text.trim_end() {
        "" => block,
        kept => format!("{kept}{newline}{newline}{block}"),
    }
}

fn push_once(files: &mut Vec<String>, file: &str) {
    if !files.iter().any(|f| f == file) {
        files.push(file.to_string());
    }
}

/// `22`, `22.x`, `'22.12'` and `v22` are major 22; `lts/*` has none.
pub(super) fn major(version: &str) -> Option<u32> {
    let version = version.trim().trim_start_matches(['v', '^', '~', '>', '=']);
    let end = version.find(|c: char| !c.is_ascii_digit()).unwrap_or(version.len());
    version[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// If this fails, the templates stopped saying what the rule reads.
    #[test]
    fn the_standard_is_read_from_the_templates() {
        let standard = standard();
        assert!(standard.package_manager.starts_with(&format!("pnpm@{}.", standard.pnpm_major)), "{standard:?}");
        assert!(standard.node_major >= 20, "{standard:?}");
    }

    #[test]
    fn the_pin_is_added_as_the_file_writes_its_fields() {
        let pretty = "{\r\n    \"name\": \"site\",\r\n    \"private\": true\r\n}\r\n";
        assert_eq!(
            with_package_manager(pretty, "pnpm@10.34.5").unwrap(),
            "{\r\n    \"name\": \"site\",\r\n    \"private\": true,\r\n    \"packageManager\": \"pnpm@10.34.5\"\r\n}\r\n"
        );
        let one_line = "{ \"name\": \"site\" }\n";
        assert_eq!(
            with_package_manager(one_line, "pnpm@10.34.5").unwrap(),
            "{ \"name\": \"site\",\n  \"packageManager\": \"pnpm@10.34.5\"\n}\n"
        );
        assert_eq!(
            with_package_manager("{}", "pnpm@10.34.5").unwrap(),
            "{\n  \"packageManager\": \"pnpm@10.34.5\"\n}"
        );
        assert_eq!(with_package_manager("not json", "pnpm@10.34.5"), None);
    }

    /// The repair writes what the templates write, so a repaired project and
    /// a generated one cannot drift apart.
    #[test]
    fn the_setting_is_the_one_the_templates_write() {
        for form in [Form::Spa, Form::Mono, Form::Docs, Form::TauriPlugin] {
            let settings = templates::sources_for(form)
                .into_iter()
                .find(|s| s.path.ends_with("pnpm-workspace.yaml"))
                .unwrap();
            let Contents::Text(text) = settings.contents else { panic!() };
            assert!(text.contains(VERIFY), "{form}'s pnpm-workspace.yaml does not say it as the rule repairs it");
            assert!(verifies_before_run(Some(text), None), "{form}");
        }
    }

    #[test]
    fn the_setting_is_added_or_corrected() {
        assert_eq!(with_verify(""), VERIFY);
        assert_eq!(
            with_verify("onlyBuiltDependencies:\n  - esbuild\n"),
            format!("onlyBuiltDependencies:\n  - esbuild\n\n{VERIFY}")
        );
        assert_eq!(with_verify("verifyDepsBeforeRun: false\nx: 1\n"), "verifyDepsBeforeRun: install\nx: 1\n");
        let crlf = with_verify("x: 1\r\n");
        assert!(
            crlf.ends_with("verifyDepsBeforeRun: install\r\n") && !crlf.replace("\r\n", "").contains('\n'),
            "{crlf:?}"
        );
        assert!(verifies_before_run(Some(&with_verify("x: 1\n")), None));
        assert!(verifies_before_run(None, Some("verify-deps-before-run=install\n")));
        assert!(!verifies_before_run(Some("verifyDepsBeforeRun: warn\n"), None));
        assert!(!verifies_before_run(None, None));
    }

    #[test]
    fn a_node_version_has_a_major() {
        assert_eq!(major("22"), Some(22));
        assert_eq!(major("22.x"), Some(22));
        assert_eq!(major("v20.11.1"), Some(20));
        assert_eq!(major(">=18"), Some(18));
        assert_eq!(major("lts/*"), None);
    }
}
