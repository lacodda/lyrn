//! `lyrn adopt`: an existing repository brought up to the line's standard.
//!
//! The line has repositories older than lyrn, and repositories started by
//! hand. `adopt` reads what one already is - its form, its name, its
//! description, where it lives - and adds the files of the standard it does
//! not have yet: licence, changelog, editor and git settings, the CI gate, the
//! first ADR, and `lyrn.toml`, which is what makes it a lyrn project that
//! `doctor` and `upgrade` can later read.
//!
//! It never replaces a file. Unlike a form added to a repository, it does not
//! refuse when some are present either: each file of the standard stands on
//! its own, so the ones already there are kept, the rest are added, and the
//! tree shown beforehand says which is which. A file under another common
//! name - `LICENSE.md`, an ADR 0001 about something else - counts as present.

use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::{AdoptArgs, IdentityArgs};
use crate::commands::init::name_of;
use crate::commands::new::{Wanted, build_context};
use crate::generate::{self, Plan};
use crate::host;
use crate::model::{Form, TemplateManifest};
use crate::naming;
use crate::templates;

pub fn run(args: AdoptArgs) -> Result<(), Box<dyn Error>> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!(
            "`{}` is not a directory - `lyrn adopt` brings an existing repository up to the standard",
            root.display()
        )
        .into());
    }

    let facts = Facts::read(&root)?;
    let (form, detected_host) = match args.form.or(facts.lyrn_form) {
        Some(form) => (form, None),
        None => detect(&root, &facts)?,
    };
    if !form.adoptable() {
        return Err(format!("a repository is not a `{form}`: a documentation site is added to one with `lyrn new <name> --form docs`").into());
    }

    let name = match args.name.clone().or_else(|| facts.name.clone()) {
        Some(name) => {
            naming::validate_name(&name).map_err(|reason| format!("the project is called `{name}`, which is not a project name: {reason} - pass `--name`"))?;
            name
        }
        None => name_of(&root)?,
    };

    // What the repository already says about itself wins over a default, and
    // anything given on the command line wins over both.
    let identity = IdentityArgs {
        accent: args
            .identity
            .accent
            .clone()
            .or_else(|| facts.accent.clone())
            .or_else(|| naming::product_accent(&name).map(str::to_string)),
        description: args.identity.description.clone().or_else(|| facts.description.clone()),
        author: args.identity.author.clone(),
        repo: args.identity.repo.clone().or_else(|| origin_repo(&root)),
    };
    let host = args.host.as_deref().or(detected_host);
    let wanted = Wanted {
        name: &name,
        form,
        host,
        with: &[],
        identity: &identity,
    };

    let manifest: TemplateManifest = toml::from_str(templates::manifest_for(form))?;
    let interactive = !args.assume_yes && std::io::stdin().is_terminal();
    let context = build_context(&wanted, &manifest, interactive)?;
    let standard = generate::plan_with(&templates::standard_sources(form), &manifest, &context, &[])?;

    let mut missing = Plan::default();
    let mut notes: Vec<(PathBuf, Option<String>)> = Vec::new();
    for file in standard.files {
        match already_there(&root, &file.path) {
            Some(note) => notes.push((file.path.clone(), Some(note))),
            None => {
                notes.push((file.path.clone(), None));
                missing.files.push(file);
            }
        }
    }
    let kept = notes.len() - missing.files.len();

    if missing.files.is_empty() {
        println!("Nothing to add: `{}` already carries every file of the `{form}` standard.", root.display());
        return Ok(());
    }

    let tree = generate::draw_tree(notes.iter().map(|(path, note)| (path.as_path(), note.as_deref())));
    let heading = format!(
        "{} to `{}` as a `{form}` project; {kept} of its standard {} already there:",
        plural(missing.files.len()),
        root.display(),
        if kept == 1 { "is" } else { "are" }
    );

    if args.dry_run {
        println!("Would add {heading}\n");
        println!("{}", indent(&tree));
        return Ok(());
    }
    if interactive {
        println!("\nWill add {heading}\n");
        println!("{}\n", indent(&tree));
        if !confirm("Add them?")? {
            println!("Nothing was written.");
            return Ok(());
        }
    }

    generate::write(&missing, &root)?;
    println!("Added {} to `{}`.", plural(missing.files.len()), root.display());

    println!("\nNext:");
    println!("  git status    # what was added, before it is committed");
    if !has_a_docs_site(&root) {
        println!("  lyrn new {name} --form docs    # it has no documentation site");
    }
    Ok(())
}

