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
desktop  Desktop app: Tauri 2 around the spa stack
  --with i18n          i18next, with a gate holding every locale to the source
service  HTTP service: axum, sqlx, Postgres
  --with spa           A web UI compiled into the binary and served by it
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

## desktop

The `spa` stack with a Rust shell around it:

| | |
| --- | --- |
| Shell | Tauri 2 |
| Frontend | Everything the `spa` form writes |
| Commands | `src-tauri/src/commands.rs`, reachable from the webview |
| Permissions | `src-tauri/capabilities/`, granted per window |
| CI | The frontend gate, plus Rust on three platforms |

`main.rs` is deliberately thin - the application lives in `lib.rs`, which is
what lets tests reach it without opening a window. On Windows the binary is
built with `windows_subsystem = "windows"`, or a console would sit behind the
app in release builds.

### The icons

`src-tauri/icons/` ships a placeholder: a graphite tile with `??`, plainly not
a real mark. It is there because a Tauri build on Windows fails outright
without an `.ico` - a generated project has to build on the first try.

Replace it with `pnpm tauri icon path/to/mark.png` once the mark exists. In the
`.ico` the **largest image must come first**: Windows reads the first entry for
the taskbar and the title bar, and a 16px one there leaves the application
looking blurred everywhere that matters.

## service

An HTTP service:

| | |
| --- | --- |
| Server | axum |
| Database | Postgres through sqlx, migrations as plain SQL |
| Config | Environment only - the same binary runs anywhere |
| Errors | One type, mapped to status codes once |
| Deploy | A Dockerfile and a compose file for a stand |

Migrations run at startup, so a binary and the schema it expects travel
together and there is no window where one is ahead of the other.

The tests talk to a **real Postgres** and skip themselves when `DATABASE_URL`
is unset - right on a laptop without one. The CI the form ships provides a
database, so the suite is never green and blind.

### `--with spa`

Adds a web UI under `frontend/`, compiled into the binary with `rust-embed`. A
self-hosted install is then one file, and the UI can never be from a different
build than the API it calls.

Anything the API has not claimed falls through to the app, which owns its own
paths - without that, a refresh on any route but `/` would 404: the server has
no such file, and only the app knows what to draw there.

Left out, `--with spa` is what a service with no interface of its own needs -
a sync relay, a webhook receiver, an API somebody else's frontend calls.

## Translation

`--with i18n` is offered by `desktop`; `spa` gets it in 2.6, with its own add-ons. It writes i18next with English
and Russian, and `tools/check-locales.mjs` into `pnpm lint`.

English is the **source**, never a fallback. A missing translation fails the
build rather than quietly rendering in English, because a half-translated
screen is the kind of thing nobody reports and everybody notices. The gate also
holds `{{placeholder}}` names to the source, so a translation cannot silently
drop the one that carries a number.

## Coming in 2.x

`lib`, and the workspace and plugin shapes the line already uses.
