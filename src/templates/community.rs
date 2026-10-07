//! What every repository the line starts carries beside its code: how to work
//! on it, how to report a vulnerability, how people treat each other, why
//! Dependabot is off and what audits instead, and the mark it wears until it
//! has its own.
//!
//! Written once here and added to each form by `templates::sources_for`,
//! rather than kept in every form: nine copies of a Code of Conduct are nine
//! places to forget the next fix. What genuinely differs by form - the gate in
//! CONTRIBUTING, the toolchains the audit covers - is chosen here by form, from
//! pieces that are each written once.

use crate::generate::{Contents, SourceFile};
use crate::model::{Addon, Form};

fn text(path: &'static str, contents: &'static str) -> SourceFile {
    SourceFile {
        path,
        contents: Contents::Text(contents),
        executable: false,
        addon: None,
    }
}

/// The paths every form that starts a repository writes, whatever else it
/// writes; the tests hold each form's tree to it.
#[cfg(test)]
const EVERY_FORM: &[&str] = &[
    "CONTRIBUTING.md",
    "SECURITY.md",
    "CODE_OF_CONDUCT.md",
    "llms.txt",
    ".github/ISSUE_TEMPLATE/bug_report.yml",
    ".github/ISSUE_TEMPLATE/feature_request.yml",
    ".github/ISSUE_TEMPLATE/config.yml",
    ".github/pull_request_template.md",
    ".github/workflows/audit.yml",
    "docs/adr/0001-record-architecture-decisions.md",
    "docs/adr/0002-dependabot-is-off.md",
    "docs/adr/README.md",
    "assets/logo.svg",
    "assets/logo-m.svg",
    "assets/logo-s.svg",
];

/// The files this module adds to a form. None for a form that adds to an
/// existing repository: that repository has its own.
pub fn sources(form: Form) -> Vec<SourceFile> {
    if form == Form::Docs {
        return Vec::new();
    }

    let mut files = vec![
        text("CONTRIBUTING.md", contributing(form)),
        text("SECURITY.md", include_str!("community/SECURITY.md.tmpl")),
        text("CODE_OF_CONDUCT.md", include_str!("community/CODE_OF_CONDUCT.md.tmpl")),
        text("llms.txt", include_str!("community/llms.txt.tmpl")),
        text(
            ".github/ISSUE_TEMPLATE/bug_report.yml",
            include_str!("community/github/ISSUE_TEMPLATE/bug_report.yml.tmpl"),
        ),
        text(
            ".github/ISSUE_TEMPLATE/feature_request.yml",
            include_str!("community/github/ISSUE_TEMPLATE/feature_request.yml.tmpl"),
        ),
        text(
            ".github/ISSUE_TEMPLATE/config.yml",
            include_str!("community/github/ISSUE_TEMPLATE/config.yml.tmpl"),
        ),
        text(
            ".github/pull_request_template.md",
            include_str!("community/github/pull_request_template.md.tmpl"),
        ),
        text(".github/workflows/audit.yml", audit(form)),
        text(
            "docs/adr/0001-record-architecture-decisions.md",
            include_str!("community/docs-adr-0001.md.tmpl"),
        ),
        text("docs/adr/0002-dependabot-is-off.md", include_str!("community/docs-adr-0002.md.tmpl")),
        text("docs/adr/README.md", adr_index(form)),
        // The masters of the mark: the line's umbrella mark at its three
        // levels, until the product has one of its own. Copies of dowel-ui's,
        // like the spa's favicon; the docs form's exporter draws every raster
        // from these.
        text("assets/logo.svg", include_str!("dowel/marks/lacodda-L.svg")),
        text("assets/logo-m.svg", include_str!("dowel/marks/lacodda-M.svg")),
        text("assets/logo-s.svg", include_str!("dowel/marks/lacodda-S.svg")),
    ];

    if has_crates(form) {
        files.push(text("deny.toml", include_str!("community/deny.toml.tmpl")));
    }
    match form {
        Form::Spa | Form::Mono | Form::Desktop | Form::TauriPlugin => {
            files.push(text("tools/check-licenses.mjs", include_str!("community/check-licenses.mjs.tmpl")));
        }
        // The service's packages are the web UI's, and exist only with it.
        Form::Service => files.push(SourceFile {
            addon: Some(Addon::Spa),
            ..text("frontend/tools/check-licenses.mjs", include_str!("community/check-licenses.mjs.tmpl"))
        }),
        _ => {}
    }
    files
}

/// Whether a form builds Rust, and so has crates for `cargo deny` to check.
pub fn has_crates(form: Form) -> bool {
    !matches!(form, Form::Spa | Form::Mono | Form::Docs)
}

