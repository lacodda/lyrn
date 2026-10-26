//! `lyrn msrv`: the oldest Rust the code builds on, measured and written as
//! `rust-version`.
//!
//! `rust-version` is a promise to everyone running `cargo install`, and a
//! number typed in is a guess: `lyrn new` writes the toolchain in the room,
//! which is the newest, not the oldest that works. The standard's `msrv` job
//! keeps a number from rotting; this finds the true one. The floor is what
//! the code cannot go under anyway - its edition, and the highest
//! `rust-version` its dependencies declare - and each toolchain is asked
//! the question the `msrv` job asks: `cargo check --all-targets`.

use std::error::Error;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::Instant;

use regex::Regex;

use crate::cli::PathArgs;
use crate::commands::doctor::{Repo, newline_of};

/// A Rust release as `rust-version` states it: `1.88`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u32,
    minor: u32,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let mut parts = text.trim().trim_matches('"').split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
        Some(Version { major, minor })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// The oldest Rust an edition compiles on.
fn edition_floor(edition: &str) -> Version {
    let minor = match edition {
        "2024" => 85,
        "2021" => 56,
        "2018" => 31,
        _ => 0,
    };
    Version { major: 1, minor }
}

/// Why the search starts where it does: the highest of the edition's floor
/// and every package's declared `rust-version`, with the package that set it.
#[derive(Debug, PartialEq, Eq)]
pub struct Floor {
    pub version: Version,
    pub because: String,
}

/// The floor from `cargo metadata`: the workspace's own packages give their
/// edition, every package in the build gives the `rust-version` it declares.
/// Platform-specific dependencies count: the promise is for every platform.
pub fn floor(metadata: &serde_json::Value) -> Option<Floor> {
    let members: Vec<&str> = metadata.get("workspace_members")?.as_array()?.iter().filter_map(|m| m.as_str()).collect();
    let mut best: Option<Floor> = None;
    let mut consider = |version: Version, because: String| {
        if best.as_ref().is_none_or(|b| version > b.version) {
            best = Some(Floor { version, because });
        }
    };
    for package in metadata.get("packages")?.as_array()? {
        let name = package.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        let id = package.get("id").and_then(|i| i.as_str()).unwrap_or_default();
        if members.contains(&id) {
            if let Some(edition) = package.get("edition").and_then(|e| e.as_str()) {
                consider(edition_floor(edition), format!("edition {edition}"));
            }
        } else if let Some(version) = package.get("rust_version").and_then(|v| v.as_str()).and_then(Version::parse) {
            let declared = package.get("version").and_then(|v| v.as_str()).unwrap_or_default();
            consider(version, format!("{name} {declared} asks for it"));
        }
    }
    best
}

