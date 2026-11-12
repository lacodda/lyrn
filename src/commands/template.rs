//! `lyrn template`: checking a template, and writing a built-in form out as
//! one.
//!
//! `check` is the gate a template repository's CI runs before it generates
//! anything: the same checks lyrn holds its built-in forms to, and the same
//! ones it runs before using a template from outside - so a template that
//! passes here is one `lyrn new --template` accepts.
//!
//! `export` is how a template repository is born: the built-in form's files,
//! placeholders intact, under `template/`, with a manifest that says what the
//! code of the binary says about them, and a CI that generates every
//! combination of its add-ons and runs the gate each project ships with.

use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::cli::TemplateCommand;
use crate::commands::new;
use crate::generate::{self, Plan, PlannedFile};
use crate::model::{AddonManifest, Context, Form};
use crate::template::{self, FILES_DIR, Kind, Origin, VARIABLES};
use crate::templates;

mod follow;

pub fn run(command: TemplateCommand) -> Result<(), Box<dyn Error>> {
    match command {
        TemplateCommand::Check { path } => check(&path.unwrap_or_else(|| PathBuf::from("."))),
        TemplateCommand::Export { form, path, repo } => {
            let root = path.unwrap_or_else(|| PathBuf::from(format!("template-{form}")));
            export(form, &root, repo.as_deref())
        }
        TemplateCommand::Follow { paths, dry_run } => follow::run(&paths, dry_run),
    }
}

/// Read a template the way `lyrn new --template` reads one, and render it
/// with none and with all of its add-ons: a problem found here is a problem
/// its users would have met.
fn check(dir: &Path) -> Result<(), Box<dyn Error>> {
    let template = template::load_dir(dir, Origin::Path(dir.to_path_buf()))?;
    match &template.kind {
        Kind::Lyrn(native) => {
            let context = sample_context(native.form);
            let everything: Vec<String> = native.addons.iter().map(|a| a.name.clone()).collect();
            for addons in [&[][..], &everything[..]] {
                generate::plan_with(&native.files, &native.manifest, &context, addons)?;
            }
            let addons = if everything.is_empty() {
                "no add-ons".to_string()
            } else {
                format!("add-ons: {}", everything.join(", "))
            };
            println!(
                "`{}` is a lyrn template of the {} form: {} files, {addons}.",
                dir.display(),
                native.form,
                native.files.len()
            );
        }
        Kind::CargoGenerate(_) => {
            println!("`{}` is a cargo-generate template lyrn can read.", dir.display());
        }
    }
    Ok(())
}

/// A context that fills every variable, for rendering a template to see that
/// it renders. The values are the variables' own names, so nothing in them
/// can collide with a path or a section.
fn sample_context(form: Form) -> Context {
    let mut context = Context::new();
    for variable in VARIABLES {
        context.set(*variable, variable.replace('_', "-"));
    }
    // A number where a file reads one: turnout.json is JSON.
    context.set("name", "demo-app").set("form", form.as_str()).set("stand_port", "8100");
    context
}

/// Write a built-in form out as a template repository - into a new
/// directory, or over an earlier export of the same form.
fn export(form: Form, root: &Path, repo: Option<&str>) -> Result<(), Box<dyn Error>> {
    if let Some(earlier) = Exported::read(root)? {
        if earlier.form != form {
            return Err(format!(
                "`{}` is a template of the {} form; writing the {form} form over it would make it another template - export into a new directory",
                root.display(),
                earlier.form
            )
            .into());
        }
        follow::require_clean(root)?;
        let repo = repo
            .map(str::to_string)
            .or_else(|| crate::repo::origin_repo(root))
            .unwrap_or_else(|| new::repo(&dir_name(root, form), None));
        let over = Over::plan(form, root, &repo)?;
        over.write(root)?;
        println!(
            "Wrote the {form} form of lyrn {} over the template in `{}`: {}.",
            env!("CARGO_PKG_VERSION"),
            root.display(),
            over.counts()
        );
        return Ok(());
    }
    generate::check_destination(root)?;
    let repo = new::repo(&dir_name(root, form), repo);
    let plan = export_plan(form, &repo)?;
    generate::write(&plan, root)?;
    println!("Wrote the {form} form as a template in `{}` ({} files).", root.display(), plan.files.len());
    println!("\nNext:");
    println!("  lyrn template check {}", root.display().to_string().replace('\\', "/"));
    println!("  push it to {repo}, tag it once its CI is green, and use it:");
    println!("  lyrn new my-app --template {repo}@v1.0.0");
    Ok(())
}

