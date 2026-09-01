//! Turning a template plus a context into a directory on disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{Context, TemplateManifest};
use crate::render;

/// One file the generator is about to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    /// Path relative to the project root, already rendered.
    pub path: PathBuf,
    pub contents: String,
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

/// A template file as it lives inside the binary.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path relative to the template root, placeholders still in place.
    pub path: &'static str,
    pub contents: &'static str,
    pub executable: bool,
}

/// Render every source file into a plan.
pub fn plan(sources: &[SourceFile], manifest: &TemplateManifest, context: &Context) -> Result<Plan, GenerateError> {
    let mut files = Vec::with_capacity(sources.len());

    for source in sources {
        // A path can carry placeholders too, so `src/{{ name }}.ts` works.
        let rendered_path = render::render(source.path, context).map_err(|source_err| GenerateError::Placeholder {
            file: PathBuf::from(source.path),
            source: source_err,
        })?;

        let verbatim = manifest.verbatim.iter().any(|pattern| pattern == source.path);
        let contents = if verbatim {
            source.contents.to_string()
        } else {
            render::render(source.contents, context).map_err(|source_err| GenerateError::Placeholder {
                file: PathBuf::from(source.path),
                source: source_err,
            })?
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
                contents: "# {{ name }}",
                executable: false,
            },
            SourceFile {
                path: "src/main.ts",
                contents: "export const n = '{{ name }}'",
                executable: false,
            },
        ]
    }

    #[test]
    fn renders_contents_into_the_plan() {
        let plan = plan(&sources(), &TemplateManifest::default(), &ctx()).unwrap();
        let readme = plan.files.iter().find(|f| f.path == Path::new("README.md")).unwrap();
        assert_eq!(readme.contents, "# demo-app");
    }

    #[test]
    fn renders_placeholders_in_paths() {
        let sources = [SourceFile {
            path: "src/{{ name }}.ts",
            contents: "x",
            executable: false,
        }];
        let plan = plan(&sources, &TemplateManifest::default(), &ctx()).unwrap();
        assert_eq!(plan.files[0].path, Path::new("src/demo-app.ts"));
    }

    #[test]
    fn a_verbatim_file_keeps_its_braces() {
        let sources = [SourceFile {
            path: "keep.md",
            contents: "{{ name }}",
            executable: false,
        }];
        let manifest = TemplateManifest {
            verbatim: vec!["keep.md".into()],
            ..Default::default()
        };
        let plan = plan(&sources, &manifest, &ctx()).unwrap();
        assert_eq!(plan.files[0].contents, "{{ name }}");
    }

    #[test]
    fn an_unknown_placeholder_names_the_file_it_is_in() {
        let sources = [SourceFile {
            path: "bad.md",
            contents: "{{ nope }}",
            executable: false,
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
