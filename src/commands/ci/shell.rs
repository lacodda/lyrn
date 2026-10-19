//! The shells a `run:` step runs in, invoked the way GitHub's runner invokes
//! them - which decides what a failing command does to the step.
//!
//! bash with `shell: bash` is `-eo pipefail`, without it on Linux just `-e`;
//! PowerShell stops on an error and then exits with the last native command's
//! code, so a script whose first `cargo` fails and whose last succeeds is a
//! green step on a Windows runner, and has to be one here.

use std::path::{Path, PathBuf};
use std::process::Command as Process;

use super::Os;

/// A shell, and the file extension its script is written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shell {
    /// `explicit` when the workflow named bash, which adds `pipefail`.
    Bash {
        explicit: bool,
    },
    Sh,
    Pwsh,
    Powershell,
    Cmd,
    Python,
    /// A command of the workflow's own, with `{0}` where the script goes.
    Custom(String),
}

impl Shell {
    /// The shell a step runs in: its own, else the job's or workflow's
    /// default, else the runner's - bash on Linux and macOS, PowerShell on
    /// Windows.
    pub fn of(named: Option<&str>, os: Os) -> Shell {
        match named.map(str::trim) {
            Some("bash") => Shell::Bash { explicit: true },
            Some("sh") => Shell::Sh,
            Some("pwsh") => Shell::Pwsh,
            Some("powershell") => Shell::Powershell,
            Some("cmd") => Shell::Cmd,
            Some("python") => Shell::Python,
            Some(other) if !other.is_empty() => Shell::Custom(other.to_string()),
            _ if os == Os::Windows => Shell::Pwsh,
            _ => Shell::Bash { explicit: false },
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Shell::Bash { .. } | Shell::Sh | Shell::Custom(_) => "sh",
            Shell::Pwsh | Shell::Powershell => "ps1",
            Shell::Cmd => "cmd",
            Shell::Python => "py",
        }
    }

    /// The script as the runner writes it to disk.
    pub fn script(&self, body: &str) -> Vec<u8> {
        match self {
            Shell::Pwsh | Shell::Powershell => {
                // Windows PowerShell reads a file without a byte order mark
                // in the machine's code page; the mark makes it UTF-8 for
                // both PowerShells.
                let text = format!("$ErrorActionPreference = 'stop'\n{body}\nif ((Test-Path -LiteralPath variable:\\LASTEXITCODE)) {{ exit $LASTEXITCODE }}\n");
                let mut bytes = vec![0xEF, 0xBB, 0xBF];
                bytes.extend(text.into_bytes());
                bytes
            }
            Shell::Cmd => body.replace("\r\n", "\n").replace('\n', "\r\n").into_bytes(),
            _ => body.replace("\r\n", "\n").into_bytes(),
        }
    }

