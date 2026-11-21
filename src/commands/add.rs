//! `lyrn add`: items of dowel's registry, copied into the project from the
//! dowel-ui it pins.
//!
//! `shadcn add` with dowel's address reads the registry on the site, which is
//! the newest dowel there is - not necessarily the one the project installed.
//! A copy of 1.2's Button in a project on 1.0 is the drift the project's own
//! `check-registry` refuses, so the first thing an add would do is fail the
//! lint. lyrn takes the copies from the registry inside the installed
//! `node_modules/dowel-ui` instead: the same file `check-registry` holds the
//! copies to, so what lands matches what is checked.
//!
//! A primitive under `components/ui` is dowel's text and is held to it, so
//! one that differs is taken again. A block under `components/blocks` is the
//! product's from the moment it lands: one that is there already is never
//! written over.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::AddArgs;
use crate::templates::dowel::{Catalogue, Item, Kind, Places};

pub fn run(args: AddArgs) -> Result<(), Box<dyn Error>> {
    let start = match &args.path {
        Some(path) => path.clone(),
        None => std::env::current_dir()?,
    };
    let frontend = frontend_of(&start).ok_or_else(|| {
        format!(
            "no components.json in {} or above it - `lyrn add` copies dowel into a project drawn with dowel: the spa and desktop forms, or a service's frontend/",
            start.display()
        )
    })?;
    let places = Places::read(&std::fs::read_to_string(frontend.join("components.json"))?)?;
    let catalogue = Catalogue::installed(&frontend)?;
    let added = add(&frontend, &places, &catalogue, &args.items)?;
    // Where the copies went, as the person standing here would type it.
    let here = std::env::current_dir().ok();
    let shown = here
        .as_deref()
        .and_then(|here| frontend.strip_prefix(here).ok())
        .map(|rest| {
            if rest.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                rest.to_path_buf()
            }
        })
        .unwrap_or_else(|| frontend.clone());
    print!("{}", added.report(&shown));

    let missing = added.missing_packages(&frontend)?;
    if !missing.is_empty() {
        install(&frontend, &missing);
    }
    Ok(())
}

/// The directory holding the project's `components.json`: the one given, its
/// `frontend/` as a service keeps it, or the nearest above.
fn frontend_of(start: &Path) -> Option<PathBuf> {
    let has = |dir: &Path| dir.join("components.json").is_file();
    if has(start) {
        return Some(start.to_path_buf());
    }
    if has(&start.join("frontend")) {
        return Some(start.join("frontend"));
    }
    start.ancestors().skip(1).find(|dir| has(dir)).map(Path::to_path_buf)
}

/// What happened to one file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Added,
    /// Already the registry's text.
    Kept,
    /// A primitive that was not the registry's text, written again.
    TakenAgain,
    /// A block that is there and differs: the product's, left alone.
    Yours,
}

/// What an add wrote, and what the items need of npm.
#[derive(Debug)]
struct Added {
    version: String,
    files: Vec<(PathBuf, Outcome)>,
    packages: Vec<String>,
}

/// Copies `names` and what they are made of into `frontend`. Every item is
/// checked before a file is written: a name the registry has no copy of
/// refuses the whole add, rather than leaving half of it behind.
fn add(frontend: &Path, places: &Places, catalogue: &Catalogue, names: &[String]) -> Result<Added, Box<dyn Error>> {
    for name in names {
        if let Some(short) = name.strip_prefix(crate::commands::new::blocks::PREFIX) {
            return Err(format!("`{name}` is how `lyrn new --with` names a block; in the registry it is `{short}-screen`").into());
        }
    }
    let items = catalogue.closure(names)?;
    let mut writes: Vec<(&Item, PathBuf, &str)> = Vec::new();
    for item in &items {
        match &item.kind {
            Kind::Ui | Kind::Block => {
                let place = places.of(item)?.ok_or_else(|| format!("`{}` has no file", item.name))?;
                let text = item.file.as_ref().map(|(_, text)| text.as_str()).unwrap_or_default();
                writes.push((item, place, text));
            }
            Kind::Bundle => {}
            Kind::Other(kind) => {
                return Err(format!(
                    "`{}` is a {kind} item, which comes with dowel-ui itself rather than as a copy - the theme is `@import 'dowel-ui/theme.css'`",
                    item.name
                )
                .into());
            }
        }
    }

    let mut files = Vec::new();
    for (item, place, text) in writes {
        let path = frontend.join(&place);
        let outcome = match std::fs::read_to_string(&path) {
            Ok(ours) if normal(&ours) == normal(text) => Outcome::Kept,
            Ok(_) if item.kind == Kind::Block => Outcome::Yours,
            Ok(_) => Outcome::TakenAgain,
            Err(_) => Outcome::Added,
        };
        if matches!(outcome, Outcome::Added | Outcome::TakenAgain) {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, text)?;
        }
        files.push((place, outcome));
    }
    let mut packages: Vec<String> = items.iter().flat_map(|i| i.packages.iter().cloned()).collect();
    packages.sort();
    packages.dedup();
    Ok(Added {
        version: catalogue.version.clone(),
        files,
        packages,
    })
}

