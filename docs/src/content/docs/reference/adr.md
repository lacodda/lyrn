---
title: lyrn adr
description: Write a decision down - the next number in the log, listed in its index.
---

```console
$ lyrn adr new <title> [--path <path>]
```

Writes a decision record into the repository's log, `docs/adr/`, and lists it in
the log's index. What makes a record a chore is the part around the writing:
finding the next number, keeping the file's name in the log's shape, adding the
row. The number is read from the directory at the moment of writing, so two
decisions on one afternoon are not both 0012, and a gap stays a gap: a decision
that was deleted was still that number once.

```console
$ lyrn adr new "One service form, not two"
Wrote docs/adr/0012-one-service-form-not-two.md.
Listed it in docs/adr/README.md.
```

The file has the shape of the rest of the log - the title, the status and date,
then `Context`, `Decision` and `Consequences`, each with a comment saying what
goes there:

```md
# ADR 0012: One service form, not two

- Status: accepted
- Date: 2026-11-06

## Context

<!-- What forces a decision now, and what makes it hard. -->
```

| Option | What it does |
| --- | --- |
| `<title>` | What was decided: the record's title, and in lowercase its file's name, so it has to be in English |
| `--path <path>` | The repository; defaults to the one the current directory is in |

An index without a table of decisions is the repository's own idea of an index:
it is left as it was, and the command says so. A repository with no log at all
is refused - [`lyrn adopt`](/reference/adopt/) starts one, with the decisions
the standard points at.

Superseding an earlier decision is a word in the new record and a word in the
old one: the log's rule says the earlier one is marked, not deleted.