/// What a repository already says about itself.
#[derive(Debug, Default)]
struct Facts {
    name: Option<String>,
    description: Option<String>,
    accent: Option<String>,
    lyrn_form: Option<Form>,
    cargo: Option<toml::Table>,
    package: Option<serde_json::Value>,
}

impl Facts {
    fn read(root: &Path) -> Result<Self, Box<dyn Error>> {
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
            let lyrn: toml::Table = toml::from_str(&text).map_err(|e| format!("lyrn.toml does not parse: {e}"))?;
            let project = lyrn.get("project").and_then(|p| p.as_table());
            let field = |key: &str| project.and_then(|p| p.get(key)).and_then(|v| v.as_str()).map(str::to_string);
            if let Some(form) = field("form") {
                facts.lyrn_form = Some(form.parse().map_err(|e| format!("lyrn.toml: {e}"))?);
            }
            facts.name = field("name").or(facts.name);
            facts.accent = field("accent");
        }
        Ok(facts)
    }

    fn cargo_depends_on(&self, krate: &str) -> bool {
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

    fn package_depends_on(&self, name: &str) -> bool {
        let Some(package) = &self.package else { return false };
        ["dependencies", "devDependencies"]
            .iter()
            .filter_map(|key| package.get(*key).and_then(|d| d.as_object()))
            .any(|deps| deps.contains_key(name))
    }
}

/// The form a repository has, read from what is in it.
///
/// Most specific first: a Tauri app is also a Cargo project and a Vite app,
/// and a service is also a binary crate. A plugin is known by its name, which
/// is how its host finds it too.
fn detect(root: &Path, facts: &Facts) -> Result<(Form, Option<&'static str>), Box<dyn Error>> {
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
fn already_there(root: &Path, path: &Path) -> Option<String> {
    if root.join(path).exists() {
        return Some("already there".to_string());
    }
    let posix = path.display().to_string().replace('\\', "/");
    let others: &[&str] = match posix.as_str() {
        "README.md" => &["README", "README.txt", "README.rst", "readme.md"],
        "LICENSE" => &["LICENSE.md", "LICENSE.txt", "LICENSE-MIT", "LICENCE", "COPYING"],
        "CHANGELOG.md" => &["CHANGELOG", "CHANGES.md", "HISTORY.md"],
        ".github/workflows/ci.yml" => &[".github/workflows/ci.yaml"],
        _ => &[],
    };
    if let Some(other) = others.iter().find(|o| root.join(o).exists()) {
        return Some(format!("`{other}` is there"));
    }
    // Numbering is what an ADR is found by: a second 0001 beside the one the
    // repository already has would make "ADR 1" mean two things.
    if posix == "docs/adr/0001-record-architecture-decisions.md" {
        for dir in ["docs/adr", "doc/adr"] {
            let Ok(entries) = std::fs::read_dir(root.join(dir)) else { continue };
            let mut firsts: Vec<String> = entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("0001"))
                .collect();
            firsts.sort();
            if let Some(first) = firsts.first() {
                return Some(format!("`{dir}/{first}` is there"));
            }
        }
    }
    None
}

/// Whether the repository already has a documentation site, of any kind.
fn has_a_docs_site(root: &Path) -> bool {
    root.join("docs").join("astro.config.mjs").is_file() || templates::docs::FOREIGN_SITES.iter().any(|m| root.join(m).exists())
}

/// `owner/name` of the GitHub repository `origin` points at.
fn origin_repo(root: &Path) -> Option<String> {
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

fn plural(count: usize) -> String {
    if count == 1 { "1 file".to_string() } else { format!("{count} files") }
}

fn indent(text: &str) -> String {
    text.lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n")
}

fn confirm(prompt: &str) -> Result<bool, Box<dyn Error>> {
    use dialoguer::{Confirm, theme::ColorfulTheme};
    Ok(Confirm::with_theme(&ColorfulTheme::default()).with_prompt(prompt).default(true).interact()?)
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
    fn a_first_adr_of_its_own_counts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs/adr")).unwrap();
        std::fs::write(dir.path().join("docs/adr/0001-use-rust.md"), "# Use Rust").unwrap();
        let adr = Path::new("docs/adr/0001-record-architecture-decisions.md");
        assert_eq!(already_there(dir.path(), adr).as_deref(), Some("`docs/adr/0001-use-rust.md` is there"));
        // The index is its own file: a repository with ADRs and no index
        // still gets one.
        assert_eq!(already_there(dir.path(), Path::new("docs/adr/README.md")), None);
    }
}
