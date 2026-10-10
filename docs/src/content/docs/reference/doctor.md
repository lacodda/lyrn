---
title: lyrn doctor
description: Hold a repository to the line's standard, and say what is off, how to fix it and why the rule exists.
---

```console
$ lyrn doctor [path]
$ lyrn doctor --explain <rule>
```

Examines a repository - the one the current directory is in, or the one at
`path` - against the line's standard, and prints one line per rule. Each rule is
a lesson the line learnt the hard way: a release that could not run, a lockfile
nobody audited, an old alias in a public history. A finding says what is wrong
and how to put it right; `--explain` says why the rule exists at all.

```console
$ lyrn doctor
lyrn doctor: ~/word-count, a `cli` project on standard 2026.10

✓ standard   all 21 files of the `cli` standard
✓ readme     the longest block is 15 words, in 38 lines
✓ mark       a mark of its own
✗ actions
    ✗ actions/checkout@v4 in release.yml: the standard is on v7
      → move to actions/checkout@v7, as the templates lyrn writes are
    why: lyrn doctor --explain actions
✗ msrv
    ✗ the msrv job in ci.yml builds on 1.85, and the manifest says 1.99
      → read the number from the manifest instead of writing it twice
    why: lyrn doctor --explain msrv
! node
    ! docs/package.json names no packageManager: CI and every machine use whatever pnpm they have
      → add `"packageManager": "pnpm@10.34.5"`
    why: lyrn doctor --explain node
✗ audit
    ✗ docs/pnpm-lock.yaml: no workflow audits it
      → a job running `pnpm audit` with `working-directory: docs` - the docs form's docs-audit.yml is one
    why: lyrn doctor --explain audit
✓ ai-traces  no assistant files tracked or in the history, no stamps, .gitignore keeps them out
✗ authors
    ✗ `ada <ada@old-laptop.local>` wrote 1 commit and is neither you nor anyone .mailmap names
      → if it is you, map it in .mailmap: `Ada Lovelace <ada@example.com> ada <ada@old-laptop.local>`
        if it is somebody else, name them there: `ada <ada@old-laptop.local>`
    why: lyrn doctor --explain authors
✓ identity   commits as `Ada Lovelace <ada@example.com>`, the global identity; the global excludes file keeps assistant files out

4 failed, 1 to look at, 5 passed
```

## Failures and warnings

A failure (`✗`) can be put right now, in a commit, and makes `lyrn doctor` exit
with 1 - so it works as a gate. A warning (`!`) is real but asks for something
a gate cannot: a rewrite of history that is already public, a setting on the
machine rather than in the repository, or a matter of taste. A rule that does
not apply - `msrv` in a repository with no Rust - says why and counts as
neither.

The standard is read from the templates lyrn writes, not from numbers kept
beside them: the action majors, the pnpm and Node versions, the list of
assistant files. When a template moves, the doctor moves with it, and a project
lyrn has just generated passes its own doctor - the test suite holds every form
to that.

## The rules

### standard

Every file of the standard for the repository's form is there: licence,
changelog and `cliff.toml`, editor and git settings, the CI gate and the audit,
the security policy and code of conduct, issue and pull request templates, the
first ADRs, and `lyrn.toml`. The list is exactly the one
[`lyrn adopt`](/reference/adopt/) writes, with the same rule for a file kept
under another name, so running `lyrn adopt` satisfies it. The form comes from
`lyrn.toml`, or is read from the repository the way `adopt` reads it.

### readme

The README is a shopfront: what it is, what it looks like at work, how to
install it. Measured by blocks rather than by the file - a paragraph, a quote
or a list item over about 90 words is a page of the documentation site; over
100 fails, between 91 and 100 warns. Code and tables are not counted. It also
fails a section the shopfront does not have (Roadmap, Building, Development,
Layout, Principles, Design notes), a `# title` under a banner that already
carries the name, and a README past 300 lines; it warns when there is no
one-sentence promise as a `>` quote before the first section, and when Status
is more than one paragraph of two to four lines.

### mark

The project wears a mark of its own. `lyrn.toml` records `mark = "placeholder"`
while it still wears the line's lambda on graphite, and the doctor warns until
it says `chosen`.

### actions

No GitHub Action GitHub has retired or archived - `create-release`,
`upload-release-asset`, the `actions-rs` family, the artifact actions before
v4, the cache before v3, the Pages actions built on the retired artifacts - and
no workflow command it disabled or deprecated (`::set-output`, `::save-state`,
`::set-env`, `::add-path`). An action on an older major than the templates use
fails too: the line takes every major as it comes. A commit pin counts by the
version in its comment, `@<sha> # v4.2.2`. A workflow that does not parse
fails, since GitHub runs none of it.

### msrv

`rust-version` is declared in `Cargo.toml` (and `src-tauri/Cargo.toml` for a
desktop app), and a CI job named `msrv` builds on it - reading the number from
the manifest, as the job `lyrn new` writes does. A job that builds on another
number or on `stable` fails; one that repeats the right number by hand warns.

### node

pnpm and Node at the majors the templates use. A project with a
`pnpm-lock.yaml` pins `packageManager` to the line's pnpm major; a workflow's
`setup-node` runs the line's Node or later; `pnpm/action-setup` is not asked
for an older pnpm. No pin at all, a pin ahead of the line and `setup-node`
without a version warn.

### audit

Every tracked `Cargo.lock`, `pnpm-lock.yaml` and `package-lock.json` is read
by an audit job: `cargo deny check` or `cargo-deny-action` with advisories,
`cargo audit`, `pnpm audit` or `npm audit` run in its directory - through
`working-directory`, a job's `defaults`, `cd`, `--dir` or `--manifest-path`. A
documentation site, a frontend folder and a desktop app's Rust shell each bring
a lockfile of their own, and an audit of the top of the repository reads none
of them. Lockfiles under `fixtures/` and `testdata/` are left out.

### ai-traces

No coding assistant's files or stamps. `.gitignore` keeps out every file of the
standard's assistant block (`CLAUDE.md`, `AGENTS.md`, `.claude/`, `.cursor/`
and the rest), and none of them is tracked - a file the repository ships on
purpose, such as a briefing it hands to the projects that use it, is
un-ignored by name after the block: `!registry/files/AGENTS.md`. An assistant
file anywhere in the history, and a commit message carrying an assistant's
stamp - a `Co-authored-by` trailer naming one, a "Generated with" line, a
session link - warn: taking them out is a rewrite of history.

### authors

Every author in the history, and every person in a `Co-authored-by` trailer,
resolves through `.mailmap` to the global git identity of this machine or to
someone `.mailmap` names. An alias of yours is mapped to you; anyone else who
wrote a commit is named as themselves:

```text
Ada Lovelace <ada@example.com> ada <ada@old-laptop.local>
Grace Hopper <grace@example.com>
```

Pushed history is never rewritten for this - `.mailmap` is git's own file for
saying who is who, and GitHub and `git log` read it. On CI, where a runner has
no identity of its own, `.mailmap` is the whole list.

### identity

This machine commits as its global identity, and keeps assistant files out of
every repository. It warns when commits here are signed by an identity set
inside the repository (and says which config file sets it), and when the
global excludes file - `core.excludesFile`, or git's default
`~/.config/git/ignore` - is missing or does not keep out every file of the
assistant block; it fails when there is no identity at all. `lyrn new` takes
the author for LICENSE and the manifests from the global identity for the same
reason. On CI this rule is not applicable.

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `[path]` | `.` | The repository to examine; a directory inside one examines all of it |
| `--explain <rule>` | — | Say why the rule exists and how to satisfy it, instead of examining |
