//! The installers, held to what the line learned about them the hard way.
//!
//! They only ever run on somebody else's machine, after a release, so a
//! mistake in them arrives as a confused user rather than a failed build. Each
//! test here is a mistake one of the line's tools shipped once.

use std::path::{Path, PathBuf};

/// The repository's root: the nearest directory above this crate holding the
/// installers. A crate in a workspace sits two levels below it.
fn root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .find(|dir| dir.join("tools").join("install.sh").is_file())
        .unwrap_or_else(|| panic!("no tools/install.sh in or above {}", crate_dir.display()))
        .to_path_buf()
}

fn read(installer: &str) -> String {
    let path = root().join("tools").join(installer);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Git Bash, MSYS2 and Cygwin all run a `sh` script on Windows. Without a case
/// of its own for them the script answers "no prebuilt binary for
/// MINGW64_NT-...", which reads as "there is no Windows build" - when there is
/// one, installed by the other script.
///
/// Run rather than read: a case that exists but sits below the catch-all
/// would satisfy a search of the text and never match. `uname` is replaced by
/// a shell function, so nothing executable is written for the test to run.
#[cfg(unix)]
#[test]
fn the_unix_installer_sends_windows_shells_to_powershell() {
    let installer = root().join("tools").join("install.sh");
    for system in ["MINGW64_NT-10.0-26200", "MSYS_NT-10.0-26200", "CYGWIN_NT-10.0-26200"] {
        let script = ["uname() { case \"$1\" in -s) echo ", system, " ;; *) echo x86_64 ;; esac; }; . \"$0\""].concat();
        let output =
            std::process::Command::new("sh").args(["-c", &script]).arg(&installer).output().expect("cannot run sh");
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "on {system}, install.sh went on as if it could install");
        assert!(said.contains("install.ps1"), "on {system}, install.sh does not send the user to install.ps1:\n{said}");
        assert!(!said.contains("No prebuilt binary"), "on {system}, install.sh says there is no build:\n{said}");
    }
}

/// The fallback an installer suggests has to work: `cargo install` with a
/// name that is not the crate's fails, and a suggestion that fails is worse
/// than none. kasl suggested `kasl` while publishing `kasl-cli`.
#[test]
fn the_installers_suggest_the_crate_that_is_published() {
    let name = env!("CARGO_PKG_NAME");
    for installer in ["install.sh", "install.ps1"] {
        for (number, line) in read(installer).lines().enumerate() {
            let Some(at) = line.find("cargo install ") else { continue };
            let named = line[at + "cargo install ".len()..]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_');
            assert_eq!(
                named,
                name,
                "{installer} line {} suggests `cargo install {named}`, but the crate is `{name}`",
                number + 1
            );
        }
    }
}

/// The Windows installer adds itself to the user PATH, which is almost always
/// an expandable string: entries like `%JAVA_HOME%\bin` are stored unexpanded.
/// `[Environment]::SetEnvironmentVariable` writes it back as a plain string,
/// and every such entry becomes literal text - one installer of the line did.
/// Comments are left out of the search: the one explaining this names the call.
#[test]
fn the_windows_installer_keeps_the_path_expandable() {
    let code: Vec<String> =
        read("install.ps1").lines().map(str::trim).filter(|line| !line.starts_with('#')).map(String::from).collect();
    let code = code.join("\n");
    assert!(
        !code.contains("SetEnvironmentVariable"),
        "install.ps1 calls SetEnvironmentVariable, which turns the PATH into a plain string"
    );
    assert!(
        code.contains("DoNotExpandEnvironmentNames") && code.contains("ExpandString"),
        "install.ps1 has to read the PATH unexpanded and write it back as an expandable string"
    );
}