/// The oldest of `candidates`, oldest first, that builds. The first is the
/// floor and is tried on its own - it is the answer more often than not;
/// past it, a toolchain that builds the code is taken to mean every newer one
/// does too, and the rest is halved. `None` when the newest does not build.
pub fn lowest<E>(candidates: &[Version], mut builds: impl FnMut(Version) -> Result<bool, E>) -> Result<Option<Version>, E> {
    let Some((&first, rest)) = candidates.split_first() else { return Ok(None) };
    if builds(first)? {
        return Ok(Some(first));
    }
    let (mut low, mut high) = (0, rest.len());
    let mut found = None;
    while low < high {
        let middle = (low + high) / 2;
        if builds(rest[middle])? {
            found = Some(rest[middle]);
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(found)
}

/// A manifest with `rust-version` set in `section` (`package` or
/// `workspace.package`): the value replaced where it is, else a line added
/// under `edition`, else after the section's last key - the section is
/// written when the manifest has none. The file's line endings are kept.
pub fn with_rust_version(text: &str, section: &str, version: Version) -> String {
    let newline = newline_of(text);
    let line = format!("rust-version = \"{version}\"");
    let header = format!("[{section}]");
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == header) else {
        let body = text.trim_end_matches(['\r', '\n']);
        return format!("{body}{newline}{newline}{header}{newline}{line}{newline}");
    };
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with('['))
        .map_or(lines.len(), |p| start + 1 + p);
    let existing = Regex::new(r#"^\s*rust-version\s*="#).expect("a valid pattern");
    let edition = Regex::new(r#"^\s*edition\s*="#).expect("a valid pattern");
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    if let Some(at) = (start + 1..end).find(|&i| existing.is_match(lines[i])) {
        out[at] = line;
    } else {
        let after = (start + 1..end)
            .find(|&i| edition.is_match(lines[i]))
            .or_else(|| (start + 1..end).rev().find(|&i| !lines[i].trim().is_empty()))
            .unwrap_or(start);
        out.insert(after + 1, line);
    }
    let ending = if text.ends_with('\n') { newline } else { "" };
    format!("{}{ending}", out.join(newline))
}

/// Where a manifest's `rust-version` lives: its own `[package]`, or the
/// workspace's when it inherits it or is a virtual workspace.
fn section_of(table: &toml::Table) -> &'static str {
    match table.get("package") {
        Some(package) => match package.get("rust-version") {
            Some(toml::Value::Table(t)) if t.get("workspace").and_then(|w| w.as_bool()) == Some(true) => "workspace.package",
            _ => "package",
        },
        None => "workspace.package",
    }
}

pub fn run(args: PathArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn msrv` measures a Rust project", path.display()).into());
    }
    let repo = Repo::read(&path);
    let roots = build_roots(&repo.root)?;
    if roots.is_empty() {
        return Err(format!("`{}` has no Cargo.toml - `lyrn msrv` measures a Rust project", repo.root.display()).into());
    }
    let stable = rustc_version("stable").ok_or("cannot ask rustup for the stable toolchain - `lyrn msrv` builds with rustup's toolchains")?;
    println!("lyrn msrv: {}, stable {stable}", repo.root.display());

    let installed_before = toolchains();
    let mut installed_here: Vec<Version> = Vec::new();
    let mut result = Ok(());
    for root in &roots {
        if let Err(e) = measure(&repo, root, stable, &installed_before, &mut installed_here) {
            result = Err(e);
            break;
        }
    }

    // Leave the machine as it was found: a toolchain is a few hundred
    // megabytes, and `lyrn ci msrv` installs the one it needs when it runs.
    let removed: Vec<String> = installed_here
        .iter()
        .filter(|v| {
            Process::new("rustup")
                .args(["toolchain", "uninstall", &v.to_string()])
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .map(|v| v.to_string())
        .collect();
    if !removed.is_empty() {
        println!("\nremoved the toolchains installed to measure: {}", removed.join(", "));
    }
    result
}

/// The directories whose `cargo check` covers the repository's Rust: the top,
/// and a desktop app's shell unless the top's workspace already holds it.
fn build_roots(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut roots = Vec::new();
    let top = root.join("Cargo.toml");
    let tauri = root.join("src-tauri").join("Cargo.toml");
    if top.is_file() {
        roots.push(PathBuf::new());
    }
    if tauri.is_file() {
        let covered = top.is_file()
            && metadata(root, None)
                .ok()
                .and_then(|m| m.get("workspace_members").and_then(|w| w.as_array()).cloned())
                .is_some_and(|members| members.iter().filter_map(|m| m.as_str()).any(|m| m.contains("src-tauri")));
        if !covered {
            roots.push(PathBuf::from("src-tauri"));
        }
    }
    Ok(roots)
}

/// `cargo metadata` of the manifest in `dir`, on `toolchain` or the default.
fn metadata(dir: &Path, toolchain: Option<Version>) -> Result<serde_json::Value, String> {
    let mut command = Process::new("cargo");
    command.args(["metadata", "--format-version", "1", "--locked"]).current_dir(dir);
    if let Some(version) = toolchain {
        command.env("RUSTUP_TOOLCHAIN", version.to_string());
    }
    let out = command.output().map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!("`cargo metadata` failed in {}: {}", dir.display(), last_error(&out.stderr)));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("`cargo metadata` said something unreadable: {e}"))
}

/// The version of `rustc` on a toolchain, as `rust-version` would state it.
fn rustc_version(toolchain: &str) -> Option<Version> {
    let out = Process::new("rustc").arg("--version").env("RUSTUP_TOOLCHAIN", toolchain).output().ok()?;
    // `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
    out.status
        .success()
        .then(|| Version::parse(String::from_utf8_lossy(&out.stdout).split_whitespace().nth(1)?))
        .flatten()
}

/// The numbered toolchains rustup has: `1.88-x86_64-pc-windows-msvc` is 1.88.
fn toolchains() -> Vec<Version> {
    let Ok(out) = Process::new("rustup").args(["toolchain", "list"]).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split('-').next())
        .filter(|v| v.matches('.').count() == 1)
        .filter_map(Version::parse)
        .collect()
}

