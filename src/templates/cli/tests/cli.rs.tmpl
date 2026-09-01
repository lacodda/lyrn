//! Tests that run the built binary, the way a user does.
//!
//! Unit tests check that a function is right; these check that the command is
//! there, spelled the way the docs say, and exits with the code a script would
//! branch on.

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("{{ name }}").unwrap()
}

#[test]
fn greets_by_default() {
    bin().arg("hello").assert().success().stdout(predicate::str::contains("Hello, world!"));
}

#[test]
fn greets_by_name() {
    bin().args(["hello", "friend"]).assert().success().stdout(predicate::str::contains("Hello, friend!"));
}

#[test]
fn reports_its_version() {
    bin().arg("--version").assert().success().stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn an_unknown_command_fails_rather_than_doing_nothing() {
    bin().arg("nonesuch").assert().failure();
}
