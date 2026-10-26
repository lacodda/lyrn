---
title: lyrn msrv
description: Measure the oldest Rust the code builds on - from the floor its edition and dependencies set, toolchain by toolchain - and write it as rust-version.
---

```console
$ lyrn msrv [path]
```

`rust-version` in `Cargo.toml` is a promise to everyone running
`cargo install`, and Cargo believes it. The standard's `msrv` job keeps the
number from rotting - it builds on exactly that toolchain - but it cannot say
whether the number is the true one. `lyrn new` writes the toolchain in the room,
which is the newest that builds the code, not the oldest. `lyrn msrv` measures
the oldest and writes it.

```console
$ lyrn msrv
lyrn msrv: ~/demo-tool, stable 1.99

Cargo.toml
  declared   1.99
  floor      1.85 - assert_cmd 2.2.2 asks for it
  install    1.85 ... done
  check      1.85 ... builds, 12s
  measured   1.85 - rust-version 1.99 → 1.85 in Cargo.toml [package]

removed the toolchains installed to measure: 1.85
```

## How it measures

The search starts at the floor the code cannot go under anyway: its edition -
2024 needs 1.85 - and the highest `rust-version` any package in the build
declares, platform-specific ones included, since the promise is for every
platform. The floor is tried first, and it is the answer more often than not.
Past it the toolchains up to stable are halved: one that builds the code is
taken to mean every newer one does too, so a handful of builds find the oldest.

Each toolchain is asked the question the `msrv` job asks -
`cargo check --all-targets` - with the lockfile as it is (`--locked`) and
`--ignore-rust-version`, because otherwise the number being measured would
answer it: Cargo refuses a package whose `rust-version` is above the toolchain
before compiling a line.

The builds go into a target directory of their own, which is removed after, so
`target/` keeps the build the gate made. A toolchain the machine lacks is
installed with rustup's minimal profile and uninstalled when the measuring is
done; stable is the one already there. `lyrn ci msrv` installs the toolchain it
needs when it runs.

## Where it writes

`rust-version` goes where the manifest keeps it: the crate's `[package]`, or the
workspace's `[workspace.package]` when the crate inherits it or the manifest is
a virtual workspace. A desktop app's shell, `src-tauri/Cargo.toml`, is measured
on its own, unless the top's workspace already holds it. A number that is
already right is left as it is.

[`lyrn doctor`](/reference/doctor/#msrv) holds the other half: that the number
is declared, and that a CI job builds on it.
