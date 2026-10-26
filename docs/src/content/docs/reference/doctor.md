---
title: lyrn doctor
description: Hold a repository to the line's standard, and say what is off, how to fix it and why the rule exists.
---

```console
$ lyrn doctor [path...] [--fix] [--json]
$ lyrn doctor --all [--fix] [--json]
$ lyrn doctor --explain <rule>
```

Examines a repository - the one the current directory is in, or the one at
`path` - against the line's standard, and prints one line per rule. Each rule is
a lesson the line learnt the hard way: a release that could not run, a lockfile
nobody audited, an old alias in a public history. A finding says what is wrong
and how to put it right; `--explain` says why the rule exists at all. More than
one repository, or [every one you have](#every-repository), is a
[table](#many-repositories); `--fix` [writes the repairs](#repairs) the findings
carry; `--json` says it all [as one document](#json).

```console
$ lyrn doctor
lyrn doctor: ~/word-count, a `cli` project on standard 2026.11

✓ standard   all 22 files of the `cli` standard
✓ readme     the longest block is 15 words, in 38 lines
✓ changelog  4 releases apart from what is above
✓ mark       a mark of its own
✓ assets     three levels in assets/, 4 copies and icons in place
✓ language   64 text files in English, translations in their places
✗ private
    ✗ the home of `ada`, this computer's account, in docs/export-assets.mjs:8
      → a path from the repository's top, or `~` and `$HOME` where a home is meant; in an example, a made-up name
    why: lyrn doctor --explain private
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
      → add `"packageManager": "pnpm@10.34.7"`
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

5 failed, 1 to look at, 8 passed; `lyrn doctor --fix` puts 2 findings right
stack: edition 2024 · msrv 1.99 · node 22 · pnpm 10.33.0 (the standard is on 10.34.7) · actions 1 behind · updated 9 days ago, due
```

The last line is the repository's [stack](#the-stack) beside the standard's.

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
first ADRs, the shopfront workflow, and `lyrn.toml`. The list is exactly the
one [`lyrn adopt`](/reference/adopt/) writes, with the same rule for a file
kept under another name, so running `lyrn adopt` satisfies it. The form comes
from `lyrn.toml`, or is read from the repository the way `adopt` reads it.

A file still holding a place [`lyrn upgrade`](/reference/upgrade/) left for a
person - the `<<<<<<< yours` markers - fails, lyrn.toml included, which then
does not parse. A repository on an older
[version of the standard](/concepts/the-standard-by-version/) than lyrn writes
warns, with `lyrn upgrade` as the way forward.

### changelog

Every release in `CHANGELOG.md` starts after a blank line. git-cliff writes the
changelog one release at a time, and a body template that ends without a blank
line under `trim = true` glues each release to the last line of the one before;
the file then reads as one long list. A release heading - `## [1.2.0] - ...` -
right after a line that is not blank fails. `--fix` puts the blank lines back;
the cause is in `cliff.toml`, whose body should open with an empty line as the
standard's does.

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

### assets

Each level of the mark sits where its size calls for it: L from 64px up, M from
28 to 63, S - the filled tile - at 27 and below. The masters are
`assets/logo.svg`, `assets/logo-m.svg` and `assets/logo-s.svg`, and a mark
with only the first fails: one drawing at every size is the defect itself.
Copies are held to their masters byte for byte - a site's header
(`src/assets/logo.svg`) to L, every `favicon.svg` to S, the documentation
site's `apple-touch-icon.png` to the one in `assets/`. Each image inside an
`.ico` is judged by how much of it is the line's plate, `#1B2126`: M and L
stand on it, the S tile is filled with the product's colour, so a 48px image
with no plate in it is the S tile where the plated mark belongs. The images
are listed largest first - Tauri takes the first one as the window's icon -
and an `.ico` without 16, 24, 32, 48 and 256 warns. A touch icon and
`assets/logo-512.png` are drawn large and must be plated. Templates and test
material are left out. `--fix` copies the masters over stale copies, reorders
an `.ico`, and draws a wrong `.ico` or touch icon again from the masters with
the code of [`lyrn assets`](/reference/assets/). An exporter of the
repository's own, `export-assets.mjs`, warns: it is a second drawing of the
rule, and the line's own got it wrong in nine products of fifteen.

### language

What ships is written in English; another language lives where its
translations go - a directory or file named for the language (`docs/ru/`,
`locales/ru.json`, `README.ru.md`) or a directory of translations (`i18n/`,
`locales/`). Text in another script is not by itself a finding: what fails is
prose, four words or more in a row. In code only the comments are read, and
what a comment quotes is a citation; in Markdown, code and emphasis are shown
rather than said; JSON, CSV and the like are values. A word typed partly in a
Cyrillic layout - `command` whose `c` is Cyrillic - fails anywhere outside a
test: it looks right, and search and spelling miss it. Commit subjects in
another language warn, since only a rewrite takes them out.

### private

Nothing from this computer: the home of its account in any spelling
(`C:\Users\<you>`, `/c/Users/<you>`, `/home/<you>`), a path into the directory
the repository lives in that names something actually there - a script that
finds its assets by its author's absolute path - and the machine's own name. A
made-up example path names nothing on this computer and passes. On CI the rule
does not apply.

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
without a version warn; `--fix` adds the missing pin.

Every pnpm project also holds `node_modules` to its lockfile before a script
runs: `verifyDepsBeforeRun: install` in its `pnpm-workspace.yaml` (or
`verify-deps-before-run=install` in `.npmrc`). Without it a gate run after a
pull checks the packages installed last month, not the ones the lockfile - and
the release - names. `--fix` adds the setting, as the templates write it.

A `setup-node` given a major - `22`, `22.x` - also says `check-latest: true`.
Without it the job runs on whatever release of the major the runner has
cached, and the cache lags the floor tools raise in patch releases: Astro 7.4.2
asked for Node 22.23.4 while GitHub's runners had 22.23.3. `--fix` adds the
line under `node-version`.

### audit

Every tracked `Cargo.lock`, `pnpm-lock.yaml` and `package-lock.json` is read
by an audit job: `cargo deny check` or `cargo-deny-action` with advisories,
`cargo audit`, `pnpm audit` or `npm audit` run in its directory - through
`working-directory`, a job's `defaults`, `cd`, `--dir` or `--manifest-path`. A
documentation site, a frontend folder and a desktop app's Rust shell each bring
a lockfile of their own, and an audit of the top of the repository reads none
of them. Lockfiles in a test's material - under `tests/`, `fixtures/`,
`testdata/` - are left out. For a site under `docs/` that nothing audits,
`--fix` writes the docs form's `docs-audit.yml`.

### ai-traces

No coding assistant's files or stamps. `.gitignore` keeps out every file of the
standard's assistant block (`CLAUDE.md`, `AGENTS.md`, `.claude/`, `.cursor/`
and the rest), and none of them is tracked - a file the repository ships on
purpose, such as a briefing it hands to the projects that use it, is
un-ignored by name after the block: `!registry/files/AGENTS.md`. An assistant
file anywhere in the history, and a commit message carrying an assistant's
stamp - a `Co-authored-by` trailer naming one, a "Generated with" line, a
session link - warn: taking them out is a rewrite of history. `--fix` ends
`.gitignore` with the patterns of the block it lacks.

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

## Many repositories

Name more than one, and the report is a table: a mark per rule, a row per
repository, then the stack of each beside the standard's. `lyrn doctor <path>`
on one of them says what is off and how to put it right.

```console
$ lyrn doctor ../word-count ../tally
lyrn doctor: 2 repositories

            standard readme changelog mark assets language private actions msrv node audit ai-traces authors identity
word-count  ✓        ✓      ✓         ✓    ✓      ✓        ✗       ✗       ✗    !    ✗     ✓         ✗       ✓
tally       ✓        ✓      ✓         ✓    ✓      ✓        ✓       ✓       ✓    ✓    ✓     ✓         ✓       ✓

1 fails, 1 passes; `lyrn doctor --fix` puts 2 findings right
`lyrn doctor <path>` says what is off in one of them, how to put it right and why

stack       edition  msrv  node  pnpm       actions          updated
standard    2024           22    10.34.7                     weekly
word-count  2024     1.99  22    10.33.0*   1 behind*        9 days ago, due
tally       2024     1.88  22    10.34.7    on the standard  2 days ago
* behind the standard
```

### Every repository

`--all` examines every project in [rigger](https://github.com/lacodda/rigger)'s
record - the one list of which projects are yours and where each lives - rather
than every directory that happens to sit beside the others. A project the
record has retired or keeps out of the line is named as not examined; a path the record has
that is not there fails its row. Without rigger, name the repositories instead.

### The stack

What each repository builds on - Rust's edition, the `rust-version` it promises
(and a toolchain `rust-toolchain.toml` pins), the oldest Node its workflows set
up, the oldest pnpm a `packageManager` pins, the actions on a major the
standard has moved past - and when its dependencies were last brought up to
date. That is a `chore(deps)` commit, or the first commit of a project, which
starts on the stack of its day. The habit is to bring the stack up to date at
the start of a version, once a week at most, and the doctor checks it by the
fact: when the newest version started a week or more after the last update
and shipped without one, it says that version skipped it. The stack is not a
rule - a repository a pnpm release behind is not broken, and `node` and
`actions` fail on their own when it is - but `*` marks who has fallen behind.

## Repairs

Some findings carry their own repair: the contents a file should have. The rule
that finds the problem computes it, so `--fix` writes exactly what the report
promised, then examines the repository again with the written files counted
in. The report says how many findings `--fix` would put right. Only an edit
that cannot change what the project does qualifies, and none of them runs git:

- `.gitignore` gets the patterns of the assistant block it lacks, after which
  a file shipped on purpose is let back in again by its `!` line;
- a `package.json` beside a pnpm lockfile gets `packageManager`, written in
  the file's own indentation and line endings;
- a site's copy of the mark is copied from its master;
- an `.ico` gets its images in order, largest first, the images untouched;
- a site under `docs/` that nothing audits gets the docs form's
  `docs-audit.yml`.

A missing file of the standard is not a repair: [`lyrn adopt`](/reference/adopt/)
adds those, with the tree shown first, since some of them change what CI runs.
Review what `--fix` wrote with `git diff`, and commit it.

## JSON

`--json` prints one document, the same shape for one repository or many:

```json
{
  "lyrn": "2.12.0",
  "standard": { "edition": "2024", "node": 22, "pnpm": "10.34.7", "actions": { "actions/checkout": 7 } },
  "repositories": [
    {
      "name": "word-count",
      "path": "/home/ada/word-count",
      "result": "fail",
      "form": "cli",
      "standard": "2026.10",
      "rules": [
        { "rule": "standard", "result": "pass", "text": "all 21 files of the `cli` standard", "findings": [] },
        {
          "rule": "node",
          "result": "warn",
          "findings": [
            {
              "level": "warn",
              "what": "docs/package.json names no packageManager: ...",
              "fix": "add `\"packageManager\": \"pnpm@10.34.7\"`",
              "repair": { "path": "docs/package.json", "what": "pins pnpm@10.34.7" }
            }
          ]
        }
      ],
      "fixed": [],
      "stack": { "edition": "2024", "msrv": "1.99", "node": 22, "pnpm": "10.33.0", "behind": ["pnpm", "actions"] }
    }
  ],
  "skipped": []
}
```

A repository's `result` is `pass`, `warn`, `fail`, or `missing` when its
directory is not there; a rule's is `pass`, `skip`, `warn` or `fail`.

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `[path...]` | `.` | The repositories to examine; a directory inside one examines all of it. More than one is a table |
| `--all` | — | Examine every project in rigger's record |
| `--fix` | — | Write the repairs the findings carry, then examine again |
| `--json` | — | Print one JSON document instead of the report |
| `--explain <rule>` | — | Say why the rule exists and how to satisfy it, instead of examining |
