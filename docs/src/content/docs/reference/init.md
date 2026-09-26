---
title: lyrn init
description: Start a project in a directory that already exists.
---

```console
$ lyrn init [path] [options]
```

Starts a project in a directory that is already there - the current one, or
`path`. It is `lyrn new` with the other placement: the same forms, add-ons and
options, the same tree shown before anything is written. The usual case is a
repository created on GitHub and cloned empty:

```console
$ gh repo create my-tool --public --clone
$ cd my-tool
$ lyrn init --form cli
```

## The name

The directory's name, unless `--name` says otherwise. A directory whose name is
not a valid project name is refused with that reason:

```console
$ lyrn init "My Tool"
error: the directory is called `My Tool`, which is not a project name: the name starts with `M`; it has to start with a lowercase letter - pass `--name`
```

## What is already there

Whatever the directory holds stays as it is. A file the form would write that
is already present stops the whole run before anything is written, and every
such file is named - a partly generated project beside files of its own would
be neither:

```console
$ lyrn init --form cli
error: nothing was written: these files are already in `.`:
  LICENSE
  README.md
move them aside and run again, or `lyrn adopt` to add only the standard files that are missing
```

Those are most often the README, LICENSE and `.gitignore` a hosting service
offers to put in a new repository. [`lyrn adopt`](/reference/adopt/) is the
other way round: it keeps what is there and adds only what is missing, but
only the files of the standard, not a scaffold.

## The repository

lyrn starts a repository - `git init` and one Conventional Commit - only where
there is none. Inside a work tree that already exists, whether it is the
directory's own or one it is a folder of, the history and the next commit
belong to its owner: the files are written, and the commit is left to you.

```console
Next:
  git add . && git commit -m "feat: scaffold the project with lyrn"
  cargo run -- hello
```

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `[path]` | `.` | The directory to start the project in |
| `--name <name>` | the directory's | The project name |
| `--form <form>` | `spa` | The shape of the project; see [Forms](/reference/forms/) |
| `--host <host>` | — | The application a plugin extends; `--form plugin` only |
| `--accent <colour>` | asked, else graphite | A product of the line, or a `#rrggbb` value |
| `--description <text>` | asked, else generic | One line describing what the project is |
| `--author <name>` | `git config user.name` | Recorded in LICENSE |
| `--repo <owner/name>` | looked up | The GitHub repository it will live in |
| `--with <addon,...>` | none | Optional pieces, per form |
| `-y`, `--yes` | — | Accept the defaults instead of asking |
| `--dry-run` | — | Show the tree that would be written, and write nothing |
| `--no-hooks` | — | Skip installing dependencies and starting the repository |

Every option means what it means for [`lyrn new`](/reference/new/).
