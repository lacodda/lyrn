//! What a repository already says about itself.
//!
//! `adopt` reads it to decide what to add, and `doctor` to decide what to
//! hold the repository to: its form, its name, its manifests, where it lives,
//! and which files of the standard it keeps under which names.

use std::error::Error;
use std::path::Path;
use std::process::Command as Process;

use crate::host;
use crate::model::Form;
use crate::templates;

/// What a repository already says about itself.
#[derive(Debug, Default)]
pub struct Facts {
    pub name: Option<String>,
    pub description: Option<String>,
    pub accent: Option<String>,
    pub lyrn_form: Option<Form>,
    /// `[project] mark` in `lyrn.toml`: `placeholder` or `chosen`.
    pub mark: Option<String>,
    /// `[template] standard` in `lyrn.toml`: the revision of the standard the
    /// repository was generated or adopted at.
    pub standard: Option<String>,
    /// Whether there is a `lyrn.toml` at all.
    pub has_lyrn_toml: bool,
    pub cargo: Option<toml::Table>,
    pub package: Option<serde_json::Value>,
}

impl Facts {
    pub fn read(root: &Path) -> Result<Self, Box<dyn Error>> {
        let mut facts = Facts::default();

        // A manifest that exists and does not parse is reported, not skipped:
        // reading past it would adopt the repository as whatever the next
        // guess says it is.
        if let Some(text) = read(&root.join("Cargo.toml")) {
            facts.cargo = Some(toml::from_str(&text).map_err(|e| format!("Cargo.toml does not parse: {e}"))?);
        }
        if let Some(text) = read(&root.join("package.json")) {
            facts.package = Some(serde_json::from_str(&text).map_err(|e| format!("package.json does not parse: {e}"))?);
        }

        let cargo_package = facts.cargo.as_ref().and_then(|c| c.get("package")).and_then(|p| p.as_table());
        let cargo_string = |key: &str| cargo_package.and_then(|p| p.get(key)).and_then(|v| v.as_str()).map(str::to_string);
        let package_string = |key: &str| facts.package.as_ref().and_then(|p| p.get(key)).and_then(|v| v.as_str()).map(str::to_string);

        // An npm scope is where a package is published, not what it is called.
        facts.name = cargo_string("name").or_else(|| package_string("name").map(|n| n.rsplit('/').next().unwrap_or(&n).to_string()));
        facts.description = cargo_string("description")
            .or_else(|| package_string("description"))
            .filter(|d| !d.trim().is_empty());

        if let Some(text) = read(&root.join("lyrn.toml")) {
            facts.has_lyrn_toml = true;
            let lyrn: toml::Table = toml::from_str(&text).map_err(|e| format!("lyrn.toml does not parse: {e}"))?;
            let section = |name: &str| lyrn.get(name).and_then(|p| p.as_table());
            let field = |table: &str, key: &str| section(table).and_then(|p| p.get(key)).and_then(|v| v.as_str()).map(str::to_string);
            if let Some(form) = field("project", "form") {
                facts.lyrn_form = Some(form.parse().map_err(|e| format!("lyrn.toml: {e}"))?);
            }
            facts.name = field("project", "name").or(facts.name);
            facts.accent = field("project", "accent");
            facts.mark = field("project", "mark");
            facts.standard = field("template", "standard");
        }
        Ok(facts)
    }

    pub fn cargo_depends_on(&self, krate: &str) -> bool {
        let Some(cargo) = &self.cargo else { return false };
        // Platform-specific tables count too: a Windows-only dependency is
        // still what the crate is built on.
        let per_target = cargo
            .get("target")
            .and_then(|t| t.as_table())
            .into_iter()
            .flat_map(|targets| targets.values().filter_map(|t| t.as_table()));
        std::iter::once(cargo)
            .chain(per_target)
            .flat_map(|table| ["dependencies", "build-dependencies"].map(|key| table.get(key).and_then(|t| t.as_table())))
            .flatten()
            .any(|table| table.contains_key(krate))
    }

