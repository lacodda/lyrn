---
title: lyrn deps
description: When the stack was last brought up to date, and which dependencies are behind - the versions a build uses beside the newest there are.
---

```console
$ lyrn deps [path]
```

The line brings its whole stack up to date at the start of a version, at most
once a week, in one `chore(deps)` commit - major versions included, with the
code they break fixed in the same commit. `lyrn deps` says whether the week has
passed, read from the history rather than remembered, and what an update would
move.

```console
$ lyrn deps
lyrn deps: ~/lyrn

stack     updated 6 days ago, on 2026-10-13 - holds until 2026-10-20
on        edition 2024 · msrv 1.88 · node 22 · pnpm 10.34.7 · actions on the standard
rust      stable 1.99.1, the newest

Cargo.toml - crates.io
  (17 up to date)

docs/package.json - npm
  astro               7.3.8   → 7.4.0
  (2 up to date)

1 dependency behind.
```

- **stack** is the last `chore(deps)` (or `build(deps)`) commit; a project's
  first commit counts, since it starts on the stack of its day. Once a week has
  passed the next stage starts with an update, and a version that began on an
  older stack and shipped without one is named.
- **on** is the stack [`lyrn doctor`](/reference/doctor/#the-stack) reads,
  each cell behind the standard saying what the standard is on.
- **rust** is `rustup check` about the stable toolchain, when rustup is there.

Each package that builds here - every workspace member in a Cargo.lock, every
importer in a pnpm lockfile - is listed with its direct dependencies at the
version the lockfile resolves, beside the newest stable version on crates.io
(read from its sparse index) or the version npm installs by default. A step
across a breaking version - by Cargo's reading of semver, where `0.13` to
`0.14` is one - is marked, since that is the one whose code has to change in
the same commit.

`lyrn deps` says; it updates nothing and exits with 0.
