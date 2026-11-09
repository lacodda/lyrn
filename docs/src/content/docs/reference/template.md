---
title: lyrn template
description: Templates from outside the binary - where they come from, what they look like, and how lyrn checks them.
---

```console
$ lyrn template check [path]
$ lyrn template export <form> [--path <path>] [--repo <owner/name>]
$ lyrn template follow <path>... [--dry-run]
```

Every form lyrn carries is built into the binary, so `lyrn new` works offline.
`--template` generates from one that is not: a directory, one of yours under
`~/.lyrn/templates`, or a tag of a GitHub repository. `lyrn template check` holds
a template to what lyrn will accept, `lyrn template export` writes a built-in
form out as a template repository to start from, and `lyrn template follow`
keeps such a repository up with the form once lyrn moves on.

## Where a template comes from

`--template` is read by its shape alone, never by what happens to be on disk:

| Shape | Where it is read from |
| --- | --- |
| `./mine`, `../mine`, `/abs/path`, `~/mine` | That directory |
| `mine` | `~/.lyrn/templates/mine` |
| `owner/repo@tag` | That tag of the GitHub repository |

```console
$ lyrn new my-app --template ./my-template
$ lyrn new my-app --template house-spa
$ lyrn new my-app --template lacodda/template-spa@v1.0.0
```

`--template` and `--form` are one or the other: a template says which form it
is. `~/.lyrn` moves with `LYRN_HOME`.

### Your templates stand in for the forms

A directory in `~/.lyrn/templates` named after a form takes that form's place
everywhere a form is asked for - `lyrn new --form spa`, `lyrn init`, `lyrn
adopt` - and lyrn says so each time:

```console
$ lyrn new my-app
Using your template `/home/me/.lyrn/templates/spa` in place of the built-in spa form.
```

It has to be that form: a `~/.lyrn/templates/spa` whose manifest says
`form = "cli"` is refused, because `--form spa` would then mean something else
on one machine. `lyrn forms` lists your templates under the built-in ones, each
read the way `lyrn new` would read it, so one that cannot be used says why
before the day it is needed.

### From GitHub, only at a tag whose CI passed

A template from GitHub is used at a **tag**, never a branch, and only once its
own CI has passed on that tag:

1. The tag is resolved to its commit (`git ls-remote`).
2. GitHub is asked about the CI of **that commit** - its check runs and its
   commit statuses. It passes when something ran, everything finished, and
   nothing failed.
3. The tag is cloned, and the clone has to be the same commit. A tag moved in
   between is a refusal: the files a project is generated from are always the
   files the CI built.

```console
$ lyrn new my-app --template someone/template-x@v2
error: the CI of `someone/template-x@v2` failed: generate (router) (failure); lyrn does not generate from a template whose CI is red - to use it anyway, clone it and pass the directory: `--template ./template-x`
```

No CI result, a red one, one still running, or GitHub not answering - each is a
refusal, and there is no flag to skip the check. The way round is the one in
the message: a template you have cloned yourself is yours to judge.

lyrn asks GitHub anonymously, which is sixty questions an hour and enough for a
public template. For a private one, or a busier hour, it uses `GH_TOKEN`,
`GITHUB_TOKEN`, or the account `gh` is logged in as - and sends that token to
GitHub only.

The project records where it came from, so it can be named again:

```toml
# lyrn.toml
[template]
source = "lacodda/template-spa@v1.0.0"
revision = "3f9c1e0b7d2a4c6e8f1a3b5d7c9e0f2a4b6c8d0e"
```

A built-in form records `source = "lyrn"` and lyrn's version; a directory,
`local:<name>` or `path:<name>` and `unversioned`.

## What a template looks like

```text
template-spa/
├── template.toml        # the manifest
├── template/            # the project's files, placeholders in place
│   ├── package.json
│   ├── src/
│   └── ...
├── .github/workflows/   # the template's own CI - not the project's
└── README.md            # the template's own README
```

Only what is under `template/` reaches the project. Everything beside it - the
repository's README, its CI, its licence - belongs to the template.

### `template.toml`

```toml
form = "spa"            # the form this is a version of - required
lyrn = "2.9.0"          # the oldest lyrn that understands it
name = "spa"
description = "Single-page app: Vite, React, TypeScript, Tailwind, dowel"
standard = "2026.10"

verbatim = ["cliff.toml"]   # copied as they are, no placeholders
executable = ["install.sh"] # need the executable bit

[[hooks]]
name = "Installing dependencies"
run = ["pnpm", "install"]
optional = true

[[addons]]
name = "pwa"
summary = "Installable and offline: a manifest, its icons, a service worker"
files = ["public/sw.js", "src/pwa.ts"]
```