fn measure(
    repo: &Repo,
    root: &Path,
    stable: Version,
    installed_before: &[Version],
    installed_here: &mut Vec<Version>,
) -> Result<Option<Version>, Box<dyn Error>> {
    let repo_root = repo.root.as_path();
    let dir = repo_root.join(root);
    let manifest_path = if root.as_os_str().is_empty() {
        PathBuf::from("Cargo.toml")
    } else {
        root.join("Cargo.toml")
    };
    let manifest_name = manifest_path.display().to_string().replace('\\', "/");
    let meta = metadata(&dir, None)?;
    let floor = floor(&meta).ok_or_else(|| format!("{manifest_name}: `cargo metadata` names no packages"))?;
    let text = std::fs::read_to_string(dir.join("Cargo.toml"))?;
    let table: toml::Table = toml::from_str(&text).map_err(|e| format!("{manifest_name} does not parse: {e}"))?;
    let section = section_of(&table);
    // A member inheriting the workspace's number is written at the top.
    let written_to = if section == "workspace.package" && table.get("workspace").is_none() {
        meta.get("workspace_root")
            .and_then(|r| r.as_str())
            .map(|r| PathBuf::from(r).join("Cargo.toml"))
            .unwrap_or_else(|| dir.join("Cargo.toml"))
    } else {
        dir.join("Cargo.toml")
    };
    let declared = crate::commands::doctor::msrv::declared_version(repo, &manifest_name)
        .ok()
        .flatten()
        .and_then(|v| Version::parse(&v));

    println!("\n{manifest_name}");
    println!("  declared   {}", declared.map_or("nothing".to_string(), |v| v.to_string()));
    println!("  floor      {} - {}", floor.version, floor.because);
    if floor.version > stable {
        return Err(format!(
            "{manifest_name}: the floor {} is above stable {stable} - update the toolchain first (`rustup update stable`)",
            floor.version
        )
        .into());
    }
    let candidates: Vec<Version> = (floor.version.minor..=stable.minor).map(|minor| Version { major: 1, minor }).collect();

    let target = tempfile::Builder::new().prefix("lyrn-msrv-").tempdir()?;
    let mut failure: Option<(Version, String)> = None;
    let found = lowest(&candidates, |version| -> Result<bool, Box<dyn Error>> {
        // The newest is the stable toolchain the machine already has.
        let toolchain = if version == stable { "stable".to_string() } else { version.to_string() };
        if version != stable && !installed_before.contains(&version) && !installed_here.contains(&version) {
            print!("  install    {version} ... ");
            std::io::stdout().flush().ok();
            let out = Process::new("rustup")
                .args(["toolchain", "install", &version.to_string(), "--profile", "minimal", "--no-self-update"])
                .output()?;
            if !out.status.success() {
                return Err(format!("rustup could not install {version}: {}", last_error(&out.stderr)).into());
            }
            installed_here.push(version);
            println!("done");
        }
        print!("  check      {version} ... ");
        std::io::stdout().flush().ok();
        let started = Instant::now();
        // The `msrv` job's question. `--ignore-rust-version`, because the
        // number being measured would otherwise answer it: cargo refuses a
        // package whose rust-version is above the toolchain before compiling
        // a line. The dependencies' numbers are under the floor already.
        let out = Process::new("cargo")
            .args(["check", "--all-targets", "--locked", "--ignore-rust-version", "--quiet"])
            .current_dir(&dir)
            .env("RUSTUP_TOOLCHAIN", &toolchain)
            .env("CARGO_TARGET_DIR", target.path())
            .output()?;
        let took = started.elapsed().as_secs();
        if out.status.success() {
            println!("builds, {took}s");
        } else {
            let error = last_error(&out.stderr);
            println!("does not build, {took}s: {error}");
            failure = Some((version, error));
        }
        Ok(out.status.success())
    })?;

    let Some(found) = found else {
        let (version, error) = failure.unwrap_or((stable, String::new()));
        return Err(format!("{manifest_name} does not build on {version}, the newest there is: {error}").into());
    };
    let shown = written_to
        .strip_prefix(repo_root)
        .unwrap_or(&written_to)
        .display()
        .to_string()
        .replace('\\', "/");
    if declared == Some(found) {
        println!("  measured   {found}, as declared - {shown} is left as it is");
        return Ok(Some(found));
    }
    let current = std::fs::read_to_string(&written_to)?;
    std::fs::write(&written_to, with_rust_version(&current, section, found))?;
    let was = declared.map_or("none".to_string(), |v| v.to_string());
    println!("  measured   {found} - rust-version {was} → {found} in {shown} [{section}]");
    Ok(Some(found))
}

