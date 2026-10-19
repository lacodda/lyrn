---
title: lyrn ci
description: Run here the CI a push of this commit would start - on a clean checkout of the commit, before pushing it.
---

```console
$ lyrn ci [job...] [--path <path>]
$ lyrn ci --list
```

A red CI on a release commit is the most expensive kind: the tag waits, the
fix is one more commit on a history that was meant to be finished, and the
reason is usually something a terminal could have shown - a file the commit
needs left untracked, a lint the newest toolchain added, a promised
`rust-version` the code has outgrown, an advisory published since the last
push. `lyrn ci` reads the workflows the commit carries, works out which of them
a push of the branch starts, and runs them here, first.

```console
$ lyrn ci
lyrn ci: ~/word-count at 25bd80d, as a push of `main` would run it
Checked out clean in ~/.cache/lyrn/ci/word-count-efe09fbe/checkout

audit.yml
  Crates - a Linux job, run here on Windows
    · actions/checkout@v7 - the commit is checked out clean
    ✓ EmbarkStudios/cargo-deny-action@v2  1s
  ✓ Crates passed, 1s

ci.yml
  windows-latest
    · actions/checkout@v7 - the commit is checked out clean
    · dtolnay/rust-toolchain@stable - stable, as installed here
    · Swatinem/rust-cache@v2 - nothing to cache - the copy keeps target/ and node_modules/
    ✓ Format  3s
    ✓ Lint  9s
    ✓ Test  14s
    ✓ Build  21s
  ✓ windows-latest passed, 47s
  msrv - a Linux job, run here on Windows
    · actions/checkout@v7 - the commit is checked out clean
    ! Read rust-version from the manifest  0s - needs `jq`, which is not installed here; the commit is not at fault
    - dtolnay/rust-toolchain@master - not run: a step before it could not run here
    - Swatinem/rust-cache@v2 - not run: a step before it could not run here
    - Build on the declared MSRV - not run: a step before it could not run here
  ! msrv could not run here: "Read rust-version from the manifest" needs `jq`, which is not installed here, 0s
  + 2 more rows on Linux and macOS: the same steps as the Windows rows here, CI's alone

docs.yml
  build - a Linux job, run here on Windows
    · actions/checkout@v7 - the commit is checked out clean
    ! pnpm/action-setup@v6 - pnpm 10.34.6 here; the job sets up the newest 10, which may be newer
    ! actions/setup-node@v7 - Node v22.18.0 here; the job sets up the newest 22, which may be newer
    ✓ Install  2s
    ✗ Build  1s
        > astro build

        Node.js v22.18.0 is not supported by Astro!
        Please upgrade Node.js to a supported version: ">=22.22.0"
         ELIFECYCLE  Command failed with exit code 1.
        whole log: ~/.cache/lyrn/ci/word-count-efe09fbe/logs/05-05-build.log
  ✗ build failed at "Build", 3s
  - deploy - needs build, which did not pass

publish.yml - not started: it starts on a pushed tag, not a branch

release.yml - not started: it starts on a pushed tag, not a branch

1 job failed, 2 jobs passed, 1 job could not run here, 1 not run here in 58s
  ✗ docs.yml: build (at "Build")
  ! ci.yml: msrv: "Read rust-version from the manifest" needs `jq`, which is not installed here - install it and run again
Logs: ~/.cache/lyrn/ci/word-count-efe09fbe/logs
```

It exits with 0 only when every job that runs here passes: a failed job and a
job this machine could not stand in for both exit with 1.

## What runs, and where

**The commit, not the work tree.** The run happens in a clean checkout of the
commit - a clone sharing the repository's objects, kept in the user's cache
(`%LOCALAPPDATA%\lyrn\ci` on Windows, `~/Library/Caches/lyrn/ci` on macOS,
`~/.cache/lyrn/ci` elsewhere) and cleaned before each run of everything the
commit does not track, except `target/` and `node_modules/`, which CI keeps in
its caches too. A file the build needs and nobody committed fails here as it
fails there. The remote is set to the repository's own, so a test that reads
where the repository lives reads GitHub's address, as on CI.

