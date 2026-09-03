---
title: lyrn forms
description: The shapes a generated project can take.
---

```console
$ lyrn forms
spa         Single-page app: Vite, React, TypeScript, Tailwind, dowel
cli         Command-line tool: Rust, clap, anyhow, dialoguer
  --with keyring       Secrets in the OS keyring, never in a config file
  --with self-update   A `self-update` command that reads the releases page
desktop     Desktop app: Tauri 2 around the spa stack
  --with i18n          i18next, with a gate holding every locale to the source
service     HTTP service: axum, sqlx, Postgres
  --with spa           A web UI compiled into the binary and served by it
workspace   Cargo workspace: a library crate plus the CLI that uses it
  --with keyring       Secrets in the OS keyring, never in a config file
  --with self-update   A `self-update` command that reads the releases page
mono        pnpm monorepo publishing a TypeScript package to npm
  --with stand         A Vite page in the workspace where the package runs
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

## workspace

The `cli` form with its logic moved into a library crate of its own:

| | |
| --- | --- |
| Layout | `crates/<name>-core` (the library) and `crates/<name>` (the binary) |
| Version | One, in `[workspace.package]` - both crates release together |
| Errors | An enum in the library, `anyhow` chains at the binary's edge |
| Docs | rustdoc with doctests, checked by CI |
| Publish | Two crates in order, the library first |

Reach for it when the logic is worth depending on without the command line
attached - which on this line is most of them once they grow: sefy publishes
its core so a plugin can link it, and a library is the only shape another
program can call.

The library owns the reasons things fail. Its errors are an enum a caller can
match on, not a formatted string; the binary turns them into a chain a person
reads. That is the whole reason for the split, and it is why the form ships a
`greet` that returns `Result` rather than a function that cannot fail.

### Publishing two crates

`publish.yml` publishes the library, waits for it to appear in the crates.io
index, then publishes the binary. crates.io resolves the dependency at publish
time and refuses a package whose dependency it cannot see yet - and the index
does not carry a new version the instant `publish` returns.

Both crates need **their own trusted publisher** on crates.io, and the first
version of each has to go up by hand: a publisher cannot be attached to a crate
that does not exist. Missing the second one fails with a message about an
invalid token, which is not what is wrong.

### The MSRV job

CI reads `rust-version` as the **maximum** across the workspace's packages
rather than from the first one cargo happens to list. `.packages[0]` is not a
promise, and testing whichever crate came first is green on half a repository.

### Add-ons

The same two the `cli` form has, in the same place: `keyring` and
`self-update` live in the binary, because a library has no business reaching
for the user's keychain or replacing an executable on disk.

## mono

A pnpm workspace that publishes a TypeScript package:

| | |
| --- | --- |
| Layout | `packages/<name>` publishes; the root is `private: true` |
| Build | `tsc`, not a bundler - a library ships modules |
| Resolution | `nodenext` in the package, `bundler` everywhere else |
| Tests | vitest at the workspace root, one runner for every member |
| Publish | npm over OIDC, with provenance |

The line has **no standalone publishable TypeScript package**: dowel, kjui and
lyrnui are all monorepos whose root is private and whose shipped artefact sits
under `packages/`. So this form is the monorepo, and the package lives inside
it - one member today, room for the second the day it is needed.

### Why not a bundler

The package is compiled by `tsc` with `moduleResolution: nodenext`, and its
sources import each other with the `.js` extension the emitted files will have.

Under `bundler` - which is right for everything a bundler consumes, and is what
the workspace's own `tsconfig.base.json` keeps - an extensionless specifier
typechecks, `tsc` emits it unchanged, and the package builds, packs and
publishes. Node's resolver then refuses it. The failure belongs entirely to
whoever installs the package, and nothing in this repository would have caught
it (ADR 0002).

So CI installs the packed tarball into a scratch directory and imports it with
a plain `node`. A different process, outside the workspace, resolving the way a
consumer's does - which is the only thing that can answer this question.

### `--with stand`

Adds a second workspace member: a Vite page that imports the package **by its
published name**. pnpm resolves that to the workspace member, so the page
renders the built package rather than its source - if the build is broken, the
stand is too, which is the point of having one.

With a stand, `pnpm lint` builds the package first: the stand's typecheck
resolves the package through its `exports`, which point at `dist`.

Its test asks for a DOM in the file itself, with an `@vitest-environment`
docblock, rather than in the runner's config. Only the stand renders anything,
and a jsdom the package's tests never touch costs more than twenty seconds of
start-up - once enough to push a healthy run past the worker timeout.

## Translation

`--with i18n` is offered by `desktop`; `spa` gets it in 2.6, with its own add-ons. It writes i18next with English
and Russian, and `tools/check-locales.mjs` into `pnpm lint`.

English is the **source**, never a fallback. A missing translation fails the
build rather than quietly rendering in English, because a half-translated
screen is the kind of thing nobody reports and everybody notices. The gate also
holds `{{placeholder}}` names to the source, so a translation cannot silently
drop the one that carries a number.

## Coming in 2.x

The `egui`, plugin and docs shapes the line already uses.