A field lyrn does not know is refused rather than read past: a template written
for a newer lyrn would otherwise generate something it never meant, silently.
`lyrn` says which version to update to.

### Placeholders

`{{ name }}` in a file or a file name is replaced with its value. `${{ }}`, the
GitHub Actions syntax, is left alone. A placeholder lyrn does not provide is an
error, never an empty string. What there is:

| Variable | Value |
| --- | --- |
| `name` | The project name, as given: `word-count` |
| `title` | The name as words: `Word Count` |
| `description` | The one-line description |
| `description_json`, `description_rust`, `description_html`, `description_comment` | The same, quoted for JSON (and YAML, TOML), a Rust literal, HTML text, a block comment |
| `title_json`, `core_description_json` | The title, and the workspace library's description, as JSON |
| `accent` | The accent colour, `#rrggbb` |
| `mark` | `chosen`, or `placeholder` while the project has no mark |
| `author` | Who LICENSE names |
| `repo`, `owner`, `repo_name` | `owner/name`, and its two halves |
| `form` | The form |
| `year`, `date` | Today, as `2026` and `2026-09-30` |
| `lib_name` | The name as a Rust identifier: `word_count` |
| `env_prefix` | The name shouted: `WORD_COUNT` |
| `msrv` | The Rust toolchain generating it, as `1.98` |
| `registry` | dowel's registry address |
| `lyrn_version`, `standard` | lyrn's version, and the standard the template writes |
| `template_source`, `template_revision` | Where the template came from, and its revision |
| `host`, `prefix`, `host_about`, `host_lookup`, `protocol_version`, `subject_about`, `target`, `target_about`, `target_list`, `command_key`, `command_label`, `bin_name` | The `plugin` form's host and its command |
| `type_name`, `command_fn`, `command_camel` | The `tauri-plugin` form's type and command |

### Sections

A few lines that only belong with an add-on sit between markers on lines of
their own:

```text
{{#pwa}}
import { registerServiceWorker } from './pwa'
{{/pwa}}
{{^pwa}}
// no service worker
{{/pwa}}
```

`{{#pwa}}` keeps its lines when `--with pwa` is given, `{{^pwa}}` when it is
not; the markers disappear either way. Sections do not nest, and each has to
name an add-on of the template. A whole file of one add-on is listed under its
`files` instead.

### Hooks

A template may run only what lyrn's own forms run: `pnpm install`, `npm
install`, `cargo fetch`, `git init`, `git add .` and `git commit -m <message>`.
Anything else refuses the template. Its CI proves the project builds; it says
nothing about what a hook would do to the machine it runs on.

With someone at the terminal, a template from outside shows the commands it
will run beside the tree of files it will write, before either happens.

## `lyrn template check`

Reads a template the way `lyrn new --template` does and renders it with none
and with all of its add-ons. Every problem is named at once:

```console
$ lyrn template check .
error: `.` is not a template lyrn can use:
  `src/App.tsx` asks for `{{ nmae }}`, which lyrn does not provide
  `index.html` line 9: the section `pwa` is not an add-on of the template
  the hook `Phoning home` runs `curl https://example.com`; a template may only install dependencies ...