**The workflows a push starts.** A workflow runs when a push of the branch
checked out would start it: `on: push` with its `branches`, `branches-ignore`,
`paths` and `paths-ignore` read the way GitHub reads them, against the files
the commits not yet pushed change. A workflow that starts only on a tag - the
release, the publish - never runs here, nor does one that starts only on a pull
request, a schedule or by hand. `--list` says what would run and why the rest
would not, and runs nothing; naming jobs runs only those.

**The rows of this system.** A matrix is expanded by GitHub's rules - the
product of its lists, less what `exclude` names, with each `include` merged
into every combination it does not contradict - and each combination's
`runs-on` says its system. The rows for this machine's system run; the same
steps on another system are left to CI and counted in one line. A job CI runs
only on another system - msrv and audit jobs usually live on Linux - runs here
all the same, marked as crossed: they are where red CI comes from, and they
seldom care which system builds them.

**The steps, as a runner runs them.** Each `run:` script is written to a file
and started in the shell a runner would use, the way it would start it: bash
with `-e` (and `-o pipefail` when the workflow names bash) on Linux and macOS -
Git's bash on Windows, never the one in System32 - and PowerShell on Windows,
stopping on an error and exiting with the last command's code. Without
PowerShell 7 the scripts run in Windows PowerShell, which the run says once.
The environment is a runner's: `CI`, `GITHUB_ACTIONS`, `GITHUB_WORKSPACE`,
`GITHUB_SHA`, `GITHUB_REF`, `RUNNER_OS`, `RUNNER_TEMP` and the rest; the
`env:` of the workflow, the job and the step; and `$GITHUB_OUTPUT`,
`$GITHUB_ENV` and `$GITHUB_PATH`, read back after each step as the runner reads
them. `${{ }}` expressions and `if:` conditions are evaluated as GitHub does -
`matrix`, `runner`, `github`, `env`, `steps` and `needs` are known, and a step
that needs anything else (`secrets`, `vars`) is not run.

**Actions, for what they mean here.** An action is GitHub's code, so it is read
for what it does rather than run:

| Action | Here |
| --- | --- |
| `actions/checkout` | The clean checkout is already there |
| `dtolnay/rust-toolchain`, `actions-rust-lang/setup-rust-toolchain` | The job's later steps build with that toolchain; a version not installed is installed with rustup, as the runner installs it. A `rust-toolchain.toml` decides, here as on CI |
| `actions/setup-node`, `pnpm/action-setup` | The installed version beside the one the job asks for; a job naming only a major gets the newest of it on CI |
| `EmbarkStudios/cargo-deny-action` | `cargo deny` with the action's inputs, when cargo-deny is installed |
| `actions/cache`, `Swatinem/rust-cache` | Nothing: the copy keeps its build |
| anything else | Not run - only GitHub runs it |

## What never runs here

A step whose script publishes past this machine - `cargo publish`,
`npm publish`, `pnpm publish`, `git push`, `gh release`, `docker push` and
their like - is never run, nor is one that changes the system: `sudo`, `apt`,
`brew install`. A container job and a reusable workflow are only GitHub's. A
job's `services:` are not started; its steps reach whatever answers where they
look, and the run says so.

## Could not run here

A step that stops on a program this machine does not have - `jq: command
not found` - is not a failure of the commit, and is not reported as one: the
job could not run here, the run names the program, and exits with 1 until it
is installed. The same goes for a step whose output the job builds on and that
could not run, and for a toolchain that comes out empty: the steps after it
are not run, because a green there would be a check of something else than CI
checks.

## Options

| Option | What it does |
| --- | --- |
| `[job...]` | Only these jobs, by their id in the workflow |
| `--path <path>` | The repository; defaults to the one the current directory is in |
| `--list` | What would run and what would not, without running anything |
