//! Turning a template plus a context into a directory on disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{Addon, Context, TemplateManifest};
use crate::render;

/// One file the generator is about to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    /// Path relative to the project root, already rendered.
    pub path: PathBuf,
    pub contents: Vec<u8>,
    /// Whether the file needs the executable bit on Unix.
    pub executable: bool,
}

/// Everything a generation will do, before it does any of it.
///
/// Building the whole plan first is what makes `--dry-run` honest and keeps a
/// failure from leaving a half-written project behind.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub files: Vec<PlannedFile>,
}

impl Plan {
    /// The directories the plan implies, without duplicates.
    pub fn directories(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .files
            .iter()
            .filter_map(|f| f.path.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }

    /// A tree of what will be created, for `--dry-run` and for the wizard.
    pub fn tree(&self) -> String {
        let mut paths: Vec<String> = self.files.iter().map(|f| f.path.display().to_string().replace('\\', "/")).collect();
        paths.sort();
        paths.join("\n")
    }
}

/// What went wrong while planning or writing.
#[derive(Debug)]
pub enum GenerateError {
    /// The destination exists and is not empty.
    DestinationNotEmpty(PathBuf),
    Placeholder {
        file: PathBuf,
        source: render::UnknownPlaceholder,
    },
    Io(io::Error),
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateError::DestinationNotEmpty(p) => {
                write!(f, "`{}` already exists and is not empty", p.display())
            }
            GenerateError::Placeholder { file, source } => {
                write!(f, "in `{}`: {source}", file.display())
            }
            GenerateError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for GenerateError {}

impl From<io::Error> for GenerateError {
    fn from(e: io::Error) -> Self {
        GenerateError::Io(e)
    }
}

/// What a template file is made of: text that gets rendered, or bytes that do
/// not. An icon has no placeholders to fill and must not be read as UTF-8.
#[derive(Debug, Clone, Copy)]
pub enum Contents {
    Text(&'static str),
    Binary(&'static [u8]),
}

/// A template file as it lives inside the binary.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path relative to the template root, placeholders still in place.
    pub path: &'static str,
    pub contents: Contents,
    pub executable: bool,
    /// The add-on this file belongs to; `None` means the form always writes it.
    ///
    /// A whole file is the cheap case. Where an add-on only contributes a few
    /// lines to a shared file - a dependency, a match arm - the file carries
    /// `{{#addon}}` sections instead, so the template stays readable rather
    /// than turning into nested conditions.
    pub addon: Option<Addon>,
}

/// Strip the sections whose condition does not hold, and unwrap the ones whose
/// does: `{{#name}} ... {{/name}}` keeps its body when the add-on is enabled,
/// `{{^name}} ... {{/name}}` when it is not.
///
/// The markers live on their own lines and are removed with them, so a kept
/// section leaves no trace of ever having been conditional. The inverted form
/// exists because a file often needs a line either way in slightly different
/// shape - a `lint` script with and without the locale step - and duplicating
/// the whole file to say that would be worse.
fn apply_sections(text: &str, enabled: &[Addon]) -> String {
    let mut out = String::with_capacity(text.len());
    // Sections do not nest: an add-on is either on or off, and a section
    // inside a section of another add-on would be a way to express "both",
    // which the manifest says with two flags instead. The name is kept so a
    // stray closing marker cannot end a section it did not open.
    let mut skipping: Option<&str> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        if let Some(name) = trimmed.strip_prefix("{{#").and_then(|r| r.strip_suffix("}}")) {
            if !enabled.iter().any(|a| a.as_str() == name) {
                skipping = Some(name);
            }
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("{{^").and_then(|r| r.strip_suffix("}}")) {
            if enabled.iter().any(|a| a.as_str() == name) {
                skipping = Some(name);
            }
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("{{/").and_then(|r| r.strip_suffix("}}")) {
            if skipping == Some(name) {
                skipping = None;
            }
            continue;
        }
        if skipping.is_some() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    // A file that did not end in a newline should not gain one.
    if !text.ends_with('\n') {
        out.pop();
    }
    out
}

/// Render every source file into a plan, with no add-ons enabled.
#[cfg(test)]
pub fn plan(sources: &[SourceFile], manifest: &TemplateManifest, context: &Context) -> Result<Plan, GenerateError> {
    plan_with(sources, manifest, context, &[])
}

/// Render every source file into a plan, with the given add-ons enabled.
pub fn plan_with(sources: &[SourceFile], manifest: &TemplateManifest, context: &Context, addons: &[Addon]) -> Result<Plan, GenerateError> {
    let mut files = Vec::with_capacity(sources.len());

    for source in sources {
        // A file belonging to an add-on that was not asked for is simply not
        // written; nothing downstream has to know it exists.
        // `is_some_and` rather than a let-chain: those are stable from 1.88
        // and this crate promises 1.85.
        if source.addon.is_some_and(|required| !addons.contains(&required)) {
            continue;
        }

        // A path can carry placeholders too, so `src/{{ name }}.ts` works.
        let rendered_path = render::render(source.path, context).map_err(|source_err| GenerateError::Placeholder {
            file: PathBuf::from(source.path),
            source: source_err,
        })?;

        let verbatim = manifest.verbatim.iter().any(|pattern| pattern == source.path);
        let contents = match source.contents {
            Contents::Binary(bytes) => bytes.to_vec(),
            Contents::Text(text) if verbatim => text.as_bytes().to_vec(),
            Contents::Text(text) => {
                // Sections first: a section that is switched off must not have
                // its placeholders resolved, and may legitimately mention
                // variables that only make sense with that add-on enabled.
                let sectioned = apply_sections(text, addons);
                render::render(&sectioned, context)
                    .map_err(|source_err| GenerateError::Placeholder {
                        file: PathBuf::from(source.path),
                        source: source_err,
                    })?
                    .into_bytes()
            }
        };

        files.push(PlannedFile {
            path: PathBuf::from(rendered_path),
            contents,
            executable: source.executable,
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Plan { files })
}

/// A destination is usable when it does not exist, or exists and is empty.
pub fn check_destination(root: &Path) -> Result<(), GenerateError> {
    match fs::read_dir(root) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                Err(GenerateError::DestinationNotEmpty(root.to_path_buf()))
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(GenerateError::Io(e)),
    }
}

/// Write the plan under `root`.
pub fn write(plan: &Plan, root: &Path) -> Result<(), GenerateError> {
    fs::create_dir_all(root)?;
    for dir in plan.directories() {
        fs::create_dir_all(root.join(dir))?;
    }
    for file in &plan.files {
        let target = root.join(&file.path);
        fs::write(&target, &file.contents)?;
        set_executable(&target, file.executable)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !executable {
        return Ok(());
    }
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> io::Result<()> {
    // Windows has no executable bit; git carries the mode instead.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        let mut c = Context::new();
        c.set("name", "demo-app");
        c
    }

    fn sources() -> Vec<SourceFile> {
        vec![
            SourceFile {
                path: "README.md",
                contents: Contents::Text("# {{ name }}"),
                executable: false,
                addon: None,
            },
            SourceFile {
                path: "src/main.ts",
                contents: Contents::Text("export const n = '{{ name }}'"),
                executable: false,
                addon: None,
            },
        ]
    }

    #[test]
    fn renders_contents_into_the_plan() {
        let plan = plan(&sources(), &TemplateManifest::default(), &ctx()).unwrap();
        let readme = plan.files.iter().find(|f| f.path == Path::new("README.md")).unwrap();
        assert_eq!(readme.contents, b"# demo-app");
    }

    #[test]
    fn renders_placeholders_in_paths() {
        let sources = [SourceFile {
            path: "src/{{ name }}.ts",
            contents: Contents::Text("x"),
            executable: false,
            addon: None,
        }];
        let plan = plan(&sources, &TemplateManifest::default(), &ctx()).unwrap();
        assert_eq!(plan.files[0].path, Path::new("src/demo-app.ts"));
    }

    #[test]
    fn a_verbatim_file_keeps_its_braces() {
        let sources = [SourceFile {
            path: "keep.md",
            contents: Contents::Text("{{ name }}"),
            executable: false,
            addon: None,
        }];
        let manifest = TemplateManifest {
            verbatim: vec!["keep.md".into()],
            ..Default::default()
        };
        let plan = plan(&sources, &manifest, &ctx()).unwrap();
        assert_eq!(plan.files[0].contents, b"{{ name }}");
    }

    #[test]
    fn an_unknown_placeholder_names_the_file_it_is_in() {
        let sources = [SourceFile {
            path: "bad.md",
            contents: Contents::Text("{{ nope }}"),
            executable: false,
            addon: None,
        }];
        let err = plan(&sources, &TemplateManifest::default(), &ctx()).unwrap_err();
        match err {
            GenerateError::Placeholder { file, source } => {
                assert_eq!(file, Path::new("bad.md"));
                assert_eq!(source.key, "nope");
            }
            other => panic!("expected a placeholder error, got {other:?}"),
        }
    }

    #[test]
    fn the_plan_lists_the_directories_it_needs() {
        let plan = plan(&sources(), &TemplateManifest::default(), &ctx()).unwrap();
        assert_eq!(plan.directories(), vec![PathBuf::from("src")]);
    }

    #[test]
    fn the_tree_is_sorted_and_uses_forward_slashes() {
        let plan = plan(&sources(), &TemplateManifest::default(), &ctx()).unwrap();
        assert_eq!(plan.tree(), "README.md\nsrc/main.ts");
    }

    #[test]
    fn writes_the_plan_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-app");
        let plan = plan(&sources(), &TemplateManifest::default(), &ctx()).unwrap();
        write(&plan, &root).unwrap();
        assert_eq!(fs::read_to_string(root.join("README.md")).unwrap(), "# demo-app");
        assert_eq!(fs::read_to_string(root.join("src/main.ts")).unwrap(), "export const n = 'demo-app'");
    }

    #[test]
    fn a_file_of_an_unrequested_addon_is_not_written() {
        let sources = [SourceFile {
            path: "secret.rs",
            contents: Contents::Text("x"),
            executable: false,
            addon: Some(Addon::Keyring),
        }];
        let plan = plan_with(&sources, &TemplateManifest::default(), &ctx(), &[]).unwrap();
        assert!(plan.files.is_empty());
    }

    #[test]
    fn a_file_of_a_requested_addon_is_written() {
        let sources = [SourceFile {
            path: "secret.rs",
            contents: Contents::Text("x"),
            executable: false,
            addon: Some(Addon::Keyring),
        }];
        let plan = plan_with(&sources, &TemplateManifest::default(), &ctx(), &[Addon::Keyring]).unwrap();
        assert_eq!(plan.files.len(), 1);
    }

    #[test]
    fn an_addon_asked_for_does_not_bring_in_another() {
        let sources = [SourceFile {
            path: "update.rs",
            contents: Contents::Text("x"),
            executable: false,
            addon: Some(Addon::SelfUpdate),
        }];
        let plan = plan_with(&sources, &TemplateManifest::default(), &ctx(), &[Addon::Keyring]).unwrap();
        assert!(plan.files.is_empty(), "keyring dragged in the self-update file");
    }

    #[test]
    fn a_section_of_an_unrequested_addon_is_cut_out() {
        let text = "keep
{{#keyring}}
secret
{{/keyring}}
tail
";
        assert_eq!(
            apply_sections(text, &[]),
            "keep
tail
"
        );
    }

    #[test]
    fn a_section_of_a_requested_addon_is_unwrapped() {
        let text = "keep
{{#keyring}}
secret
{{/keyring}}
tail
";
        // The markers go with their lines: an enabled section leaves no trace
        // of ever having been conditional.
        assert_eq!(
            apply_sections(text, &[Addon::Keyring]),
            "keep
secret
tail
"
        );
    }

    #[test]
    fn sections_of_different_addons_are_decided_separately() {
        let text = "{{#keyring}}
a
{{/keyring}}
{{#self-update}}
b
{{/self-update}}
";
        assert_eq!(
            apply_sections(text, &[Addon::SelfUpdate]),
            "b
"
        );
    }

    #[test]
    fn an_indented_marker_is_still_a_marker() {
        // A section inside an indented block - a match arm, a TOML table -
        // must not depend on sitting at column zero.
        let text = "    {{#keyring}}
    a
    {{/keyring}}
";
        assert_eq!(
            apply_sections(text, &[Addon::Keyring]),
            "    a
"
        );
    }

    #[test]
    fn an_inverted_section_is_kept_when_the_addon_is_off() {
        let text = "a
{{^i18n}}
plain
{{/i18n}}
b
";
        assert_eq!(
            apply_sections(text, &[]),
            "a
plain
b
"
        );
    }

    #[test]
    fn an_inverted_section_is_cut_when_the_addon_is_on() {
        let text = "a
{{^i18n}}
plain
{{/i18n}}
b
";
        assert_eq!(
            apply_sections(text, &[Addon::I18n]),
            "a
b
"
        );
    }

    #[test]
    fn the_two_forms_of_a_section_are_exclusive() {
        // The pattern a file uses to say "this line either way, in two
        // shapes": exactly one of the pair survives, never both or neither.
        let text = "{{#i18n}}
with
{{/i18n}}
{{^i18n}}
without
{{/i18n}}
";
        assert_eq!(
            apply_sections(text, &[Addon::I18n]),
            "with
"
        );
        assert_eq!(
            apply_sections(text, &[]),
            "without
"
        );
    }

    #[test]
    fn a_file_without_sections_is_untouched() {
        assert_eq!(
            apply_sections(
                "a
b
",
                &[Addon::Keyring]
            ),
            "a
b
"
        );
    }

    #[test]
    fn a_cut_section_leaves_its_placeholders_unresolved() {
        // A switched-off section may mention variables that only exist with
        // that add-on; resolving them would fail the whole generation.
        let sources = [SourceFile {
            path: "Cargo.toml",
            contents: Contents::Text(
                "[deps]
{{#keyring}}
keyring = \"{{ nonexistent }}\"
{{/keyring}}
",
            ),
            executable: false,
            addon: None,
        }];
        let plan = plan_with(&sources, &TemplateManifest::default(), &ctx(), &[]).unwrap();
        assert_eq!(
            plan.files[0].contents,
            b"[deps]
"
        );
    }

    #[test]
    fn a_missing_destination_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_destination(&dir.path().join("not-there")).is_ok());
    }

    #[test]
    fn an_empty_destination_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_destination(dir.path()).is_ok());
    }

    #[test]
    fn a_populated_destination_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("occupied.txt"), "x").unwrap();
        let err = check_destination(dir.path()).unwrap_err();
        assert!(matches!(err, GenerateError::DestinationNotEmpty(_)));
    }
}