    /// The command that runs `script`; why it cannot, when the shell is not
    /// installed here.
    pub fn command(&self, script: &Path) -> Result<(Process, Option<String>), String> {
        let path = script.display().to_string();
        let mut note = None;
        let command = match self {
            Shell::Bash { explicit } => {
                let mut c = Process::new(bash().ok_or("needs bash, and no bash is installed here (Git for Windows brings one)")?);
                if *explicit {
                    c.args(["--noprofile", "--norc", "-eo", "pipefail"]);
                } else {
                    c.arg("-e");
                }
                c.arg(posix(&path));
                c
            }
            Shell::Sh => {
                let program = if cfg!(windows) {
                    bash().ok_or("needs sh, and none is installed here")?
                } else {
                    PathBuf::from("sh")
                };
                let mut c = Process::new(program);
                c.args(["-e", &posix(&path)]);
                c
            }
            Shell::Pwsh | Shell::Powershell => {
                let program = if *self == Shell::Pwsh && find("pwsh").is_some() {
                    "pwsh"
                } else if cfg!(windows) && find("powershell").is_some() {
                    if *self == Shell::Pwsh {
                        note = Some("PowerShell 7 (pwsh) is not installed here; Windows PowerShell ran it".to_string());
                    }
                    "powershell"
                } else {
                    return Err("needs PowerShell (pwsh), which is not installed here".to_string());
                };
                let mut c = Process::new(program);
                c.args(["-NoProfile", "-NonInteractive", "-command", &format!(". '{}'", path.replace('\'', "''"))]);
                c
            }
            Shell::Cmd => {
                if !cfg!(windows) {
                    return Err("needs cmd, which only Windows has".to_string());
                }
                let mut c = Process::new("cmd");
                raw(&mut c, &format!("/D /E:ON /V:OFF /S /C \"CALL \"{path}\"\""));
                c
            }
            Shell::Python => {
                let program = ["python3", "python"]
                    .into_iter()
                    .find(|p| find(p).is_some())
                    .ok_or("needs Python, which is not installed here")?;
                let mut c = Process::new(program);
                c.arg(&path);
                c
            }
            Shell::Custom(template) => {
                let mut words = template.split_whitespace().map(|w| w.replace("{0}", &posix(&path)));
                let program = words.next().ok_or("an empty `shell`")?;
                let program = if cfg!(windows) && (program == "bash" || program == "sh") {
                    bash().ok_or("needs bash, and none is installed here")?.display().to_string()
                } else {
                    program
                };
                let mut c = Process::new(program);
                c.args(words);
                if !template.contains("{0}") {
                    c.arg(posix(&path));
                }
                c
            }
        };
        Ok((command, note))
    }
}

#[cfg(windows)]
fn raw(command: &mut Process, arguments: &str) {
    use std::os::windows::process::CommandExt;
    command.raw_arg(arguments);
}

#[cfg(not(windows))]
fn raw(command: &mut Process, arguments: &str) {
    command.arg(arguments);
}

/// A path as bash on Windows reads it: forward slashes, drive kept.
fn posix(path: &str) -> String {
    if cfg!(windows) { path.replace('\\', "/") } else { path.to_string() }
}

/// The bash a runner would use: on Windows, Git's - never the one in
/// System32, which starts a Linux distribution instead.
pub fn bash() -> Option<PathBuf> {
    if !cfg!(windows) {
        return find("bash");
    }
    let exec = Process::new("git").arg("--exec-path").output().ok()?;
    let exec = PathBuf::from(String::from_utf8_lossy(&exec.stdout).trim());
    // <git>/mingw64/libexec/git-core
    let git_root = exec.ancestors().nth(3)?;
    ["bin/bash.exe", "usr/bin/bash.exe"].iter().map(|p| git_root.join(p)).find(|p| p.is_file())
}

/// A program on PATH, as the system would find it.
pub fn find(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string())
            .split(';')
            .map(str::to_lowercase)
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(&path).find_map(|dir| {
        extensions
            .iter()
            .map(|ext| dir.join(format!("{name}{ext}")))
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runner_s_default_is_bash_but_on_windows() {
        assert_eq!(Shell::of(None, Os::Linux), Shell::Bash { explicit: false });
        assert_eq!(Shell::of(None, Os::Windows), Shell::Pwsh);
        assert_eq!(Shell::of(Some("bash"), Os::Windows), Shell::Bash { explicit: true });
        assert_eq!(Shell::of(Some("bash -l {0}"), Os::Linux), Shell::Custom("bash -l {0}".into()));
    }

    /// PowerShell's exit is the last native command's, as on a runner.
    #[test]
    fn a_powershell_script_is_wrapped_the_runner_s_way() {
        let script = String::from_utf8(Shell::Pwsh.script("cargo test")[3..].to_vec()).unwrap();
        assert!(script.starts_with("$ErrorActionPreference = 'stop'\ncargo test\n"), "{script}");
        assert!(script.ends_with("{ exit $LASTEXITCODE }\n"), "{script}");
    }
}
