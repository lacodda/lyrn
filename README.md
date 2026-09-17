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
time somebody wishes it had them — a CI gate, a changelog, an ADR directory, an
editor config, a license.

It is not a scaffolder for any framework you like. It is **one stack, assembled
to the end**. The choice a generator usually hands back to you has already been
made, and made the same way for every product on the line.

## A day in the life

```console
$ lyrn new demo-app --accent kilna
Created 25 files in `demo-app`.
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
> eslint . && tsc --noEmit && pnpm test

 Test Files  1 passed (1)
      Tests  2 passed (2)
```

That gate is the same one CI runs, so a green terminal means a green pull
request.

## Install

```console
$ npm install -g lyrn          # or: cargo install lyrn
```

## Usage

```console
$ lyrn new <name> [options]
$ lyrn forms
```

| Option | What it does |
| --- | --- |
| `--form <form>` | The shape of the project (default: `spa`) |
| `--accent <colour>` | A product of the line, or a `#rrggbb` value |
| `--description <text>` | One line describing what the project is |
| `--author <name>` | Recorded in LICENSE; defaults to `git config user.name` |
| `--path <path>` | Where to create it |
| `--repo <owner/name>` | The GitHub repository it will live in |
| `--with <addon,...>` | Optional pieces of the form, comma-separated |
| `-y`, `--yes` | Accept the defaults instead of asking |
| `--dry-run` | Show what would be written, and write nothing |
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

A form can carry optional pieces. The line's four CLIs agree on clap, anyhow
and dialoguer and disagree about everything else, so the rest is asked for:

```console
$ lyrn new my-tool --form cli --with keyring,self-update
```

`keyring` puts secrets in the OS keyring rather than a config file;
`self-update` adds a command that checks the releases page; `i18n` adds
i18next with a gate that holds every locale to the source language; `spa` gives
a service a web UI compiled into its binary. None is generated unless asked for
— a project should not carry code it never calls.

`--form workspace` is the `cli` form with its logic in a library crate of its
own: `crates/<name>-core` publishes to crates.io beside the binary, so another
program can call the logic without taking the command line with it.

`--form mono` is a pnpm workspace publishing a TypeScript package from
`packages/<name>`: the shape dowel, kjui and lyrnui all take. It is built with
`tsc` rather than a bundler, and CI installs the packed tarball elsewhere and
imports it with a plain `node` — the only check that catches a package which
builds and cannot be imported. `--with stand` adds a page where it runs.

More forms — egui, plugins, docs sites — follow in 2.x.

## Status

v2.5.3, in daily use. All six forms work today, and each one is generated and
put through its own gate on Linux, macOS and Windows on every push. What landed
in each version:
[CHANGELOG](https://github.com/lacodda/lyrn/blob/main/CHANGELOG.md).

## Migrating from 1.x

Version 1 wrapped webpack and generated its configuration. That job belongs to
Vite now, and the wrapper is gone: `lyrn create`, `start`, `build` and `export`
no longer exist. An existing 1.x project keeps working — pin `lyrn@1.3.0`, or
move to Vite directly, which is what the 1.x templates were producing
configuration for anyway.

## License

MIT (c) [Kirill Lakhtachev](https://lacodda.com)