fn dir_name(root: &Path, form: Form) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("template-{form}"))
}

/// What an earlier export says it is: its form, the lyrn that wrote it, and
/// the standard it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exported {
    pub form: Form,
    pub lyrn: Option<String>,
    pub standard: String,
}

impl Exported {
    /// The template at `root`, if one is there: `None` for a directory that
    /// does not exist or holds no `template.toml`.
    pub fn read(root: &Path) -> Result<Option<Self>, Box<dyn Error>> {
        let path = root.join("template.toml");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("cannot read `{}`: {e}", path.display()).into()),
        };
        // Read as data, not as a template: a manifest an older lyrn wrote may
        // carry what this one no longer accepts, and it is about to be
        // written over all the same.
        let table: toml::Table = toml::from_str(&text).map_err(|e| format!("`{}` does not parse: {e}", path.display()))?;
        let field = |key: &str| table.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let form = field("form")
            .ok_or_else(|| format!("`{}` names no form, so it is not an export of one", path.display()))?
            .parse()
            .map_err(|e| format!("`{}`: {e}", path.display()))?;
        Ok(Some(Self {
            form,
            lyrn: field("lyrn"),
            standard: field("standard").unwrap_or_default(),
        }))
    }
}

/// An export written over an earlier one: every file the form writes now,
/// and the files under `template/` it no longer writes, to go.
///
/// The licence is left as it is: it names whoever holds the copyright, which
/// is the repository's to say once it has one.
pub(crate) struct Over {
    pub plan: Plan,
    pub changed: Vec<PathBuf>,
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

impl Over {
    pub fn plan(form: Form, root: &Path, repo: &str) -> Result<Self, Box<dyn Error>> {
        let mut plan = export_plan(form, repo)?;
        if root.join("LICENSE").exists() {
            plan.files.retain(|f| f.path != Path::new("LICENSE"));
        }
        let (mut changed, mut added) = (Vec::new(), Vec::new());
        for file in &plan.files {
            match std::fs::read(root.join(&file.path)) {
                Ok(bytes) if bytes == file.contents => {}
                Ok(_) => changed.push(file.path.clone()),
                Err(_) => added.push(file.path.clone()),
            }
        }
        let mut removed = Vec::new();
        let files = root.join(FILES_DIR);
        if files.is_dir() {
            for path in walk(&files)? {
                let relative = Path::new(FILES_DIR).join(path.strip_prefix(&files).unwrap_or(&path));
                if !plan.files.iter().any(|f| f.path == relative) {
                    removed.push(relative);
                }
            }
        }
        removed.sort();
        Ok(Self { plan, changed, added, removed })
    }

    pub fn counts(&self) -> String {
        format!("{} changed, {} added, {} removed", self.changed.len(), self.added.len(), self.removed.len())
    }

