---
title: lyrn upgrade
description: Catch a repository up with the newest version of the line's standard, as a three-way merge and one commit.
---

```console
$ lyrn upgrade [path] [--dry-run] [--yes]
```

The standard moves on - a workflow gains a job, `cliff.toml` a group - and the
repositories it was written into stay where they were. `lyrn.toml` says which
version each one carries; `lyrn upgrade` brings it to the version this lyrn
writes, and commits the result as one change.

```console
$ lyrn upgrade --yes
lyrn upgrade: `.`, a cli project, from the standard 2026.10 to 2026.11

What the standard changed since:
  2026.11
    - A shopfront workflow, `.github/workflows/shopfronts.yml`: `lyrn check` on every push, ...
    - cliff.toml keeps the changelog's explanation in its header, ...
    ...

  .github/workflows/shopfronts.yml  added
  CHANGELOG.md                      merged: the standard's change, beside the repository's own
  cliff.toml                        updated
  lyrn.toml                         updated

Committed: chore(standard): take the line's standard 2026.11
  git show --stat    # what the upgrade changed
```

What each version changed is listed on
[The standard, version by version](/concepts/the-standard-by-version/).

## A three-way merge

Every file of the standard is merged from three versions of it: what the
standard wrote then, what the repository has now, and what the standard writes
today. lyrn keeps every version of the standard it has released, as templates,
and renders the old one and the new one with the repository's own name,
description, accent and add-ons - so a line the two render alike is a line the
standard did not change, whatever the repository says in it.

| Step | When |
| --- | --- |
| `added` | The standard writes a file the repository does not have |
| `updated` | The standard changed a file the repository had left as it was written |
| `merged` | Both changed the file, in different places: the repository's lines are kept and the standard's change lands beside them |
| `needs a hand` | Both changed the same lines - see below |
| `removed` | The standard no longer writes a file the repository had left as it was |
| `kept` | The repository keeps the file under another name (`LICENSE.md`), its own decision log in `docs/adr/`, or a file the standard dropped and the repository changed |

The README is never merged: it is the product's words, and the standard's
README was only ever their first draft. A file checked out with `\r\n` line
ends is merged on its lines and written back with them.

## Where both changed the same lines

The file gets the markers git uses, with the three versions named:

```text
<<<<<<< yours
standard = "2026.10" # held here on purpose
||||||| standard 2026.10
standard = "2026.10"
=======
standard = "2026.11"
>>>>>>> standard 2026.11
```

Every file is still written, nothing is committed, and the run ends with exit
code 1. Keep what both sides meant, drop the markers, and commit with the
message it prints. Until then [`lyrn doctor`](/reference/doctor/#standard)
reports the place as a failure.

## The commit, and the changelog

A clean upgrade commits only the files it wrote, as
`chore(standard): take the line's standard 2026.11`, with what each version
changed in the body. That commit is how the changelog learns which standard
the repository caught up with: the changelog is written by git-cliff from the
commits, and the standard's `cliff.toml` gives `chore(standard)` a group of its
own, so the next release says it under **Standard**. A line typed into
CHANGELOG.md by hand would land under the wrong release, or be written over.

Files the upgrade would change that hold work not yet committed stop it before
anything is written: the merge would read the unfinished work as the
repository's version, and the commit would carry it under the standard's name.

## What it needs

- `lyrn.toml` with `[template] standard` - written by
  [`lyrn new`](/reference/new/) and [`lyrn adopt`](/reference/adopt/). A
  repository without one is adopted first.
- A version of the standard this lyrn keeps. The oldest is 2026.10, the first
  version lyrn recorded; a newer one than this lyrn writes asks for a newer
  lyrn.
- A built-in form. A project generated from a template of your own is
  refused: its files are that template's, and lyrn keeps no versions of it.

A repository already on the newest version has nothing to do, and says so.

## Options

| Option | Default | What it does |
| --- | --- | --- |
| `[path]` | `.` | The repository to upgrade |
| `--dry-run` | — | Print what would change, with the diffs, and write nothing |
| `-y`, `--yes` | — | Write without asking; otherwise, at a terminal, the diffs are shown and the question asked first |
