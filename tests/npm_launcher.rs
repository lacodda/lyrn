//! The npm launcher the cli form writes, run the way a terminal runs it.
//!
//! Ctrl+C reaches every process in the terminal's foreground group: the node
//! launcher as well as the binary it started. A launcher that dies of it at
//! once hands the prompt back while the binary is still finishing, and the
//! binary's exit code is lost - sefy measured 0xC000013A where its own code
//! should have been. The launcher has to wait, and leave the decision to the
//! binary.
//!
//! Unix only: there a test can send the keystroke's signal to a process group,
//! exactly as the terminal does. The launcher's Windows half listens for the
//! same keystroke under the names Windows gives it.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod common;

/// A stand-in for the release binary: it says when it is running, and on
/// SIGINT tidies up and exits with a code of its own.
const FAKE_BINARY: &str = "#!/bin/sh\ntrap 'sleep 0.3; exit 7' INT\n: > \"$LAUNCHER_STARTED\"\nwhile :; do sleep 0.05; done\n";

fn wait_for(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < limit, "gave up waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn ctrl_c_leaves_the_exit_code_to_the_binary() {
    if Command::new("node").arg("--version").output().is_err() {
        panic!("node is not installed; the launcher cannot be tested without it");
    }

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-tool");
    common::lyrn()
        .args([
            "new",
            "demo-tool",
            "--form",
            "cli",
            "--yes",
            "--no-hooks",
            "--repo",
            "owner/demo-tool",
            "--path",
        ])
        .arg(&root)
        .assert()
        .success();

    // Where `download.js` looks for the binary, so the launcher runs it rather
    // than downloading one.
    let binary = root.join("npm").join("demo-tool");
    std::fs::write(&binary, FAKE_BINARY).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let started = dir.path().join("started");

    let mut launcher = spawn_launcher(&root, &started);
    wait_for("the binary to start", Duration::from_secs(20), || started.exists());

    // The keystroke, as the terminal delivers it: to the whole group.
    let group = format!("-{}", launcher.id());
    let sent = Command::new("kill").args(["-s", "INT", "--", &group]).status().unwrap();
    assert!(sent.success(), "could not signal the process group");

    let mut status = None;
    wait_for("the launcher to exit", Duration::from_secs(20), || {
        status = launcher.try_wait().unwrap();
        status.is_some()
    });
    let status = status.unwrap();
    assert_eq!(
        status.code(),
        Some(7),
        "the launcher did not end with the binary's code: {status:?} (signal {:?})",
        status.signal()
    );
}

/// `node npm/run.js` in a process group of its own, as a terminal starts a
/// foreground job.
///
/// A freshly written script can be refused with ETXTBSY when another test's
/// child was forked while the file was still open for writing; that, and only
/// that, is retried.
fn spawn_launcher(root: &Path, started: &Path) -> std::process::Child {
    for _ in 0..5 {
        let mut launcher = Command::new("node")
            .arg(root.join("npm").join("run.js"))
            .env("LAUNCHER_STARTED", started)
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .unwrap();
        wait_for("the launcher to start or fail", Duration::from_secs(20), || {
            started.exists() || launcher.try_wait().unwrap().is_some()
        });
        if started.exists() {
            return launcher;
        }
        let mut said = String::new();
        std::io::Read::read_to_string(launcher.stderr.as_mut().unwrap(), &mut said).unwrap();
        assert!(said.contains("ETXTBSY"), "the launcher failed before the binary started:\n{said}");
    }
    panic!("the binary stayed busy (ETXTBSY) through five attempts");
}
