---
title: lyrn check
description: Hold the shopfronts to each other - versions, descriptions, the commands the README shows, where the documentation site lives - on every commit.
---

```console
$ lyrn check [path...] [--json]
$ lyrn check --all [--json]
$ lyrn check --explain <check>
```

A product of the line is shown in several places at once - crates.io, npm,
GitHub, the documentation site, the installers - and each place renders its own
copy of what the product is called, which version it is, what it says about
itself and which commands it has. They drift without a sound: nothing fails
when `npm/package.json` still says the old number, or the README shows a
command that was renamed, and the drift is only seen once it is published.

`lyrn check` reads the shopfronts of a repository - the one the current
directory is in, or the one at `path` - and holds them to each other. It finds
them by the standard's layout rather than by a list, so the file nobody wrote
into a list is not the one left behind. It runs in a gate, on every commit,
rather than on the day of the release:

```console
$ lyrn check
lyrn check: ~/word-count, word-count 0.2.0

✗ versions
    ✗ npm/package.json says 0.1.0, the product is 0.2.0 (Cargo.toml)
      → write 0.2.0 into npm/package.json
    ✗ npm/package.json pins the binary at v0.1.0 (`word-count.binary`), the product is 0.2.0
      → set `word-count.binary` to v0.2.0: the wrapper downloads the release it names
    why: lyrn check --explain versions
✗ changelog
    ✗ CHANGELOG.md has no section for 0.2.0, the version the manifests say
      → write it in the commit that moves the number: `git cliff --unreleased --tag v0.2.0 --prepend CHANGELOG.md`
    why: lyrn check --explain changelog
✓ descriptions  Cargo.toml and npm/package.json, docs/astro.config.mjs say the same
✓ readme        one README, its 3 links absolute
✗ commands
    ✗ README.md:27 shows `word-count count`, which `word-count --help` does not list
      → show a command the program has, or give the program the command
    why: lyrn check --explain commands
✓ site          https://lacodda.github.io/word-count/, a project site
✓ installers    2 installers fetch releases from lacodda/word-count by the redirect

3 failed, 4 passed
```

A failure (`✗`) makes `lyrn check` exit with 1, so it works as a gate: put it
after the tests, which have just built the program the `commands` check reads.
Every repository the line starts carries the gate already, as
`.github/workflows/shopfronts.yml`: on every push - documentation alone
included, since the shopfronts are mostly documentation - it installs the lyrn
that wrote the workflow from its release and runs `lyrn check`. A newer lyrn
arrives with the next version of the standard, through
[`lyrn upgrade`](/reference/upgrade/).
A check that does not apply - `site` in a repository with no documentation
site - says why and counts as neither. `lyrn check` reads the repository and
writes nothing to it.

The release itself - moving the number, writing the changelog, tagging,
checking the registries afterwards - is the release engine's, `furca release`.
`lyrn check` keeps the shopfronts agreeing in between, and it knows which files
are shopfronts, which is how it can tell `release.toml` that it forgot one.

## The checks

### versions

Every manifest of the product says one version: the top Cargo.toml, the npm
wrapper's `package.json`, an application's `tauri.conf.json` and its
`package.json`, a frontend's or a documentation site's `package.json` when they
carry the number at all. A manifest belongs to the product unless it is a
test's material or an example, is private and keeps `0.0.0` or `0.0.1` - no
version of its own - or is published under a name that is not the product's
and released on its own. The number the top manifest says is the product's.

An npm wrapper pins the release it downloads its binary from
(`"word-count": { "binary": "v0.2.0" }`), and the pin says `v` and the same
number: a pin that lags installs the previous program under the new version.
Cargo.lock records the workspace's own packages too, and a lock still holding
the old number is refused by every `--locked` build. And when the repository
keeps a `release.toml`, its `project.manifests` lists every manifest that
carries the number, since `furca release` rewrites only the files it lists.

### changelog

`CHANGELOG.md` has a section for the version the manifests say - `## [1.2.0] -
date`, `## 1.2.0`, `## [v1.2.0]`. The number moves in the release commit, and
its section moves with it: the release page is written from it. Before the
first release, a changelog with no version sections passes.

### descriptions