    pub fn package_depends_on(&self, name: &str) -> bool {
        let Some(package) = &self.package else { return false };
        ["dependencies", "devDependencies"]
            .iter()
            .filter_map(|key| package.get(*key).and_then(|d| d.as_object()))
            .any(|deps| deps.contains_key(name))
    }

    /// The form the repository says it has, or the one read from what is in
    /// it - and the host, when it is a plugin known by its name.
    pub fn form(&self, root: &Path) -> Result<(Form, Option<&'static str>), Box<dyn Error>> {
        match self.lyrn_form {
            Some(form) => Ok((form, None)),
            None => detect(root, self),
        }
    }
}

/// The form a repository has, read from what is in it.
///
/// Most specific first: a Tauri app is also a Cargo project and a Vite app,
/// and a service is also a binary crate. A plugin is known by its name, which
/// is how its host finds it too.
pub fn detect(root: &Path, facts: &Facts) -> Result<(Form, Option<&'static str>), Box<dyn Error>> {
    if root.join("src-tauri").join("tauri.conf.json").is_file() {
        return Ok((Form::Desktop, None));
    }
    if let Some(cargo) = &facts.cargo {
        if cargo.contains_key("workspace") {
            return Ok((Form::Workspace, None));
        }
        if facts.cargo_depends_on("tauri-plugin") {
            return Ok((Form::TauriPlugin, None));
        }
        if facts.cargo_depends_on("axum") {
            return Ok((Form::Service, None));
        }
        let name = facts.name.as_deref().unwrap_or_default();
        if let Some(host) = host::ALL.iter().find(|h| name.starts_with(&h.prefix())) {
            return Ok((Form::Plugin, Some(host.name)));
        }
        // Cargo finds binaries in `src/main.rs` and in `src/bin/`, or is told
        // of them in `[[bin]]`; the tool may be a window rather than a
        // terminal program, and the standard it needs is the same.
        if root.join("src").join("main.rs").is_file() || root.join("src").join("bin").is_dir() || cargo.contains_key("bin") {
            return Ok((Form::Cli, None));
        }
        return Err("this is a library crate, and lyrn has no form for one on its own - pass `--form` if it is one of `lyrn forms`".into());
    }
    if facts.package.is_some() {
        if root.join("pnpm-workspace.yaml").is_file() && root.join("packages").is_dir() {
            return Ok((Form::Mono, None));
        }
        if facts.package_depends_on("vite") {
            return Ok((Form::Spa, None));
        }
    }
    Err(format!(
        "cannot tell what `{}` is - no lyrn.toml, Cargo.toml, or package.json on Vite - pass `--form` (one of: {})",
        root.display(),
        Form::ALL.iter().filter(|f| f.adoptable()).map(|f| f.as_str()).collect::<Vec<_>>().join(", ")
    )
    .into())
}

/// Whether the repository has the file of the standard at `path`, under that
/// name or another it is commonly kept under. The note says which.
pub fn already_there(root: &Path, path: &Path) -> Option<String> {
    if root.join(path).exists() {
        return Some("already there".to_string());
    }
    let posix = path.display().to_string().replace('\\', "/");
    let others: &[&str] = match posix.as_str() {
        "README.md" => &["README", "README.txt", "README.rst", "readme.md"],
        "LICENSE" => &["LICENSE.md", "LICENSE.txt", "LICENSE-MIT", "LICENCE", "COPYING"],
        "CHANGELOG.md" => &["CHANGELOG", "CHANGES.md", "HISTORY.md"],
        ".github/workflows/ci.yml" => &[".github/workflows/ci.yaml"],
        ".github/workflows/audit.yml" => &[".github/workflows/audit.yaml"],
        _ => &[],
    };
    if let Some(other) = others.iter().find(|o| root.join(o).exists()) {
        return Some(format!("`{other}` is there"));
    }
    // The decisions are one sequence, so they are adopted as one: into a
    // repository that keeps none, all of them; into one that keeps its own,
    // none. Numbering is what an ADR is found by - a second 0002 beside the
    // one already there would make "ADR 2" mean two things - and an index
    // written for the standard's sequence would list somebody else's wrongly.
    if posix.starts_with("docs/adr/") {
        for dir in ["docs/adr", "doc/adr"] {
            let Ok(entries) = std::fs::read_dir(root.join(dir)) else { continue };
            let mut kept: Vec<String> = entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".md"))
                .collect();
            kept.sort();
            if let Some(first) = kept.first() {
                return Some(format!("`{dir}/` keeps its own, from `{first}`"));
            }
        }
    }
    None
}

