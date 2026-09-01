---
title: Getting Started
description: Create a new web application on the lacodda line's stack with one command.
---

lyrn creates a repository that is ready to work in: Vite, React, TypeScript,
Tailwind and the [dowel](https://lacodda.github.io/dowel) design system, plus
the things a project usually grows only after the third time somebody wishes it
had them — a CI gate, a changelog, an ADR directory, an editor config.

## Install

```console
$ npm install -g lyrn
```

Or with cargo:

```console
$ cargo install lyrn
```

Or without a package manager at all:

```console
$ curl -fsSL https://raw.githubusercontent.com/lacodda/lyrn/main/tools/install.sh | sh
```

```powershell
irm https://raw.githubusercontent.com/lacodda/lyrn/main/tools/install.ps1 | iex
```

## Create a project

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

The project is already a git repository with one Conventional Commit in it, its
dependencies are installed, and `pnpm dev` runs.

## Check it

```console
$ cd demo-app
$ pnpm lint
```

That single script is eslint, `tsc --noEmit` and the tests — the same gate the
generated `.github/workflows/ci.yml` runs. A green terminal means a green pull
request.

## See it first

Nothing has to be written to find out what would be:

```console
$ lyrn new demo-app --dry-run
Would create 25 files in `demo-app`:

  .editorconfig
  .gitattributes
  .github/workflows/ci.yml
  ...
```

## Answer nothing

With a terminal attached lyrn asks for what it is missing. In a script or in
CI there is nobody to ask, so it takes the defaults and never blocks:

```console
$ lyrn new demo-app --yes
```
