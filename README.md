<p align="center">
  <img src="https://raw.githubusercontent.com/lacodda/lyrn/main/assets/banner.svg" width="720" alt="lyrn">
</p>
<h1 align="center">lyrn</h1>
<p align="center">Start a new web application on one finished stack — with one command.</p>

[![NPM Version][npm-image]][npm-url] ![License][license-url]

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

A form can carry optional pieces. The line's four CLIs agree on clap, anyhow
and dialoguer and disagree about everything else, so the rest is asked for:

```console
$ lyrn new my-tool --form cli --with keyring,self-update
```

`keyring` puts secrets in the OS keyring rather than a config file;
`self-update` adds a command that checks the releases page. Neither is
generated unless asked for — a project should not carry code it never calls.

More forms — `desktop`, `service`, `lib` — follow in 2.x.

## Migrating from 1.x

Version 1 wrapped webpack and generated its configuration. That job belongs to
Vite now, and the wrapper is gone: `lyrn create`, `start`, `build` and `export`
no longer exist. An existing 1.x project keeps working — pin `lyrn@1.3.0`, or
move to Vite directly, which is what the 1.x templates were producing
configuration for anyway.

## License

MIT — see [LICENSE](LICENSE).

[npm-image]: https://img.shields.io/npm/v/lyrn.svg
[npm-url]: https://www.npmjs.com/package/lyrn
[license-url]: https://img.shields.io/npm/l/lyrn.svg
