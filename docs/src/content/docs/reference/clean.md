---
title: lyrn clean
description: Remove what a build or an install writes again - target/, node_modules/, dist/ - and say what each weighed.
---

```console
$ lyrn clean [path...] [--all] [--dry-run]
```

A repository of the line grows gigabytes it can write again in minutes: the
Rust build, the packages, the site's build. `lyrn clean` removes them and says
what each weighed.

```console
$ lyrn clean --dry-run
lyrn clean: ~/lyrn

would remove  target/               5.6 GB  cargo builds it again
would remove  docs/node_modules/  219.1 MB  the package manager installs it again
would remove  docs/dist/            2.0 MB  the build writes it again
would remove  docs/.astro/        714.4 KB  the site's build writes it again

5.8 GB to free: `lyrn clean` without --dry-run removes it.
```

## What it removes

A directory is removed only when both hold:

- **git ignores it** - a tracked file is never touched, and neither is
  anything git has not been told to ignore;
- **a tool of the stack writes it again** by itself: `target/` (cargo),
  `node_modules/` (the package manager), `dist/` (the build), `.astro/` (the
  site's build), `coverage/` (the tests).

An ignored file is not one: a local `.env` is ignored too, and nothing writes it
again. A link is not followed into the directory it points at. A directory of
the repository is cleaned as the whole of it, from its top. A file in use -
a program still running out of `target/` - is said and left; the rest goes, and
the command exits with 1.

## Every repository

`--all` cleans every project in [rigger](https://github.com/lacodda/rigger)'s
record, one line each, as [`lyrn doctor --all`](/reference/doctor/) reads them:

```console
$ lyrn clean --all --dry-run
lyrn clean: 18 repositories

  austeris       10.5 MB  docs/dist/ 9.5 MB, web/dist/ 970.0 KB, docs/.astro/ 23.2 KB
  furca          11.8 GB  target/ 11.2 GB, node_modules/ 319.0 MB, docs/node_modules/ 257.8 MB and 3 more
  kasl            4.5 GB  target/ 4.5 GB, docs/dist/ 3.3 MB, docs/.astro/ 30.3 KB
  kilna          12.4 GB  src-tauri/target/ 12.0 GB, node_modules/ 295.7 MB, docs/node_modules/ 154.6 MB and 3 more
  lyrn            5.9 GB  target/ 5.7 GB, docs/node_modules/ 219.1 MB, docs/dist/ 2.0 MB and 1 more
  midda         621.0 KB  dist/ 621.0 KB
  ...
  atlas        - retired in rigger's record on 2026-10-13

73.0 GB to free in 18 of 18 repositories.
Nothing was removed: `lyrn clean` without --dry-run removes it.
```

Each row names its largest three; `lyrn clean <path> --dry-run` lists all of a
repository's.

## Options

| Option | What it does |
| --- | --- |
| `[path...]` | The repositories to clean; defaults to the one the current directory is in |
| `--all` | Clean every project in rigger's record |
| `--dry-run` | Say what would be removed and what it weighs, and remove nothing |