```

A template repository's CI runs it first.

## `lyrn template export`

Writes a built-in form out as a template repository: its files under
`template/`, a `template.toml` saying what the binary says about them, a README,
a licence, and a CI that checks the template, generates every combination of
its add-ons and runs the gate each generated project ships with.

```console
$ lyrn template export spa --repo myorg/template-spa
Wrote the spa form as a template in `template-spa` (53 files).
```

Generating from the export gives the same project as the built-in form, byte
for byte - lyrn's own tests hold it to that for every form. Push it, tag it once
its CI is green, and it is usable as `myorg/template-spa@v1.0.0`.

The template's CI runs on the commits of `main` and on pull requests, not on
tags: lyrn asks about the CI of the commit a tag points at, so a run on the tag
would build the same files again, and the tag would read as still running
until it finished.

### Over an earlier export

`--path` may name an earlier export of the same form. The form is written over
it as this lyrn writes it: every file the form writes now, and the files under
`template/` it no longer writes removed. The licence stays as it is - it names
whoever holds the copyright, which is the repository's to say. `--repo`
defaults to the repository's `origin`.

```console
$ lyrn template export cli --path ../template-cli
Wrote the cli form of lyrn 2.19.0 over the template in `../template-cli`: 9 changed, 3 added, 0 removed.
```

An export of another form is refused, and so is a repository with changes not
yet committed: written over, the two would be mixed, and nobody could tell
afterwards which was which. A template you have changed by hand is not an
export any more; writing the form over it takes your changes out, and git is
where they are then.

## `lyrn template follow`

A template exported from a form is a copy of it at one version of lyrn, and the
form moves on - the [standard](/concepts/the-standard-by-version/) most of all.
`follow` takes each clone it is given through the whole of a release:

1. It brings the clone level with its upstream - its work tree has to be clean.
2. It writes the form over it, as `export --path` does.
3. It commits that as `feat: the cli form of lyrn 2.19.0`, with the standards
   it moved between in the body, and pushes it.
4. It waits for the CI of that commit, asking GitHub every fifteen seconds.
5. Once the CI has passed, it tags the commit with the next minor version - after
   `v1.1.0`, `v1.2.0` - and pushes the tag.

```console
$ lyrn template follow ../template-cli ../template-spa
lacodda/template-cli: the cli form, from lyrn 2.10.0 to 2.19.0, the standard 2026.10 to 2026.11.4: 9 changed, 3 added, 0 removed
  changed  .github/workflows/ci.yml
  ...
Pushed 4be1c7d0a9e2 to lacodda/template-cli: feat: the cli form of lyrn 2.19.0
Waiting for the CI of 4be1c7d0a9e2...
The CI of 4be1c7d0a9e2 passed.
Tagged lacodda/template-cli@v1.2.0 on 4be1c7d0a9e2, whose CI passed:
  lyrn new my-app --template lacodda/template-cli@v1.2.0
```

A form that writes nothing new leaves the template alone: a change of the
version of lyrn the template names, and nothing else, is not a release of it -
otherwise every release of lyrn would be one of every template. So a tag of a
template always says something in it changed, and a project made from it that
[`lyrn upgrade`](/reference/upgrade/#a-project-made-from-a-template) carries to
that tag has something to take.

A red CI stops it with the commit pushed and no tag, and names the jobs that
failed; mend the form in lyrn and follow again with the lyrn that has the fix.
No CI on the commit for five minutes, or a CI still running after an hour, stops
it the same way. Each clone is followed on its own, and the run ends non-zero
when one was not.

Pushing and tagging are done with your own git credentials; GitHub is asked
about the CI as for [a template from GitHub](#from-github-only-at-a-tag-whose-ci-passed).

| Option | What it does |
| --- | --- |
| `<path>...` | The clones of the template repositories, each the top of its own |
| `--dry-run` | Say what would change in each, and write, push and tag nothing |

## cargo-generate templates

A directory with a `cargo-generate.toml`, or a Cargo project with no manifest
at all, is read as a [cargo-generate](https://cargo-generate.github.io/cargo-generate/)
template - rendered by the same Liquid engine, with the same variables
(`project-name`, `crate_name`, `crate_type`, `authors`, `username`, `os-arch`,
`is_init`, `within_cargo_project`) and the same case filters (`kebab_case`,
`snake_case`, `pascal_case`, `upper_camel_case`, `lower_camel_case`,
`title_case`, `shouty_snake_case`, `shouty_kebab_case`).

```console
$ lyrn new word-count --template ./rust-cli-template --define edition=2024
```

- `ignore`, `include`, `exclude`, `.genignore`, `[conditional]` and `vcs` work
  as in cargo-generate; so does a `.liquid` suffix.
- `[placeholders]` are asked at a terminal, or answered with `--define
  key=value`, else take their default; `choices` and `regex` are enforced.
- A file whose Liquid does not render is kept as it is, with a warning - as
  cargo-generate does.
- Rhai hooks, the `rhai` filter and sub-templates are refused: lyrn does not
  run Rhai, and a template run without its hooks would produce a different
  project than cargo-generate does, with nothing saying so.
- The line's own flags - `--accent`, `--description`, `--repo`, `--host`,
  `--with` - mean nothing to such a template and are refused.

From GitHub, a cargo-generate template is held to the same rule as any other:
a tag whose CI passed.
