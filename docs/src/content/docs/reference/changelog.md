---
title: lyrn changelog
description: A version's changelog section, and the screens that go with it.
---

```console
$ lyrn changelog [version] [--screens] [--path <path>]
```

With no flag, prints a version's section of `CHANGELOG.md`, the newest unless a
version is named. Either `2.16.0` or `v2.16.0` will do.

## --screens

git-cliff writes a release's section from the commits, and commits say what changed
in words. For a product with a screen the words are half of it: the release that
redrew the board reads the same as one that renamed a variable.

So a version's screens are kept beside the code, in `docs/screens/<tag>/`, as
`.png`, `.jpg`, `.webp` or `.gif`. That is outside what a crate or a wrapper
carries, and outside the site. `--screens` puts them into the version's section:

```console
$ lyrn changelog --screens
CHANGELOG.md: 2 screens under v1.4.0.
The release page keeps the notes it was published with: add them with `gh release edit v1.4.0` if they belong there too. The images have to be at the tag for the links to answer.
```

```md
### 🖼️ Screens

![The new board](https://raw.githubusercontent.com/acme/app/v1.4.0/docs/screens/v1.4.0/01-the-new-board.png)

![Its list view](https://raw.githubusercontent.com/acme/app/v1.4.0/docs/screens/v1.4.0/02-its-list-view.png)
```

The images are in name order; a number in front of a name decides the order and
stays out of the caption, which is the rest of the name in words. The links are
absolute and at the tag: the changelog is read on GitHub, crates.io and npm, and a
tag does not move. So the images go in before the tag does, and the section after
[`furca release run`](https://lacodda.github.io/furca/reference/release/) has
written it - in a `docs:` commit, which leaves the release as it is.

Run again with another image, it changes the block rather than adding a second one,
and with nothing new it changes nothing.

| Option | What it does |
| --- | --- |
| `[version]` | The version; defaults to the newest in `CHANGELOG.md` |
| `--screens` | Put the version's images from `docs/screens/<tag>/` into its section, instead of printing it |
| `--path <path>` | The repository; defaults to the one the current directory is in |