    pub fn write(&self, root: &Path) -> Result<(), Box<dyn Error>> {
        for path in &self.removed {
            std::fs::remove_file(root.join(path))?;
        }
        // Directories under template/ left empty go with their files: git
        // keeps none of them, and a checkout elsewhere would not have them.
        for path in &self.removed {
            let mut dir = path.parent();
            while let Some(d) = dir.filter(|d| d.starts_with(FILES_DIR) && *d != Path::new(FILES_DIR)) {
                if std::fs::remove_dir(root.join(d)).is_err() {
                    break;
                }
                dir = d.parent();
            }
        }
        for file in &self.plan.files {
            let target = root.join(&file.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &file.contents)?;
            generate::set_executable(&target, file.executable)?;
        }
        Ok(())
    }
}

/// Every file under `dir`, by its full path.
fn walk(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Everything `export` writes, before any of it is written.
fn export_plan(form: Form, repo: &str) -> Result<Plan, Box<dyn Error>> {
    let sources = templates::sources_for(form);
    let mut files = Vec::new();
    let mut executable = Vec::new();
    let mut addons: Vec<AddonManifest> = form
        .addons()
        .iter()
        .map(|a| AddonManifest {
            name: a.as_str().to_string(),
            summary: a.summary().to_string(),
            files: Vec::new(),
        })
        .collect();

    for source in sources {
        let path = generate::inside_the_project(&format!("{FILES_DIR}/{}", source.path))
            .ok_or_else(|| format!("the {form} form writes `{}`, which cannot be a file of a template", source.path))?;
        if source.executable {
            executable.push(source.path.to_string());
        }
        if let Some(addon) = source.addon {
            let entry = addons
                .iter_mut()
                .find(|a| a.name == addon.as_str())
                .ok_or_else(|| format!("`{}` belongs to `{addon}`, which the {form} form does not offer", source.path))?;
            entry.files.push(source.path.to_string());
        }
        let verbatim = verbatim_of(form).contains(&source.path.to_string());
        let contents = match source.contents {
            // Sections of add-ons this form does not offer - the spa's, in
            // the files the desktop form borrows - are decided here, as the
            // form always decides them: off.
            generate::Contents::Text(text) if !verbatim => {
                generate::decide_sections(text, &[], |name| !form.addons().iter().any(|a| a.as_str() == name)).into_bytes()
            }
            generate::Contents::Text(text) => text.as_bytes().to_vec(),
            generate::Contents::Binary(bytes) => bytes.to_vec(),
        };
        files.push(PlannedFile {
            path,
            contents,
            executable: source.executable,
        });
    }

    let mut manifest = String::new();
    writeln!(
        manifest,
        "# The {form} form of lyrn {}, written out by `lyrn template export {form}`.",
        env!("CARGO_PKG_VERSION")
    )?;
    writeln!(manifest, "# The project's files are under {FILES_DIR}/; the variables they may ask for")?;
    writeln!(manifest, "# are listed at https://lyrn.lacodda.com/reference/template/.")?;
    writeln!(manifest, "form = \"{form}\"")?;
    writeln!(manifest, "lyrn = \"{}\"", env!("CARGO_PKG_VERSION"))?;
    if !executable.is_empty() {
        writeln!(manifest, "executable = {}", toml::Value::from(executable.clone()))?;
    }
    writeln!(manifest)?;
    manifest.push_str(templates::manifest_for(form).trim_end());
    manifest.push('\n');
    for addon in &addons {
        writeln!(manifest)?;
        let table = toml::to_string(addon)?;
        writeln!(manifest, "[[addons]]\n{}", table.trim_end())?;
    }

    let add = |files: &mut Vec<PlannedFile>, path: &str, text: String| {
        files.push(PlannedFile {
            path: PathBuf::from(path),
            contents: text.into_bytes(),
            executable: false,
        });
    };
    add(&mut files, "template.toml", manifest);
    add(&mut files, "README.md", readme(form, repo, &addons));
    add(&mut files, ".github/workflows/ci.yml", workflow(form, &addons));
    // The files under template/ are bytes to copy, not text for git to
    // normalise; the project's own .gitattributes, under template/, still
    // applies to them as it would in the project.
    add(&mut files, ".gitattributes", "* -text\n".to_string());
    add(&mut files, "LICENSE", licence());

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Plan { files })
}

/// The files a built-in form copies as they are.
fn verbatim_of(form: Form) -> Vec<String> {
    template::builtin_manifest(form).verbatim
}

fn readme(form: Form, repo: &str, addons: &[AddonManifest]) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "# {}\n", repo.split('/').nth(1).unwrap_or(repo));
    let _ = writeln!(text, "> {}\n", form.summary());
    let _ = writeln!(
        text,
        "A [lyrn](https://lyrn.lacodda.com) template of the `{form}` form. lyrn generates from it only at a tag whose CI passed, and the CI here generates every combination of its add-ons and runs the gate each generated project ships with.\n"
    );
    let _ = writeln!(text, "```console\n$ lyrn new my-app --template {repo}@v1.0.0\n```\n");
    if !addons.is_empty() {
        let _ = writeln!(text, "## Add-ons\n");
        for addon in addons {
            let _ = writeln!(text, "- `--with {}` - {}", addon.name, addon.summary);
        }
        let _ = writeln!(text);
    }
    let _ = writeln!(text, "## Changing it\n");
    let _ = writeln!(
        text,
        "The project's files live under `{FILES_DIR}/`; `template.toml` says which belong to which add-on. `lyrn template check .` holds the template to what lyrn will accept. See [writing a template](https://lyrn.lacodda.com/reference/template/)."
    );
    text
}

