---
title: lyrn new
description: Create a new project from one of lyrn's forms.
---

```console
$ lyrn new <name> [options]
```

Creates a project in a directory named after it, or wherever `--path` points.
The destination has to be empty or absent — lyrn never writes into a directory
that already holds something.

The exception is `--form docs`, which adds a site to the repository it is run
in. It never replaces a file either: if anything it would write is already
there, it writes nothing and names each one. See [the docs form](/reference/forms/#docs).

## The name

Lowercase letters, digits and hyphens, starting with a letter. That is the
strictest of what a directory, a crate and an npm package can each be called,
so a name lyrn accepts works in all three:

```console
$ lyrn new Demo App
error: the name starts with `D`; it has to start with a lowercase letter
```

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `--form <form>` | `spa` | The shape of the project; see [Forms](/reference/forms/) |
| `--template <source>` | — | Generate from a template instead: a path, a name under `~/.lyrn/templates`, or `owner/repo@tag`; see [Templates](/reference/template/) |
| `--define <key=value>` | — | Answer a cargo-generate template's placeholder; repeatable |
| `--host <host>` | — | The application a plugin extends; `--form plugin` only |
| `--accent <colour>` | asked, else graphite | A product of the line, or a `#rrggbb` value |
| `--description <text>` | asked, else generic | One line describing what the project is |
| `--author <name>` | the global `git config user.name` | Recorded in LICENSE |
| `--path <path>` | `./<name>`; `.` for `docs` | Where to create it |
| `--repo <owner/name>` | looked up | The GitHub repository it will live in |
| `--with <addon,...>` | none | Optional pieces, per form; see [Forms](/reference/forms/) |
| `-y`, `--yes` | — | Accept the defaults instead of asking |
| `--dry-run` | — | Show the tree that would be written, and write nothing |
| `--no-hooks` | — | Skip installing dependencies, starting the repository and registering with turnout |

## Before anything is written

The whole project is planned before a single file is written, and the plan is a
tree. With someone at the terminal, lyrn shows it and asks before writing:

```console
$ lyrn new demo-app --accent kilna

Will create 27 files in `demo-app`:

  ├── .github/
  │   └── workflows/
  │       └── ci.yml
  ├── src/
  │   ├── components/
  │   │   └── ui/
  │   │       └── button.tsx
  │   ├── App.tsx
  ...
  └── vitest.config.ts

? Create them? (Y/n)
```

`--dry-run` prints the same tree and stops there; `--yes` writes without
asking. Because the tree comes from the same plan the write uses, the two
cannot disagree - and a destination the run would refuse is refused by the dry
run too.

## The accent

`--accent` takes either a colour or the name of a product on the line, in which
case that product's mark supplies the colour:

```console
$ lyrn new demo-app --accent kilna     # the mark's magenta
$ lyrn new demo-app --accent '#3fa873' # a colour of your own
```

It lands in `src/styles.css` as a single declaration, and the dowel theme
derives the rest from it:

```css
:root {
  --accent-base: #d9569e;
}
```

A project with no mark of its own yet gets neutral graphite — a placeholder
that reads as "this has not been drawn yet". `lyrn.toml` records which of the
two happened:

```toml
[project]
accent = "#6E7079"
mark = "placeholder"   # or `chosen`, once the product has a mark
```

`--accent <product>` starts a project on that product's identity whole: its
colour, its masters in `assets/` - all but the banner, which says its name - and
every raster drawn from them. A colour alone, `--accent '#a1b2c3'`, colours the
palette and leaves the umbrella mark on, so `mark` stays `placeholder`:
`lyrn.toml` says what `assets/` holds. [`lyrn assets --from`](/reference/assets/)
takes a mark of its own in later, and sets it to `chosen`.

On the `desktop` form the placeholder is visible rather than implied: the
application icon is the line's umbrella mark until you replace it. See
[the desktop form](/reference/forms/#the-icons).

## The repository

Installers, the npm wrapper and the update check all need to know where the
project lives. The owner is looked up rather than derived from a name: a GitHub
account is not a person's name, and turning "Jane Smith" into `jane-smith`
produces a URL that looks right and resolves to nobody. lyrn reads
`git config github.user`, then the account `gh` is logged in as; with neither,
it writes `OWNER` - visibly a blank rather than a plausible mistake.

```console
$ lyrn new my-tool --form cli --repo myorg/my-tool
```

## Add-ons

`--with` takes the optional pieces of a form, comma-separated. What is on offer
depends on the form, and `lyrn forms` lists them underneath it:

```console
$ lyrn new my-tool --form cli --with keyring,self-update
```

An add-on a form does not have is an error rather than a silent no-op.

A form decides what an add-on means for it: `--with i18n` on a `desktop`
project writes the translation runtime and puts the locale gate into
`pnpm lint`, so a key missing from one language fails the gate.

## The host

`--form plugin` is generated against the application the plugin extends, and
`--host` names it. There is no default: the host decides what the plugin is -
a command run on something the host hands over, or a transport for a file -
the protocol version it declares, and the name it is discovered by, so a guess
would produce a plugin nothing runs.

```console
$ lyrn new wordcount --form plugin --host kilna
$ lyrn new dropbox --form plugin --host sefy
```

An add-on of the plugin form is part of one host's protocol, and asking for it
with another host is refused with who takes it:

```console
$ lyrn new gitlab --form plugin --host kasl --with source
$ lyrn new demo --form plugin --host kilna --with source
error: `--with source` is part of the protocol of a kasl plugin; kilna asks its plugins nothing beyond the call every plugin answers
```

`lyrn forms` lists the hosts on offer. They are the applications that accept
plugins today; one whose turn has not come is refused with that reason, and so
is `--host` on a form that has no host:

```console
$ lyrn new demo --form plugin --host turnout
error: `turnout` does not accept plugins yet; when it does it will be listed here (known: kilna, kasl, sefy)

$ lyrn new demo --form cli --host kilna
error: the `cli` form is not generated against a host, so `--host kilna` means nothing
```

## From a template

`--template` generates from a template that is not built in - a directory, one
of yours under `~/.lyrn/templates`, or a tag of a GitHub repository whose CI
passed on it:

```console
$ lyrn new demo-app --template lacodda/template-spa@v1.0.0
Using the template lacodda/template-spa@v1.0.0 (commit 3f9c1e0b7d2a), which passed its CI.
```

Everything else works as with a form: the add-ons are the template's, the tree
is shown before anything is written, and `lyrn.toml` records where the files
came from. A directory under `~/.lyrn/templates` named after a form takes its
place even without `--template`. See [Templates](/reference/template/).

## Hooks

After the files are written, lyrn fetches what the project depends on - `pnpm
install` or `cargo fetch`, whichever the form uses - and starts the repository
with one Conventional Commit. `--form docs` only installs the site: the
repository already exists, and its history is yours. Both are skipped by `--no-hooks`, and a missing
tool is reported rather than fatal: the files are already on disk and the step
is one command away.

## turnout

A project that describes itself to [turnout](https://lacodda.github.io/turnout/) -
the service form writes a `turnout.json` - is put into turnout's catalog once
the hooks have run: its commands, what a deploy uploads, and no gateway port,
since it talks to its own stand. The stand is then one
[`lyrn stand up --server NAME`](/reference/stand/#up) away.

```console
Registered with turnout as demo-svc, its stand on port 8100.
```

The port is the first of `8100-8199` no stand the machine knows holds - see
[the port](/reference/stand/#the-port). turnout missing, or not set up, is said
and is not fatal: `lyrn stand up` registers the project once it is. An app of
the same name elsewhere is left alone, and named. `--no-hooks` skips this too.
