---
title: lyrn repo
description: Put the repository on GitHub, or bring the one there in line with what the project says - its description, topics, pages, private reports, and tags only on green CI.
---

```console
$ lyrn repo [path] [--dry-run] [--private]
```

A repository on GitHub carries settings no file of it holds: what it says it
is, where its home is, its topics, how its pages are built, whether a
vulnerability can be reported in private, which refs cannot be rewritten. Set
by hand once in a settings page, each drifts from the project the day the
project changes - and every one of them is something the project already says,
in its manifests, its workflows, its `SECURITY.md` and its release. `lyrn repo`
reads them from there and makes GitHub say the same. Run it again whenever the
project changes: it writes only what differs.

```console
$ lyrn repo
lyrn repo: ~/lyrn, lacodda/lyrn

✓ repository      public, on GitHub
✓ description     the manifests': Start a new web application on the lacodda line's stack with one command
✓ homepage        https://lyrn.lacodda.com
→ topics          cli, scaffold, template, vite, react, tailwind, starter, rust, scaffolding, project-template - the keywords of Cargo.toml and npm/package.json; were cli, project-template, react, rust, scaffolding, vite
→ pages           built by the workflow that deploys them, at lyrn.lacodda.com; were built from a branch
✓ reports         a vulnerability is reported in private, where SECURITY.md sends it
→ tags            v* are created only on a commit ci.yml passed - its 66 checks: macos-latest, msrv, template audit (cli), template audit (desktop) and 62 more; never moved or deleted
→ main            never force-pushed or deleted
! social preview  GitHub shows another picture than assets/social-preview.png
    → upload assets/social-preview.png at https://github.com/lacodda/lyrn/settings → Social preview: GitHub has no API for it

4 set, 4 already so, 1 by hand.
```

`✓` is already so, `→` is written now - or would be, with `--dry-run` - `!` is
left to a hand, and `✗` is a setting GitHub refused, which makes the command
exit with 1.

## What it keeps

| Setting | Taken from |
| --- | --- |
| repository | `origin`, else the manifests' `repository`; created when GitHub has none |
| description | the manifests' `description`, the top ones first - the one [`lyrn check`](/reference/check/#descriptions) holds every shopfront to |
| homepage | the manifests' `homepage`, else the address the documentation site is built for |
| topics | the manifests' `keywords` - Cargo's and npm's, in that order, as GitHub writes a topic |
| pages | built by the workflow, when a workflow deploys pages, and served at the domain in `public/CNAME` |
| reports | private vulnerability reporting, open: `SECURITY.md` sends a reporter there |
| tags | `v*` created only on a commit whose CI passed, never moved or deleted |
| the default branch | never force-pushed or deleted |
| social preview | checked, not set: whether GitHub shows `assets/social-preview.png` |

The manifests are the one place each of these is written. A topic chosen by
hand on GitHub and missing from `keywords` is replaced, so move it into the
manifest first: crates.io takes five keywords, npm's `package.json` as many as
GitHub takes topics.

## A new repository

When GitHub has no repository by the name the project gives, it is created -
public unless `--private` - under the account the token belongs to, or under
the organisation the name starts with. `origin` is pointed at it if there is
none, what is committed is pushed, and the settings follow in the same run.
`--private` means nothing to a repository that exists: it keeps its
visibility. A private repository has no private reporting to open, and on a
free account no rules or pages either; those lines say so.

## Tags only on green CI

A release of the line is tagged once CI has passed on the commit, and GitHub
holds that rather than a habit: the rule on `refs/tags/v*` lets a tag be
created only on a commit whose checks passed, and lets no tag be moved or
deleted afterwards. The checks are the jobs of the workflow `[ci] workflow`
names in `release.toml` - `ci.yml` without one - as GitHub named them in the
newest run that passed on the default branch: a matrix and a job's `name:` are
spelt the way GitHub reports them only there. A repository whose CI has not
passed yet gets the rule without the checks, and is told to run `lyrn repo`
again once it has.

When the CI's jobs change, run `lyrn repo` again before the next tag: the rule
still asks for the old ones, and GitHub refuses a tag whose commit never
reported one of them.

The default branch is not held to the checks. The line pushes to `main` and lets
CI judge the push, and a branch that waits for checks refuses every push of a
commit that has none yet. What its rule keeps is the history: no force-push, no
deletion.

## The social preview

GitHub has no API to upload one. The command asks GitHub which picture it shows
and compares it with `assets/social-preview.png`, which
[`lyrn assets`](/reference/assets/#the-social-preview) draws, pixel for pixel:
the same picture holds, another one is left to a hand, with the page to upload
it on.

## The token

GitHub is asked with the token in `GH_TOKEN` or `GITHUB_TOKEN`, else the one the
`gh` CLI is logged in with (`gh auth login`). Without one the command writes
nothing and says so.

## Options

| Option | What it does |
| --- | --- |
| `[path]` | The repository; defaults to the one the current directory is in |
| `--dry-run` | Say what would change on GitHub, and change nothing |
| `--private` | Create the repository private rather than public; one that exists keeps its visibility |
