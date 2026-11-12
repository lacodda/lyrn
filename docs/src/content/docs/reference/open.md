---
title: lyrn open
description: Open one of the repository's shopfronts in the browser - the repository, the site, the CI, the newest release, the crate or the npm package.
---

```console
$ lyrn open [place] [--path <path>] [--print]
```

Every address comes from the repository itself, so the page opened is the one
the project points at:

| Place | Opens | Read from |
| --- | --- | --- |
| `repo` (the default) | the repository on GitHub | `origin`, else the manifests' `repository` |
| `docs` | the documentation site | the address its `astro.config.mjs` builds for |
| `ci` | the CI the release waits for, on GitHub | `[ci] workflow` in `release.toml`, else `ci.yml` |
| `release` | the newest release's page | the newest `v*` tag, by version |
| `crate` | the crate on crates.io | the publishable `Cargo.toml`, the product's own first |
| `npm` | the package on npm | the `package.json` that is not private |

```console
$ lyrn open release
opening https://github.com/lacodda/lyrn/releases/tag/v2.21.0
```

`--print` prints the address and opens nothing, for a terminal with no browser
or a script:

```console
$ lyrn open docs --print
https://lyrn.lacodda.com/
```

A place the repository has not got - no site, no package npm would take - is
refused with what is missing.

## Options

| Option | What it does |
| --- | --- |
| `[place]` | Which shopfront: `repo`, `docs`, `ci`, `release`, `crate` or `npm`; defaults to `repo` |
| `--path <path>` | The repository; defaults to the one the current directory is in |
| `--print` | Print the address instead of opening it |