fn licence() -> String {
    let year = chrono::Local::now().format("%Y");
    let author = new::git_identity("user.name").unwrap_or_else(|| "The author".to_string());
    format!(
        "MIT License

Copyright (c) {year} {author}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"
    )
}

/// What a form's generated project needs to be built, and the gate it ships
/// with - the same steps lyrn's own CI runs against the built-in form.
struct Gate {
    rust: bool,
    pnpm: bool,
    /// Linux packages the project links against.
    apt: &'static [&'static str],
    postgres: bool,
    /// Arguments `lyrn new` needs beyond the name, for a form that asks.
    generate: &'static str,
    /// Where in the project the steps run, and what they run.
    steps: &'static [(&'static str, &'static str)],
}

const RUST_GATE: &[(&str, &str)] = &[("", "cargo fmt --check"), ("", "cargo clippy --all-targets -- -D warnings"), ("", "cargo test")];
const PNPM_GATE: &[(&str, &str)] = &[("", "pnpm install --no-frozen-lockfile"), ("", "pnpm lint"), ("", "pnpm build")];
const WEBVIEW: &[&str] = &["libwebkit2gtk-4.1-dev", "libappindicator3-dev", "librsvg2-dev", "patchelf"];

fn gate(form: Form) -> Gate {
    let none = Gate {
        rust: false,
        pnpm: false,
        apt: &[],
        postgres: false,
        generate: "",
        steps: &[],
    };
    match form {
        Form::Spa | Form::Mono => Gate {
            pnpm: true,
            steps: PNPM_GATE,
            ..none
        },
        Form::Cli => Gate {
            rust: true,
            apt: &["libdbus-1-dev", "pkg-config"],
            steps: RUST_GATE,
            ..none
        },
        Form::Workspace => Gate {
            rust: true,
            apt: &["libdbus-1-dev", "pkg-config"],
            steps: &[
                ("", "cargo fmt --check"),
                ("", "cargo clippy --workspace --all-targets -- -D warnings"),
                ("", "cargo test --workspace"),
            ],
            ..none
        },
        Form::Service => Gate {
            rust: true,
            postgres: true,
            steps: RUST_GATE,
            ..none
        },
        Form::Plugin => Gate {
            rust: true,
            generate: "--host kilna",
            steps: RUST_GATE,
            ..none
        },
        Form::Desktop => Gate {
            rust: true,
            pnpm: true,
            apt: WEBVIEW,
            steps: &[
                ("", "pnpm install --no-frozen-lockfile"),
                ("", "pnpm lint"),
                ("", "pnpm build"),
                ("src-tauri", "cargo fmt --check"),
                ("src-tauri", "cargo clippy --all-targets -- -D warnings"),
                ("src-tauri", "cargo test"),
            ],
            ..none
        },
        Form::TauriPlugin => Gate {
            rust: true,
            pnpm: true,
            apt: &["libwebkit2gtk-4.1-dev", "libgtk-3-dev", "libsoup-3.0-dev", "libjavascriptcoregtk-4.1-dev"],
            steps: &[
                ("", "cargo fmt --check"),
                ("", "cargo clippy --all-targets -- -D warnings"),
                ("", "cargo test"),
                ("", "pnpm install --no-frozen-lockfile"),
                ("", "pnpm lint"),
                ("", "pnpm test"),
                ("", "pnpm build"),
            ],
            ..none
        },
        Form::Docs => Gate {
            pnpm: true,
            steps: &[("docs", "pnpm install --no-frozen-lockfile"), ("docs", "pnpm build")],
            ..none
        },
    }
}

