---
title: lyrn docs
description: The documentation site - a page written from the program, the site held to the program, the site served.
---

```console
$ lyrn docs ref [command] [--path <path>]
$ lyrn docs guide <title> [--description <text>] [--path <path>]
$ lyrn docs concept <title> [--description <text>] [--path <path>]
$ lyrn docs check [path]
$ lyrn docs dev [path]
```

The line's rule is that a command without a page does not exist, and a rule with
no tool behind it is kept by remembering. These work on the Starlight site of the
repository, wherever it is - `docs/` or `docs/site/`; a repository with none is
pointed at [`lyrn new --form docs`](/reference/forms/#docs).

## lyrn docs ref

Writes a command's reference page from its own `--help`: the usage, what it does,
a table of arguments and options, and a section for each subcommand, so the first
draft is right about every flag. It reads the program as built in `target/`,
building it first if a source is newer.

```console
$ lyrn docs ref adr
Wrote docs/src/content/docs/reference/adr.md.
```

With no command it writes every page that is missing, and says so when none is. A
page already there is never written over: a page someone has written is worth more
than a first draft. The draft says what the command is; why it exists and what
comes back from it are still for a person to write.

## lyrn docs guide, lyrn docs concept

Start a page: a guide walks through doing something, a concept says what something
is and why. The title is the page's title and, in lowercase, its file's name under
`guides/` or `concepts/`; `--description` is the line for search results and the
page's metadata, and without it `docs check` keeps asking.

```console
$ lyrn docs guide "Your first release" --description "From a green CI to a tag."
Wrote docs/src/content/docs/guides/your-first-release.md.
The sidebar of docs/astro.config.mjs lists no `guides/`: `{ label: 'Guides', items: [{ autogenerate: { directory: 'guides' } }] }` shows it.
```

## lyrn docs check

Says what the program has and the site does not, and what the site says that is
no longer so. It exits with 1 on a failure, and warnings are for reading.

```console
$ lyrn docs check
lyrn docs check: ~/lyrn, the site in docs/

✓ commands  89 commands shown, all real, 20 commands with a page
✓ flags     74 flags on their commands' pages
✓ pages     22 pages with a title and a description
✓ links     58 links inside the site, all answering
✓ sidebar   2 directories listed

5 passed
```

- **commands** is the same rule [`lyrn check`](/reference/check/#commands) holds on
  every push: every `$ <program> <command>` the README and the pages show is real,
  and every command has a page, or is named on one.
- **flags** goes further: every option of a command and of its subcommands is
  named on that command's page, and every subcommand is. A guide that mentions a
  flag does not count: the page is where it is looked for.
- **pages**: every page has a title, and warns without a description. A reference
  page titled for a command is for one the program has: a command that was
  renamed leaves a page behind, and nothing else would notice.
- **links**: every link inside the site reaches a page, or a file `public/` holds,
  and the heading it names. Starlight builds a broken link without a word.
- **sidebar**: warns about a directory of pages with nothing in the sidebar, which
  can be reached by a search and by nothing else.

A program of flags alone, a site with no sidebar set and a repository with no
program skip what does not apply to them.

## lyrn docs dev

Runs `pnpm dev` where the site lives, after `pnpm install` if it has not been
installed yet, and exits with what the server exits with. The commands are the
site's own; only where to run them is lyrn's to know.