/// The line of a tool's error output worth showing: the first `error`, else
/// the last line.
fn last_error(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .find(|l| l.trim_start().starts_with("error"))
        .or_else(|| text.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("no output")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn a_version_reads_as_rust_version_writes_it() {
        assert_eq!(v("1.88"), Version { major: 1, minor: 88 });
        assert_eq!(v("1.88.1"), Version { major: 1, minor: 88 });
        assert_eq!(v("\"1.9\"").to_string(), "1.9");
        assert!(v("1.9") < v("1.85"));
        assert_eq!(Version::parse("stable"), None);
    }

    #[test]
    fn the_floor_is_the_edition_or_the_hungriest_dependency() {
        let metadata = serde_json::json!({
            "workspace_members": ["path+file:///demo#0.1.0"],
            "packages": [
                { "id": "path+file:///demo#0.1.0", "name": "demo", "version": "0.1.0", "edition": "2024", "rust_version": "1.99" },
                { "id": "registry+a#1.0.0", "name": "liquid", "version": "0.26.11", "edition": "2021", "rust_version": "1.88" },
                { "id": "registry+b#1.0.0", "name": "old", "version": "1.0.0", "edition": "2018", "rust_version": null },
            ]
        });
        assert_eq!(
            floor(&metadata),
            Some(Floor {
                version: v("1.88"),
                because: "liquid 0.26.11 asks for it".to_string()
            })
        );
        // The project's own declaration is what is measured, not a floor.
        let own_only = serde_json::json!({
            "workspace_members": ["path+file:///demo#0.1.0"],
            "packages": [{ "id": "path+file:///demo#0.1.0", "name": "demo", "version": "0.1.0", "edition": "2024", "rust_version": "1.99" }]
        });
        assert_eq!(
            floor(&own_only),
            Some(Floor {
                version: v("1.85"),
                because: "edition 2024".to_string()
            })
        );
    }

    /// The search asks the floor first, and past it halves: never a toolchain
    /// it does not need, and the oldest one that builds.
    #[test]
    fn the_oldest_toolchain_that_builds_is_found_with_few_builds() {
        let candidates: Vec<Version> = (85..=99).map(|minor| Version { major: 1, minor }).collect();
        for oldest in 85..=99 {
            let mut asked = Vec::new();
            let found = lowest(&candidates, |version| -> Result<bool, ()> {
                asked.push(version.minor);
                Ok(version.minor >= oldest)
            })
            .unwrap();
            assert_eq!(found, Some(Version { major: 1, minor: oldest }), "{asked:?}");
            assert!(asked.len() <= 5, "{oldest}: asked {asked:?}");
            if oldest == 85 {
                assert_eq!(asked, [85], "the floor that builds is the only build");
            }
        }
        let never = lowest(&candidates, |_| -> Result<bool, ()> { Ok(false) }).unwrap();
        assert_eq!(never, None);
    }

    #[test]
    fn the_number_is_written_where_the_manifest_keeps_it() {
        let package = "[package]\nname = \"demo\"\nedition = \"2024\"\nrust-version = \"1.99\"\n\n[dependencies]\nclap = \"4\"\n";
        assert_eq!(
            with_rust_version(package, "package", v("1.88")),
            "[package]\nname = \"demo\"\nedition = \"2024\"\nrust-version = \"1.88\"\n\n[dependencies]\nclap = \"4\"\n"
        );
        let none = "[package]\r\nname = \"demo\"\r\nedition = \"2024\"\r\nversion = \"0.1.0\"\r\n";
        assert_eq!(
            with_rust_version(none, "package", v("1.85")),
            "[package]\r\nname = \"demo\"\r\nedition = \"2024\"\r\nrust-version = \"1.85\"\r\nversion = \"0.1.0\"\r\n"
        );
        let workspace = "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nrust-version = \"1.90\"\n";
        assert!(with_rust_version(workspace, "workspace.package", v("1.88")).ends_with("rust-version = \"1.88\"\n"));
        let bare = "[workspace]\nmembers = [\"a\"]\n";
        assert_eq!(
            with_rust_version(bare, "workspace.package", v("1.88")),
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nrust-version = \"1.88\"\n"
        );
        // A key of the same name in another table is not the one.
        let other = "[package]\nname = \"demo\"\n\n[package.metadata.x]\nrust-version = \"1.0\"\n";
        assert_eq!(
            with_rust_version(other, "package", v("1.88")),
            "[package]\nname = \"demo\"\nrust-version = \"1.88\"\n\n[package.metadata.x]\nrust-version = \"1.0\"\n"
        );
    }

    #[test]
    fn the_number_lives_where_the_manifest_says() {
        let table = |text: &str| -> toml::Table { toml::from_str(text).unwrap() };
        assert_eq!(section_of(&table("[package]\nname = \"a\"\n")), "package");
        assert_eq!(
            section_of(&table("[package]\nname = \"a\"\nrust-version.workspace = true\n")),
            "workspace.package"
        );
        assert_eq!(section_of(&table("[workspace]\nmembers = []\n")), "workspace.package");
    }
}
