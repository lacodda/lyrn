---
title: lyrn adopt
description: Bring an existing repository up to the line's standard.
---

```console
$ lyrn adopt [path] [options]
```

Adds the files of the line's standard that an existing repository does not
have yet, and never touches the ones it does. What it adds is the part of the
standard every repository carries whatever its code does:

| File | What it is |
| --- | --- |
| `README.md` | The shopfront |
| `LICENSE` | MIT, in the author's name |
| `CHANGELOG.md`, `cliff.toml` | A changelog generated from Conventional Commits |
| `.editorconfig`, `.gitattributes`, `.gitignore` | How files are written and kept |
| `.github/workflows/ci.yml` | The gate the form runs: lint, types, tests, build |
| `docs/adr/0001-record-architecture-decisions.md`, `docs/adr/README.md` | The decision log, started |
| `rustfmt.toml` | The line's formatting, for the Rust forms |
| `lyrn.toml` | What the project is, for `doctor` and `upgrade` to read |

The release contour - release and publish workflows, the npm wrapper, the
installers - is part of the standard too, but its files only work together and
against one package layout, so they are not added one by one beside a build
lyrn did not make.

## What it reads

The repository describes itself, and `adopt` listens before it assumes:

- **The form** comes from `lyrn.toml` if there is one, else from what the
  repository holds: `src-tauri/` is a `desktop`, a Cargo `[workspace]` a
  `workspace`, a crate on `tauri-plugin` a `tauri-plugin`, one on `axum` a
  `service`, a crate named `kilna-plugin-*` a `plugin` for kilna, any other
  binary crate a `cli`, a pnpm workspace with `packages/` a `mono`, and a
  package on Vite a `spa`. Anything else is refused with the forms on offer,
  rather than adopted as a guess - a library crate among them.
- **The name** and **the description** come from `Cargo.toml` or
  `package.json`; the name falls back to the directory's.
- **The repository** is the GitHub remote `origin` points at.
- **The accent** is the product's own when the project is a product of the
  line - `hilvan` is crimson without being told.

Anything given on the command line wins.

## What is already there

A file of the standard counts as present under its own name or under one it is
commonly kept under: `LICENSE.md` or `COPYING` for the licence, `CHANGES.md`
for the changelog, `ci.yaml` for the gate. An ADR numbered 0001 about something
else keeps its number, since two first decisions would make "ADR 1" mean two
things. The tree shown before writing says which files are kept and why:

```console
$ lyrn adopt --dry-run
Would add 8 files to `.` as a `cli` project; 3 of its standard are already there:

  ├── .github/
  │   └── workflows/
  │       └── ci.yml
  ├── docs/
  │   └── adr/
  │       ├── 0001-record-architecture-decisions.md
  │       └── README.md
  ├── .editorconfig
  ├── .gitattributes
  ├── .gitignore  (already there)
  ├── CHANGELOG.md
  ├── LICENSE  (`LICENSE.md` is there)
  ├── README.md  (already there)
  ├── cliff.toml
  ├── lyrn.toml
  └── rustfmt.toml
```

Running it again has nothing to add. `adopt` stages and commits nothing: the
repository is yours, and `git status` shows what arrived.

A documentation site is something a repository has rather than something it
is, so it is not adopted; `lyrn adopt` says when there is none, and
[`lyrn new <name> --form docs`](/reference/forms/#docs) adds one.

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `[path]` | `.` | The repository to adopt |
| `--form <form>` | read from the repository | The form it has |
| `--host <host>` | read from a plugin's name | The application a plugin extends |
| `--name <name>` | from its manifest | The project name |
| `--accent <colour>` | its own, else asked | A product of the line, or a `#rrggbb` value |
| `--description <text>` | from its manifest | One line describing what the project is |
| `--author <name>` | `git config user.name` | Recorded in LICENSE |
| `--repo <owner/name>` | the `origin` remote | The GitHub repository it lives in |
| `-y`, `--yes` | — | Accept the defaults instead of asking |
| `--dry-run` | — | Show what would be added and kept, and write nothing |