/// Whether the repository already has a documentation site, of any kind.
pub fn has_a_docs_site(root: &Path) -> bool {
    root.join("docs").join("astro.config.mjs").is_file() || templates::docs::FOREIGN_SITES.iter().any(|m| root.join(m).exists())
}

/// `owner/name` of the GitHub repository `origin` points at.
pub fn origin_repo(root: &Path) -> Option<String> {
    let out = Process::new("git").args(["remote", "get-url", "origin"]).current_dir(root).output().ok()?;
    if !out.status.success() {
        return None;
    }
    github_repo(String::from_utf8_lossy(&out.stdout).trim())
}

/// `owner/name` out of a GitHub remote, over HTTPS or SSH.
fn github_repo(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let (owner, name) = rest.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| format!("{owner}/{name}"))
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_remote_names_its_repository() {
        for url in [
            "https://github.com/lacodda/lyrn.git",
            "https://github.com/lacodda/lyrn",
            "git@github.com:lacodda/lyrn.git",
            "ssh://git@github.com/lacodda/lyrn.git",
        ] {
            assert_eq!(github_repo(url).as_deref(), Some("lacodda/lyrn"), "{url}");
        }
    }

    /// A remote elsewhere is not guessed into a GitHub one: the installers
    /// would then point at a repository that does not exist.
    #[test]
    fn a_remote_that_is_not_github_names_nothing() {
        assert_eq!(github_repo("https://gitlab.com/lacodda/lyrn.git"), None);
        assert_eq!(github_repo("https://github.com/lacodda"), None);
    }

    #[test]
    fn a_licence_under_another_name_counts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("LICENSE.md"), "MIT").unwrap();
        assert_eq!(already_there(dir.path(), Path::new("LICENSE")).as_deref(), Some("`LICENSE.md` is there"));
        assert_eq!(already_there(dir.path(), Path::new("CHANGELOG.md")), None);
    }

    #[test]
    fn decisions_of_its_own_count_for_the_whole_sequence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/adr")).unwrap();
        std::fs::write(dir.path().join("docs/adr/0001-use-rust.md"), "# Use Rust").unwrap();
        // Every file of the standard's sequence - a decision, the one after
        // it, and the index of both - is answered by the repository's own.
        for standard in ["0001-record-architecture-decisions.md", "0002-dependabot-is-off.md", "README.md"] {
            assert_eq!(
                already_there(dir.path(), &Path::new("docs/adr").join(standard)).as_deref(),
                Some("`docs/adr/` keeps its own, from `0001-use-rust.md`"),
                "{standard}"
            );
        }
        // Anything outside the decisions is decided on its own.
        assert_eq!(already_there(dir.path(), Path::new("SECURITY.md")), None);
    }

    #[test]
    fn a_repository_without_decisions_gets_them() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(already_there(dir.path(), Path::new("docs/adr/0002-dependabot-is-off.md")), None);
        // An empty directory keeps nothing.
        std::fs::create_dir_all(dir.path().join("docs/adr")).unwrap();
        assert_eq!(already_there(dir.path(), Path::new("docs/adr/README.md")), None);
    }

    #[test]
    fn lyrn_toml_says_the_mark_and_the_standard() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("lyrn.toml"),
            "[project]\nname = \"demo\"\nform = \"cli\"\nmark = \"placeholder\"\n\n[template]\nstandard = \"2026.10\"\n",
        )
        .unwrap();
        let facts = Facts::read(dir.path()).unwrap();
        assert!(facts.has_lyrn_toml);
        assert_eq!(facts.lyrn_form, Some(Form::Cli));
        assert_eq!(facts.mark.as_deref(), Some("placeholder"));
        assert_eq!(facts.standard.as_deref(), Some("2026.10"));
    }
}
