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
| `--host <host>` | — | The application a plugin extends; `--form plugin` only |
| `--accent <colour>` | asked, else graphite | A product of the line, or a `#rrggbb` value |
| `--description <text>` | asked, else generic | One line describing what the project is |
| `--author <name>` | `git config user.name` | Recorded in LICENSE |
| `--path <path>` | `./<name>`; `.` for `docs` | Where to create it |
| `--repo <owner/name>` | looked up | The GitHub repository it will live in |
| `--with <addon,...>` | none | Optional pieces, per form; see [Forms](/reference/forms/) |
| `-y`, `--yes` | — | Accept the defaults instead of asking |
| `--dry-run` | — | Show what would be written, and write nothing |
| `--no-hooks` | — | Skip installing dependencies and starting the repository |

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

Naming an accent is what choosing a mark looks like from here — it is the one
thing `lyrn new` learns about a product's identity, and the whole palette is
derived from it. Giving the placeholder's own hex explicitly still counts as
unmarked, so nothing is quietly promoted.

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

A form decides what an add-on means for it. `--with i18n` on a `desktop`
project also writes the locale gate into `pnpm lint`; on a `spa` it writes the
same runtime without the desktop shell around it.

## The host

`--form plugin` is generated against the application the plugin extends, and
`--host` names it. There is no default: the host decides the protocol version
the plugin declares, the point it extends and the name it is discovered by, so
a guess would produce a plugin nothing runs.

```console
$ lyrn new wordcount --form plugin --host kilna
```

`lyrn forms` lists the hosts on offer. They are the applications that accept
plugins today; one whose turn has not come is refused with that reason, and so
is `--host` on a form that has no host:

```console
$ lyrn new demo --form plugin --host kasl
error: `kasl` does not accept plugins yet; when it does it will be listed here (known: kilna)

$ lyrn new demo --form cli --host kilna
error: the `cli` form is not generated against a host, so `--host kilna` means nothing
```

## Hooks

After the files are written, lyrn fetches what the project depends on - `pnpm
install` or `cargo fetch`, whichever the form uses - and starts the repository
with one Conventional Commit. `--form docs` only installs the site: the
repository already exists, and its history is yours. Both are skipped by `--no-hooks`, and a missing
tool is reported rather than fatal: the files are already on disk and the step
is one command away.