/// CONTRIBUTING.md: the form's own gate and layout, then what every repository
/// of the line asks of a contributor.
fn contributing(form: Form) -> &'static str {
    match form {
        Form::Spa => concat!(
            include_str!("spa/CONTRIBUTING.md.tmpl"),
            include_str!("spa/contributing-ui.md.tmpl"),
            include_str!("spa/contributing-rest.md.tmpl"),
            include_str!("community/contributing.md.tmpl"),
        ),
        // The desktop's interface is the spa's, so what the spa says about
        // components and the accent is said once and read by both.
        Form::Desktop => concat!(
            include_str!("desktop/CONTRIBUTING.md.tmpl"),
            include_str!("spa/contributing-ui.md.tmpl"),
            include_str!("desktop/contributing-rest.md.tmpl"),
            include_str!("community/contributing.md.tmpl"),
        ),
        Form::Cli => concat!(include_str!("cli/CONTRIBUTING.md.tmpl"), include_str!("community/contributing.md.tmpl")),
        Form::Service => concat!(include_str!("service/CONTRIBUTING.md.tmpl"), include_str!("community/contributing.md.tmpl")),
        Form::Workspace => concat!(include_str!("workspace/CONTRIBUTING.md.tmpl"), include_str!("community/contributing.md.tmpl"),),
        Form::Mono => concat!(include_str!("mono/CONTRIBUTING.md.tmpl"), include_str!("community/contributing.md.tmpl")),
        Form::Plugin => concat!(include_str!("plugin/CONTRIBUTING.md.tmpl"), include_str!("community/contributing.md.tmpl")),
        Form::TauriPlugin => concat!(
            include_str!("tauri-plugin/CONTRIBUTING.md.tmpl"),
            include_str!("community/contributing.md.tmpl"),
        ),
        Form::Docs => unreachable!("the docs form adds to a repository and writes no CONTRIBUTING"),
    }
}

/// The audit workflow: a job per toolchain the form builds with, each where
/// that toolchain's manifest lives.
fn audit(form: Form) -> &'static str {
    match form {
        Form::Cli | Form::Workspace | Form::Plugin => {
            concat!(include_str!("community/audit/head.yml.tmpl"), include_str!("community/audit/crates.yml.tmpl"))
        }
        Form::Service => concat!(
            include_str!("community/audit/head.yml.tmpl"),
            include_str!("community/audit/crates.yml.tmpl"),
            include_str!("community/audit/packages-frontend.yml.tmpl"),
        ),
        Form::Spa | Form::Mono => {
            concat!(include_str!("community/audit/head.yml.tmpl"), include_str!("community/audit/packages.yml.tmpl"))
        }
        Form::Desktop => concat!(
            include_str!("community/audit/head.yml.tmpl"),
            include_str!("community/audit/packages.yml.tmpl"),
            include_str!("community/audit/crates-src-tauri.yml.tmpl"),
        ),
        Form::TauriPlugin => concat!(
            include_str!("community/audit/head.yml.tmpl"),
            include_str!("community/audit/crates.yml.tmpl"),
            include_str!("community/audit/packages.yml.tmpl"),
        ),
        Form::Docs => unreachable!("the docs form adds to a repository and writes no audit"),
    }
}

