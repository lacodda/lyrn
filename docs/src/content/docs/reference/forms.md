---
title: lyrn forms
description: The shapes a generated project can take.
---

```console
$ lyrn forms
spa      Single-page app: Vite, React, TypeScript, Tailwind, dowel
cli      Command-line tool: Rust, clap, anyhow, dialoguer
  --with keyring       Secrets in the OS keyring, never in a config file
  --with self-update   A `self-update` command that reads the releases page
```

A form is the shape of the repository, not a choice of framework. The line runs
one stack; a form decides what kind of thing is being built with it.

## spa

A single-page application:

| | |
| --- | --- |
| Build | Vite 8 |
| UI | React 19, TypeScript |
| Styling | Tailwind 4 and the dowel theme |
| Tests | Vitest with jsdom, Testing Library |
| Lint | eslint, typescript-eslint, `dowel/no-raw-color` |

## cli

A Rust command-line tool:

| | |
| --- | --- |
| Arguments | clap, with derive |
| Errors | anyhow, printed with their cause chain |
| Prompts | dialoguer |
| Tests | `tests/cli.rs`, running the built binary |
| Release | Three targets, installers, an npm wrapper, crates.io over OIDC |

The binary is built with `lto`, one codegen unit and a stripped symbol table:
a release binary is downloaded far more often than it is built.

### Add-ons

The line's four CLIs agree on that core and disagree about everything else, so
the rest is optional. Neither is generated unless asked for - a project should
not carry code it never calls, or the dependencies behind it.

```console
$ lyrn new my-tool --form cli --with keyring,self-update
```

**`keyring`** adds a `secret` command that stores values in the OS keyring.
A secret never goes in a config file: config files are copied into backups,
synced between machines, pasted into issues and committed by accident.

**`self-update`** adds a command that reports whether a newer release exists.
It reads the tag from the redirect `/releases/latest` performs rather than the
REST API, which is capped at 60 unauthenticated calls an hour per address.

## Coming in 2.x

`desktop` (Tauri), `service` (axum with an SPA), `lib`, and the workspace and
plugin shapes the line already uses.