/// Line endings are the checkout's business, as `check-registry` has it.
fn normal(text: &str) -> String {
    text.replace("\r\n", "\n")
}

impl Added {
    fn report(&self, frontend: &Path) -> String {
        let mut out = format!("From dowel-ui {}, in {}:\n", self.version, frontend.display());
        for (path, outcome) in &self.files {
            let (word, note) = match outcome {
                Outcome::Added => ("added", ""),
                Outcome::Kept => ("kept", ""),
                Outcome::TakenAgain => ("taken", " - again: it was not this dowel-ui's text"),
                Outcome::Yours => ("kept", " - a block is the product's once it is there, and this one has changed"),
            };
            out.push_str(&format!("  {word:<6} {}{note}\n", path.display().to_string().replace('\\', "/")));
        }
        out
    }

    /// The packages the items import that `package.json` does not list.
    fn missing_packages(&self, frontend: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(frontend.join("package.json"))?)?;
        let listed = |name: &str| {
            ["dependencies", "devDependencies"]
                .iter()
                .filter_map(|key| manifest.get(*key).and_then(|d| d.as_object()))
                .any(|deps| deps.contains_key(name))
        };
        Ok(self.packages.iter().filter(|spec| !listed(package_name(spec))).cloned().collect())
    }
}

/// `@base-ui/react@^1.9` is `@base-ui/react`: the version is after the last
/// `@` that is not the first character.
fn package_name(spec: &str) -> &str {
    match spec[1..].rfind('@') {
        Some(at) => &spec[..at + 1],
        None => spec,
    }
}

