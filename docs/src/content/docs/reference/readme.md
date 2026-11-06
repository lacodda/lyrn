---
title: lyrn readme
description: Take the README's transcripts again from the build, so what it shows is what the program prints.
---

```console
$ lyrn readme snap [path] [--check]
```

A transcript is the README's strongest claim: this is what happens when you type
this. Written by hand, or pasted once, it goes stale with the first change to the
output, and nothing fails. `lyrn readme snap` runs the commands and writes what
they print back into the file.

A block is taken again only when it asks to be, with a comment on the line above
it. Most console blocks in a README are usage, with placeholders, or commands
nobody should run for a screenshot:

````md
<!-- snap -->
```console
$ lyrn new demo-app --accent kilna --yes
```
````

`<!-- snap: keep <pattern> -->` keeps only the lines of the output the pattern
matches, for a command that prints more than a README should show:

````md
<!-- snap: keep ^(>|registry:| *Test Files| *Tests) -->
```console
$ cd demo-app && pnpm lint
```
````

## How the commands run

Marked blocks run in order, as one person's afternoon rather than separate
examples: in `bash`, in a scratch directory that is thrown away afterwards, each
command starting where the one before ended, so a `cd` in one block holds for the
next. The product's own programs come first on the `PATH`, built first if a source
is newer: the transcript is of this commit's build, not of whatever is installed.

What was printed is written as it reads anywhere: no colours, no carriage returns,
and the scratch directory as `~`, in every spelling a program on the machine may
print it in. A command that fails stops the whole run with what it said, and the
README is left as it was: a transcript of a failure is not what the README means
to show.

## Options

| Option | What it does |
| --- | --- |
| `[path]` | The repository; defaults to the one the current directory is in |
| `--check` | Say what no longer matches, as a diff, write nothing, and exit with 1 if anything does |

A mark with no console block under it, a block with no `$ ` line and a pattern that
does not compile are errors, naming the line: each would otherwise leave a
transcript nobody retakes, under a comment saying it is.