The product says the same sentence about itself on every shopfront. The top
manifest says it first - the top Cargo.toml's package, else the top
`package.json` of an application - and its npm wrapper, the top `package.json`
of an application and the `description` of the Starlight site say it in the
same words, give or take the full stop a caption leaves off and the curl of a
quote. A library published beside the product describes that library and is
left alone. A workspace has no top package to speak for the whole, so there
only an npm wrapper is held to the crate it installs.

### readme

One README for every registry: the root one, which a crate names in `readme`
and an npm wrapper copies in when it is packed. A README beside a published
package of the product fails, and so does a crate whose `readme` names
anything but the root README. Every link and image in the root README is
absolute - `assets/logo.svg` resolves on GitHub and nowhere else; `http`,
`https`, `mailto` and `#anchors` pass, and links in code are not links.

### commands

Every `$ <program> <command>` line in the README and in the pages under
`docs/` names a command `<program> --help` lists, and a second word a
subcommand of it when the command has some. Decisions in `docs/adr/` are
history and are left out. The programs are the product's own Cargo packages
that parse their arguments with clap, and the help is read from the build in
`target/debug` - built first, with `--locked`, when a source is newer. A program
that takes flags and no commands - a service - passes with nothing to page, and
a line that gives it a command fails.

On a Starlight site, every command has a page under
`src/content/docs/reference/` named after it, or is named as `<program>
<command>` on a page of the site: a command without a page does not exist. And
`--help` itself has to answer with no environment at all - a program that
reads its configuration before its arguments cannot say what it does.

### site

The documentation site agrees with itself about where it lives. Two
arrangements are consistent: a project site on `owner.github.io/name`, built
with `base: '/name'`, or a domain of its own named in `public/CNAME`, built for
that domain with no `base` and with links written from the root. A mixture
fails - every asset is asked for under a path the server does not have - and
so does a `/name/` link left in the content of a site that has a domain.
`site` and `base` are read from `astro.config.mjs` written in place, as a
constant, or as the parts of a `new URL(...)`.

The `homepage` of the product's published manifests (and a crate's
`documentation`, unless it is docs.rs) leads to wherever the site is.

### installers

`tools/install.sh` and `tools/install.ps1` name the repository they belong to,
read from the remote or a manifest's `repository`; they read the newest tag
from the redirect of `/releases/latest`, never from `api.github.com`, which
answers sixty anonymous questions an hour per address; and a `cargo install
<name>` they suggest names a crate of the repository.

## Many repositories

Name more than one, and the report is a table: a mark per check, a row per
repository. `--all` checks every project in
[rigger](https://github.com/lacodda/rigger)'s record, as
[`lyrn doctor --all`](/reference/doctor/#every-repository) does.

```console
$ lyrn check word-count lyrn
lyrn check: 2 repositories

            versions changelog descriptions readme commands site installers
word-count  ✗        ✗         ✓            ✓      ✗        ✓    ✓
lyrn        ✓        ✓         ✓            ✓      ✓        ✓    ✓

1 fails, 1 passes
`lyrn check <path>` says what is off in one of them and how to put it right
```

## JSON

`--json` prints one document of the same shape for one repository or many:

```json
{
  "lyrn": "2.13.0",
  "repositories": [
    {
      "name": "word-count",
      "path": "/home/ada/word-count",
      "product": "word-count 0.2.0",
      "result": "fail",
      "reason": null,
      "checks": [
        {
          "check": "versions",
          "result": "fail",
          "text": null,
          "findings": [
            {
              "level": "fail",
              "what": "npm/package.json says 0.1.0, the product is 0.2.0 (Cargo.toml)",
              "fix": "write 0.2.0 into npm/package.json"
            }
          ]
        }
      ]
    }
  ],
  "skipped": []
}
```

`result` is `pass`, `warn`, `fail`, or `missing` for a path that is not there;
a check's `result` is also `skip` when it does not apply.

## Options

| Option | What it does |
| --- | --- |
| `[path...]` | The repositories to check; defaults to the one the current directory is in |
| `--all` | Every project in rigger's record |
| `--json` | One JSON document instead of the report |
| `--explain <check>` | Why a check exists and how to satisfy it, instead of checking |