/// `pnpm add` for what the copies import and the project does not list -
/// or the command to run, where pnpm cannot.
fn install(frontend: &Path, missing: &[String]) {
    let command = format!("pnpm add {}", missing.join(" "));
    println!("\nThe copies import what package.json does not list: {command}");
    // `cmd /c` on Windows, where pnpm is a `.cmd` a bare spawn does not find.
    let mut process = if cfg!(windows) {
        let mut p = Process::new("cmd");
        p.args(["/c", "pnpm", "add"]);
        p
    } else {
        let mut p = Process::new("pnpm");
        p.arg("add");
        p
    };
    match process.args(missing).current_dir(frontend).output() {
        Ok(out) if out.status.success() => println!("Added to package.json."),
        Ok(out) => println!(
            "pnpm could not add them: {} - run `{command}` in {}",
            String::from_utf8_lossy(&out.stderr).trim(),
            frontend.display()
        ),
        Err(_) => println!("pnpm is not installed here - run `{command}` in {} once it is", frontend.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Catalogue {
        let json = r#"{"items": [
            {"name": "input", "type": "registry:ui", "dependencies": ["dowel-ui@^1.0.0"],
             "files": [{"path": "registry/ui/input.tsx", "target": "@ui/input.tsx", "content": "export const Input = 1\n"}]},
            {"name": "password-field", "type": "registry:ui", "registryDependencies": ["https://lacodda.github.io/dowel/r/input.json"],
             "dependencies": ["dowel-ui@^1.0.0", "@base-ui/react"],
             "files": [{"path": "registry/ui/password-field.tsx", "target": "@ui/password-field.tsx", "content": "export const PasswordField = 1\n"}]},
            {"name": "login-screen", "type": "registry:block", "registryDependencies": ["https://lacodda.github.io/dowel/r/password-field.json"],
             "files": [{"path": "registry/blocks/login-screen.tsx", "target": "@components/blocks/login-screen.tsx", "content": "export const LoginScreen = 1\n"}]},
            {"name": "theme", "type": "registry:style", "files": [{"path": "theme.css", "target": "~/dowel/theme.css", "content": ""}]}
        ]}"#;
        Catalogue::parse(json, "1.0.0").unwrap()
    }

    fn places() -> Places {
        Places::read(r#"{"aliases": {"ui": "@/components/ui", "components": "@/components"}}"#).unwrap()
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_block_comes_with_its_parts_under_their_own_places() {
        let dir = tempfile::tempdir().unwrap();
        let added = add(dir.path(), &places(), &registry(), &names(&["login-screen"])).unwrap();
        let read = |path: &str| std::fs::read_to_string(dir.path().join(path)).unwrap();
        assert_eq!(read("src/components/ui/input.tsx"), "export const Input = 1\n");
        assert_eq!(read("src/components/ui/password-field.tsx"), "export const PasswordField = 1\n");
        assert_eq!(read("src/components/blocks/login-screen.tsx"), "export const LoginScreen = 1\n");
        assert_eq!(added.packages, ["@base-ui/react", "dowel-ui@^1.0.0"]);
        let report = added.report(Path::new("app"));
        assert!(report.starts_with("From dowel-ui 1.0.0, in app:\n"), "{report}");
        assert!(report.contains("  added  src/components/blocks/login-screen.tsx\n"), "{report}");
    }

    /// A primitive is dowel's and is held to it; a block is the product's.
    #[test]
    fn a_changed_primitive_is_taken_again_and_a_changed_block_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let write = |path: &str, text: &str| {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("src/components/ui/input.tsx", "export const Input = 1\r\n");
        write("src/components/ui/password-field.tsx", "patched\n");
        write("src/components/blocks/login-screen.tsx", "ours now\n");
        let added = add(dir.path(), &places(), &registry(), &names(&["login-screen"])).unwrap();
        let outcomes: Vec<&Outcome> = added.files.iter().map(|(_, o)| o).collect();
        assert_eq!(outcomes, [&Outcome::Kept, &Outcome::TakenAgain, &Outcome::Yours]);
        let read = |path: &str| std::fs::read_to_string(dir.path().join(path)).unwrap();
        assert_eq!(read("src/components/ui/password-field.tsx"), "export const PasswordField = 1\n");
        assert_eq!(read("src/components/blocks/login-screen.tsx"), "ours now\n");
    }

    #[test]
    fn what_comes_with_the_package_is_refused_before_anything_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let error = add(dir.path(), &places(), &registry(), &names(&["input", "theme"])).unwrap_err().to_string();
        assert!(error.contains("`theme` is a style item, which comes with dowel-ui itself"), "{error}");
        assert!(!dir.path().join("src").exists(), "a refused add wrote files");
        let unknown = add(dir.path(), &places(), &registry(), &names(&["dialog"])).unwrap_err().to_string();
        assert!(unknown.contains("dowel-ui 1.0.0 has no `dialog`"), "{unknown}");
        let spelt = add(dir.path(), &places(), &registry(), &names(&["block:login"])).unwrap_err().to_string();
        assert!(spelt.contains("it is `login-screen`"), "{spelt}");
    }

    #[test]
    fn a_package_is_named_without_its_version() {
        assert_eq!(package_name("dowel-ui@^1.0.0"), "dowel-ui");
        assert_eq!(package_name("@base-ui/react@^1.9.0"), "@base-ui/react");
        assert_eq!(package_name("@base-ui/react"), "@base-ui/react");
        assert_eq!(package_name("clsx"), "clsx");
    }

    #[test]
    fn the_frontend_is_where_components_json_is() {
        let dir = tempfile::tempdir().unwrap();
        let frontend = dir.path().join("frontend");
        std::fs::create_dir_all(frontend.join("src").join("pages")).unwrap();
        std::fs::write(frontend.join("components.json"), "{}").unwrap();
        assert_eq!(frontend_of(dir.path()), Some(frontend.clone()));
        assert_eq!(frontend_of(&frontend.join("src").join("pages")), Some(frontend.clone()));
        assert_eq!(frontend_of(&frontend), Some(frontend));
        let elsewhere = tempfile::tempdir().unwrap();
        assert_eq!(frontend_of(elsewhere.path()), None);
    }
}
