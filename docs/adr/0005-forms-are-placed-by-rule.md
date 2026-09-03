# ADR 0005: A derived form places its files by rule, not by list

- Status: accepted
- Date: 2026-09-03

## Context

Three forms are now built on top of another: `desktop` on `spa`, `service` on
`spa`, and `workspace` on `cli`. Each does the same thing - take the parent's
files, drop the ones it replaces, add its own - and each did it by naming
paths in a `const &[&str]`.

The lists work as long as nobody touches the parent. The `workspace` form is
where that stopped being true: it does not replace the parent's Rust sources,
it *moves* them, from `src/` into `crates/<name>/src/`. A file added to the
`cli` form later would not be on the workspace's list, so it would be written
to the repository root - next to a `crates/` directory, where cargo does not
compile it and nothing says so. Both halves exist, one is dead, and the only
symptom is an edit that has no effect.

## Decision

A derived form states relocation as a rule over paths, not as a list of them.
`workspace` moves anything matching `build.rs`, `src/**` or `tests/**`; a file
added to `cli` tomorrow lands in the crate by itself.

Replacement stays a list - `OVERRIDDEN` names files this form writes its own
version of - because that genuinely is a finite, deliberate set, and a stale
entry there is caught: a test fails when a form's override names a file the
parent no longer writes.

## Consequences

- The `workspace` form inherits new `cli` files correctly without being edited.
- A rule cannot be checked by reading it, so the gate does the checking:
  `a_workspace_leaves_no_rust_file_outside_its_crates` reads the members out of
  the generated manifest and fails on any `.rs` file outside them. It was
  written against the broken rule first and did fail - four dead files at the
  root, every other test green.
- Where a future form needs the opposite - a parent file that must *not* move -
  it belongs in `OVERRIDDEN` with its own version, which is a statement rather
  than an omission.
