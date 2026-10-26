//! `lyrn msrv`, run as a user runs it, on toolchains the machine already has:
//! a dependency that asks for stable puts the floor at stable, so the measure
//! needs nothing installed and nothing from the network.

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

/// The stable toolchain's version, as `rust-version` states it.
fn stable() -> String {
    let out = Command::new("rustc").arg("--version").env("RUSTUP_TOOLCHAIN", "stable").output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let version = text.split_whitespace().nth(1).unwrap();
    version.rsplit_once('.').map(|(major_minor, _)| major_minor.to_string()).unwrap()
}

fn write(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

#[test]
fn the_oldest_toolchain_that_builds_is_written_as_rust_version() {
    let stable = stable();
    // The project promises a release that does not exist yet: cargo would
    // refuse every toolchain before compiling, unless told to measure.
    let (major, minor) = stable.split_once('.').unwrap();
    let promised = format!("{major}.{}", minor.parse::<u32>().unwrap() + 1);
    let dir = tempfile::tempdir().unwrap();
    // The dependency lives beside the crate, outside its workspace: a package
    // of the build, not a member, so its rust-version is a floor.
    write(
        dir.path(),
        "needs-stable/Cargo.toml",
        &format!("[package]\nname = \"needs-stable\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{stable}\"\n"),
    );
    write(dir.path(), "needs-stable/src/lib.rs", "pub fn answer() -> u32 {\n    42\n}\n");
    let root = dir.path().join("demo");
    write(
        &root,
        "Cargo.toml",
        &format!(
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{promised}\"\n\n[dependencies]\nneeds-stable = {{ path = \"../needs-stable\" }}\n"
        ),
    );
    write(&root, "src/main.rs", "fn main() {\n    println!(\"{}\", needs_stable::answer());\n}\n");
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(lock.status.success(), "{}", String::from_utf8_lossy(&lock.stderr));

    let out = common::lyrn().arg("msrv").arg(&root).output().unwrap();
    let report = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{report}");
    for line in [
        format!("  declared   {promised}"),
        format!("  floor      {stable} - needs-stable 0.1.0 asks for it"),
        format!("  check      {stable} ... builds"),
        format!("  measured   {stable} - rust-version {promised} → {stable} in Cargo.toml [package]"),
    ] {
        assert!(report.contains(&line), "no `{line}` in:\n{report}");
    }
    assert!(!report.contains("install"), "stable is the toolchain already there:\n{report}");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains(&format!("edition = \"2024\"\nrust-version = \"{stable}\"\n")), "{manifest}");
    // The builds went elsewhere: the project's target/ is not the measure's.
    assert!(!root.join("target").exists(), "the measure built into the project's target/");

    // Measured again, the number is already right.
    let out = common::lyrn().arg("msrv").arg(&root).output().unwrap();
    let report = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        report.contains(&format!("  measured   {stable}, as declared - Cargo.toml is left as it is")),
        "{report}"
    );
}

#[test]
fn a_directory_without_rust_is_told_so() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# demo\n").unwrap();
    let out = common::lyrn().arg("msrv").arg(dir.path()).output().unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("has no Cargo.toml"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
