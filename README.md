<p align="center"><img src="https://raw.githubusercontent.com/lacodda/lyrn/main/assets/banner.svg" alt="lyrn - a new app of the line, one command" width="720"></p>

> Start a new web application on the lacodda line's stack with one command - one stack assembled to the end, and a repository ready to work in from the first commit.

<p align="center">
  <a href="https://crates.io/crates/lyrn"><img src="https://img.shields.io/crates/v/lyrn?style=flat-square" alt="crates.io"></a>
  <a href="https://www.npmjs.com/package/lyrn"><img src="https://img.shields.io/npm/v/lyrn?style=flat-square" alt="npm"></a>
  <a href="https://github.com/lacodda/lyrn/actions"><img src="https://img.shields.io/github/actions/workflow/status/lacodda/lyrn/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/lacodda/lyrn/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lacodda/lyrn?style=flat-square" alt="License"></a>
</p>

## What it is

`lyrn new` creates a repository that is ready to work in: Vite, React,
TypeScript, Tailwind and the [dowel](https://lacodda.github.io/dowel) design
system, together with the things a project usually grows only after the third
time somebody wishes it had them — a CI gate and an audit, a changelog, the
first decisions, a security policy and a code of conduct, an editor config, a
license.

It is not a scaffolder for any framework you like. It is **one stack, assembled
to the end**. The choice a generator usually hands back to you has already been
made, and made the same way for every product on the line.

## A day in the life

```console
$ lyrn new demo-app --accent kilna --yes
Created 44 files in `demo-app`.
Installing dependencies... done
Starting the repository... done
Staging the first commit... done
Creating the first commit... done

Next:
  cd demo-app
  pnpm dev
```

```console
$ cd demo-app && pnpm lint
> eslint . && tsc --noEmit && pnpm registry && pnpm test

registry: 1 copy matches dowel-ui 0.36.0
 Test Files  2 passed (2)
      Tests  3 passed (3)
```

That gate is the same one CI runs, so a green terminal means a green pull
request.

```console
$ lyrn doctor
lyrn doctor: ~/demo-app, a `spa` project on standard 2026.11

✓ standard   all 20 files of the `spa` standard
✓ readme     the longest block is 18 words, in 25 lines
✓ changelog  no release yet
✓ mark       a mark of its own
✓ assets     three levels in assets/, 1 copy or icon in place
✓ language   45 text files in English, translations in their places
✓ private    nothing from this computer in 45 text files
✓ actions    7 actions in 3 workflows, none retired or behind the standard
- msrv       not a Rust project
✓ node       pnpm 10 and Node 22+ across 1 project
✓ audit      1 lockfile audited
✓ ai-traces  no assistant files tracked or in the history, no stamps, .gitignore keeps them out
✓ authors    1 commit, all by Kirill Lakhtachev <lahtachev@gmail.com>
✓ identity   commits as `Kirill Lakhtachev <lahtachev@gmail.com>`, the global identity; the global excludes file keeps assistant files out

13 passed, 1 not applicable
stack: node 22 · pnpm 10.34.6 · actions on the standard · updated today
```

On an older repository the same command says what is off - a retired action,
a lockfile nobody audits, an old alias in the history - with the fix for each;
`--fix` writes the safe ones, `--all` holds every repository you have to the
standard in one table, and `lyrn doctor --explain <rule>` says why it exists.

Before a release, `lyrn check` holds the shopfronts to each other - one
version, one description, only commands that exist - and `lyrn ci` runs the CI
the push would start on a clean copy of the commit, so a red run is seen here
first. When the standard moves on, `lyrn upgrade` merges what changed into the
repository's files and commits it as one change.

## Install

```console
$ npm install -g lyrn          # or: cargo install lyrn
```

## Usage

```console
$ lyrn new <name> [options]      # a project in a directory of its own
$ lyrn init [path] [options]     # the same, in a directory that already exists
$ lyrn adopt [path]              # the standard's missing files, into an old repository
$ lyrn upgrade [path]            # the newest standard, merged into the files and committed
$ lyrn doctor [path...] [--fix]  # what is off against the standard, why, and the safe repairs
$ lyrn check [path...]           # the shopfronts held to each other: versions, descriptions, commands
$ lyrn ci [job...]               # the CI a push of this commit would start, run here first
$ lyrn stage                     # start a stage: what stands in its way, and a line in rigger's record
$ lyrn status                    # where the repository stands, on one screen
$ lyrn versions                  # where the version is written, and where it went
$ lyrn deps                      # how old the stack is, and which dependencies are behind
$ lyrn forms                     # the forms, their add-ons, and your own templates
$ lyrn template check [path]     # hold a template to what lyrn accepts
```

| Option | What it does |
| --- | --- |
| `--form <form>` | The shape of the project (default: `spa`) |
| `--template <source>` | A template instead: a path, a name in `~/.lyrn/templates`, or `owner/repo@tag` |
| `--host <host>` | The application a plugin extends; `--form plugin` only |
| `--accent <colour>` | A product of the line, or a `#rrggbb` value |
| `--description <text>` | One line describing what the project is |
| `--author <name>` | Recorded in LICENSE; defaults to the global `git config user.name` |
| `--path <path>` | Where to create it; `--form docs` adds to the current directory |
| `--repo <owner/name>` | The GitHub repository it will live in |
| `--with <addon,...>` | Optional pieces of the form, comma-separated |
| `-y`, `--yes` | Accept the defaults instead of asking |
| `--dry-run` | Show the tree that would be written, and write nothing |
| `--no-hooks` | Skip installing dependencies and starting the repository |

Without `--yes` and with a terminal attached, lyrn asks for what it is missing.
Without a terminal — in CI, in a script — it takes the defaults and never
blocks waiting for an answer.

## The accent

A generated project states one thing about its own appearance, and the theme
derives the rest — the hover shade, the soft fill, the focus ring, the tint the
greys carry, and what colour text has to be on an accent fill:

```css
:root {
  --accent-base: #d9569e;
}
```

`--accent` takes either a colour or the name of a product on the line, in which
case its mark's colour is used.

## Forms

| Form | What it produces |
| --- | --- |
| `spa` | Single-page app: Vite, React, TypeScript, Tailwind, dowel |
| `cli` | Command-line tool: Rust, clap, anyhow, dialoguer |
| `desktop` | Desktop app: Tauri 2 around the spa stack |
| `service` | HTTP service: axum, sqlx, Postgres |
| `workspace` | Cargo workspace: a library crate plus the CLI that uses it |
| `mono` | pnpm monorepo publishing a TypeScript package to npm |
| `plugin` | Plugin for a host of the line: an executable speaking JSON over stdio |
| `tauri-plugin` | Tauri 2 plugin: a Rust crate and the npm package that calls it |
| `docs` | Documentation site added to an existing repository: Starlight, llms.txt |

A form can carry optional pieces. The line's four CLIs agree on clap, anyhow
and dialoguer and disagree about everything else, so the rest is asked for:

```console
$ lyrn new my-tool --form cli --with keyring,self-update
```

`lyrn forms` lists them under each form: `keyring` and `self-update` for the
CLIs, `i18n` for the desktop, `router`, `tanstack-query`, `auth` and `pwa` for
the spa, a web UI and a `demo` for the service. Each is what the line's
products already do, and none is generated unless asked for — a project should
not carry code it never calls. What each one writes:
[Forms](https://lyrn.lacodda.com/reference/forms/).

The rest - a workspace publishing its core as a crate, a pnpm monorepo, a
plugin for a host of the line, both halves of a Tauri plugin, a documentation
site added to an existing repository - each carries a gate that catches what
its shape gets wrong: [Forms](https://lyrn.lacodda.com/reference/forms/).

## Templates from outside

`--template` generates from a template that is not built in: a directory of
yours, one under `~/.lyrn/templates` - which, named after a form, takes that
form's place - or a GitHub repository at a tag. A tag is used only once the
template's own CI has passed on that very commit, and `lyrn.toml` records which
commit it was. cargo-generate templates are read too.

```console
$ lyrn new my-app --template lacodda/template-spa@v1.0.0
```

`lyrn template export spa` writes a built-in form out as a template repository
to start from: [Templates](https://lyrn.lacodda.com/reference/template/).

More forms — egui — follow in 2.x.

## Status

v2.14.0, in daily use. All nine forms work and pass their own `lyrn doctor`,
each generated and put through its gate on Linux, macOS and Windows on every
push; `lyrn adopt` brings an older repository up to the standard, `lyrn upgrade`
carries one to its newest version, and `lyrn doctor --all` says what is still
off across all of them. What landed in each version:
[CHANGELOG](https://github.com/lacodda/lyrn/blob/main/CHANGELOG.md).

## Migrating from 1.x

Version 1 wrapped webpack and generated its configuration. That job belongs to
Vite now, and the wrapper is gone: `lyrn create`, `start`, `build` and `export`
no longer exist. An existing 1.x project keeps working — pin `lyrn@1.3.0`, or
move to Vite directly, which is what the 1.x templates were producing
configuration for anyway.

## Contributing

The gate, where things live and how commits are written:
[CONTRIBUTING.md](https://github.com/lacodda/lyrn/blob/main/CONTRIBUTING.md).

## License

MIT (c) [Kirill Lakhtachev](https://lacodda.com)