/// Every subset of the add-ons, the empty one first: the failures live in the
/// seams, and a seam exists only once a combination is generated.
fn combinations(addons: &[AddonManifest]) -> Vec<String> {
    (0..1u32 << addons.len())
        .map(|mask| {
            addons
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, a)| a.name.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

/// The template repository's CI: check the template with the lyrn it names,
/// then generate each combination of its add-ons and run the project's gate.
fn workflow(form: Form, addons: &[AddonManifest]) -> String {
    let gate = gate(form);
    let version = env!("CARGO_PKG_VERSION");
    let mut y = String::new();
    let _ = write!(
        y,
        "\
name: CI

# On the commits of main, not on tags: lyrn asks about the CI of the commit a
# tag points at, so a run on the tag would build the same files again, and
# until it finished the tag would read as still running.
on:
  push:
    branches: [main]
  pull_request:

jobs:
  # The checks lyrn itself runs before it uses a template from outside.
  check:
    name: lyrn template check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 22
      - name: Install lyrn
        run: npm install --global lyrn@{version}
      - name: Check the template
        run: lyrn template check .

  # A template that does not build is not a template: every combination of
  # its add-ons is generated, and the gate the project ships with is run.
  generate:
    name: generate (${{{{ matrix.addons || 'none' }}}})
    needs: check
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        addons: [{}]
",
        combinations(addons).iter().map(|c| format!("'{c}'")).collect::<Vec<_>>().join(", ")
    );
    if gate.postgres {
        y.push_str(
            "    services:
      postgres:
        image: postgres:18-alpine
        env:
          POSTGRES_USER: demo_app
          POSTGRES_PASSWORD: demo_app
          POSTGRES_DB: demo_app
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 3s
          --health-retries 10
        ports:
          - 5432:5432
",
        );
    }
    y.push_str("    steps:\n      - uses: actions/checkout@v7\n");
    if !gate.apt.is_empty() {
        let _ = write!(
            y,
            "      - name: Install the Linux libraries it links against
        run: |
          sudo apt-get update
          sudo apt-get install -y {}
",
            gate.apt.join(" ")
        );
    }
    if gate.rust {
        y.push_str(
            "      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
",
        );
    }
    if gate.pnpm {
        y.push_str("      - uses: pnpm/action-setup@v6\n        with:\n          version: 10\n");
    }
    let _ = write!(
        y,
        "      - uses: actions/setup-node@v7
        with:
          node-version: 22
      - name: Install lyrn
        run: npm install --global lyrn@{version}
"
    );
    let with = "\n          if [ -n \"${{ matrix.addons }}\" ]; then args+=(--with \"${{ matrix.addons }}\"); fi";
    if form == Form::Docs {
        // A documentation site is added to a repository, so one is made
        // first: the cli form's, which lyrn carries built in.
        let _ = write!(
            y,
            "      - name: Generate a repository, then add the site to it
        shell: bash
        run: |
          set -eu
          lyrn new demo-app --form cli --yes --no-hooks --repo owner/demo-app --path \"$RUNNER_TEMP/demo-app\"
          cd \"$RUNNER_TEMP/demo-app\"
          args=(new demo-app --template \"$GITHUB_WORKSPACE\" --yes --no-hooks --repo owner/demo-app){with}
          lyrn \"${{args[@]}}\"
"
        );
    } else {
        let extra = if gate.generate.is_empty() {
            String::new()
        } else {
            format!(" {}", gate.generate)
        };
        let _ = write!(
            y,
            "      - name: Generate a project
        shell: bash
        run: |
          args=(new demo-app --template \"$GITHUB_WORKSPACE\"{extra} --yes --no-hooks --repo owner/demo-app --path \"$RUNNER_TEMP/demo-app\"){with}
          lyrn \"${{args[@]}}\"
"
        );
    }
    if gate.rust {
        // After the project exists, never before: the cache restores
        // `target/` into the project's directory, and a destination that is
        // already there is one lyrn refuses. The first run has no cache and
        // passes; every run after it would fail.
        let crate_dir = gate.steps.iter().find(|(_, c)| c.starts_with("cargo")).map_or("", |(d, _)| d);
        let workspace = if crate_dir.is_empty() { String::new() } else { format!("/{crate_dir}") };
        let _ = write!(
            y,
            "      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: ${{{{ runner.temp }}}}/demo-app{workspace}
"
        );
    }
    for (dir, command) in gate.steps {
        let workdir = if dir.is_empty() {
            "${{ runner.temp }}/demo-app".to_string()
        } else {
            format!("${{{{ runner.temp }}}}/demo-app/{dir}")
        };
        let _ = write!(y, "      - name: {command}\n        working-directory: {workdir}\n");
        if gate.postgres {
            y.push_str("        env:\n          DATABASE_URL: postgres://demo_app:demo_app@localhost:5432/demo_app\n");
        }
        let _ = writeln!(y, "        run: {command}");
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Template;

    fn native(template: &Template) -> &template::Native {
        match &template.kind {
            Kind::Lyrn(native) => native,
            Kind::CargoGenerate(_) => unreachable!(),
        }
    }

    /// The whole point of an export: generating from the written-out
    /// template gives, byte for byte, what the built-in form gives - with no
    /// add-ons and with all of them.
    #[test]
    fn an_exported_form_generates_what_the_builtin_one_does() {
        for form in Form::ALL {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("t");
            generate::write(&export_plan(*form, "owner/template-x").unwrap(), &root).unwrap();

            let exported = template::load_dir(&root, Origin::Path(root.clone())).unwrap_or_else(|e| panic!("{form}: {e}"));
            let builtin = template::builtin(*form);
            let (exported, builtin) = (native(&exported), native(&builtin));
            assert_eq!(exported.form, *form);
            assert_eq!(exported.addons, builtin.addons, "{form}: the add-ons differ");

            let context = sample_context(*form);
            let all: Vec<String> = builtin.addons.iter().map(|a| a.name.clone()).collect();
            for addons in [&[][..], &all[..]] {
                let from_export = generate::plan_with(&exported.files, &exported.manifest, &context, addons).unwrap();
                let from_builtin = generate::plan_with(&builtin.files, &builtin.manifest, &context, addons).unwrap();
                assert_eq!(
                    from_export.files, from_builtin.files,
                    "{form} with {addons:?}: the export generates something else"
                );
            }
            assert_eq!(exported.manifest.hooks.len(), builtin.manifest.hooks.len(), "{form}: hooks were lost");
        }
    }

    #[test]
    fn the_workflow_generates_every_combination() {
        let Kind::Lyrn(spa) = template::builtin(Form::Spa).kind else { unreachable!() };
        let addons: Vec<AddonManifest> = spa
            .addons
            .iter()
            .map(|a| AddonManifest {
                name: a.name.clone(),
                summary: a.summary.clone(),
                files: vec![],
            })
            .collect();
        assert_eq!(combinations(&addons).len(), 16);
        let yaml = workflow(Form::Spa, &addons);
        assert!(yaml.contains("'router,tanstack-query,auth,pwa'"), "{yaml}");
        assert!(yaml.contains("lyrn template check ."), "{yaml}");
        assert!(yaml.contains(&format!("lyrn@{}", env!("CARGO_PKG_VERSION"))), "{yaml}");
    }

    /// A workflow expression survives the Rust formatting it is built with:
    /// `${{ matrix.addons }}` and not `${ matrix.addons }`.
    /// The Rust cache restores `target/` into the project's directory, so it
    /// has to come after the project is generated - the other way round, the
    /// second run of the CI finds the destination taken.
    #[test]
    fn the_rust_cache_comes_after_the_project() {
        for form in Form::ALL.iter().filter(|f| gate(**f).rust) {
            let yaml = workflow(*form, &[]);
            let generated = yaml.find("Generate a").unwrap_or_else(|| panic!("{form}: nothing is generated"));
            let cached = yaml.find("rust-cache").unwrap_or_else(|| panic!("{form}: nothing is cached"));
            assert!(generated < cached, "{form}: the cache is restored before the project exists");
        }
        assert!(workflow(Form::Desktop, &[]).contains("workspaces: ${{ runner.temp }}/demo-app/src-tauri"));
        assert!(workflow(Form::Cli, &[]).contains("workspaces: ${{ runner.temp }}/demo-app\n"));
    }

    #[test]
    fn the_workflow_keeps_its_expressions_whole() {
        let yaml = workflow(Form::Cli, &[]);
        assert!(yaml.contains("${{ matrix.addons || 'none' }}"), "{yaml}");
        assert!(yaml.contains("${{ runner.temp }}/demo-app"), "{yaml}");
        assert!(!yaml.contains("${ "), "{yaml}");
    }
}