/// The index of `docs/adr/`: the two decisions every repository starts with,
/// then the form's own, numbered after them.
fn adr_index(form: Form) -> &'static str {
    match form {
        Form::Mono => concat!(
            include_str!("community/docs-adr-README.md.tmpl"),
            "| [0003](0003-the-package-is-built-by-tsc.md) | The package is built by tsc, resolved as nodenext | Accepted |\n",
        ),
        Form::Plugin => concat!(
            include_str!("community/docs-adr-README.md.tmpl"),
            "| [0003](0003-the-plugin-is-a-subprocess.md) | The plugin is a subprocess, not a library | Accepted |\n",
        ),
        Form::TauriPlugin => concat!(
            include_str!("community/docs-adr-README.md.tmpl"),
            "| [0003](0003-both-halves-ship-together.md) | Both halves ship from one repository | Accepted |\n",
        ),
        Form::Workspace => concat!(
            include_str!("community/docs-adr-README.md.tmpl"),
            "| [0003](0003-the-logic-lives-in-a-library-crate.md) | The logic lives in a library crate | Accepted |\n",
        ),
        _ => include_str!("community/docs-adr-README.md.tmpl"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::sources_for;

    #[test]
    fn every_form_that_starts_a_repository_writes_all_of_it() {
        for form in Form::ALL.iter().filter(|f| **f != Form::Docs) {
            let paths: Vec<&str> = sources_for(*form).iter().map(|s| s.path).collect();
            for path in EVERY_FORM {
                assert!(paths.contains(path), "{form} does not write `{path}`");
            }
            let deny = paths.contains(&"deny.toml");
            assert_eq!(deny, has_crates(*form), "{form}: deny.toml {}", if deny { "without crates" } else { "missing" });
        }
    }

    /// Every ADR a form writes is in its index, and the numbers run from 0001
    /// without a gap or a repeat - the two shared decisions first.
    #[test]
    fn the_adr_index_lists_every_decision_in_order() {
        for form in Form::ALL.iter().filter(|f| **f != Form::Docs) {
            let index = adr_index(*form);
            let mut decisions: Vec<&str> = sources_for(*form)
                .iter()
                .filter_map(|s| s.path.strip_prefix("docs/adr/"))
                .filter(|name| *name != "README.md")
                .collect();
            decisions.sort_unstable();
            for (position, file) in decisions.iter().enumerate() {
                let number = format!("{:04}", position + 1);
                assert!(file.starts_with(&number), "{form}: `{file}` is out of sequence, expected {number}");
                assert!(index.contains(&format!("[{number}]({file})")), "{form}: the ADR index does not list `{file}`");
            }
            let rows = index.lines().filter(|l| l.starts_with("| [")).count();
            assert_eq!(rows, decisions.len(), "{form}: the ADR index lists a decision the form does not write");
        }
    }

    /// The audit covers each toolchain the form builds with, and only those: a
    /// job for a manifest that is not there fails on its first run.
    #[test]
    fn the_audit_covers_the_toolchains_a_form_builds_with() {
        for form in Form::ALL.iter().filter(|f| **f != Form::Docs) {
            let workflow = audit(*form);
            let paths: Vec<&str> = sources_for(*form).iter().map(|s| s.path).collect();
            assert_eq!(workflow.contains("cargo-deny-action"), has_crates(*form), "{form}: crates job");
            let packages = paths.iter().any(|p| p.ends_with("package.json") && !p.starts_with("npm/"));
            assert_eq!(workflow.contains("pnpm audit --prod"), packages, "{form}: packages job");
            assert_eq!(
                workflow.contains("manifest-path: src-tauri/Cargo.toml"),
                *form == Form::Desktop,
                "{form}: the crates job looks for the manifest in the wrong place"
            );
        }
    }

    /// `deny.toml` and the npm license check hold dependencies to one policy;
    /// two lists that drifted would accept a license in one toolchain and
    /// refuse it in the other.
    #[test]
    fn crates_and_packages_are_held_to_the_same_licenses() {
        let deny: toml::Value = toml::from_str(include_str!("community/deny.toml.tmpl")).unwrap();
        let mut crates: Vec<String> = deny["licenses"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let script = include_str!("community/check-licenses.mjs.tmpl");
        let list = &script[script.find("const ALLOWED = new Set([").unwrap()..];
        let list = &list[..list.find("])").unwrap()];
        let mut packages: Vec<String> = list
            .lines()
            .filter_map(|line| line.trim().strip_prefix('\'')?.strip_suffix("',").map(String::from))
            .collect();
        crates.sort();
        packages.sort();
        assert!(!crates.is_empty());
        assert_eq!(crates, packages, "deny.toml and tools/check-licenses.mjs accept different licenses");
    }

    /// The masters are the umbrella mark's own files, level for level: an L
    /// master that was really the S tile is the mistake that put a flat blob
    /// in kilna's taskbar, and it would travel to every raster drawn from it.
    #[test]
    fn the_masters_are_the_umbrella_mark_at_their_levels() {
        let files = sources(Form::Cli);
        let body = |path: &str| match files.iter().find(|f| f.path == path).unwrap().contents {
            Contents::Text(text) => text,
            Contents::Binary(_) => panic!("{path} is not text"),
        };
        assert_eq!(body("assets/logo.svg"), include_str!("dowel/marks/lacodda-L.svg"));
        assert_eq!(body("assets/logo-m.svg"), include_str!("dowel/marks/lacodda-M.svg"));
        assert_eq!(body("assets/logo-s.svg"), include_str!("dowel/marks/lacodda-S.svg"));
        // The three are different drawings, not one file under three names.
        assert_ne!(body("assets/logo.svg"), body("assets/logo-s.svg"));
        assert_ne!(body("assets/logo-m.svg"), body("assets/logo-s.svg"));
    }

    #[test]
    fn every_gitignore_keeps_assistant_files_out() {
        let block = include_str!("community/gitignore.tmpl");
        for form in Form::ALL.iter().filter(|f| **f != Form::Docs) {
            let gitignore = sources_for(*form).into_iter().find(|s| s.path == ".gitignore").unwrap();
            let Contents::Text(text) = gitignore.contents else { panic!() };
            assert!(text.ends_with(block), "{form}: .gitignore does not keep assistant files out");
        }
    }
}
