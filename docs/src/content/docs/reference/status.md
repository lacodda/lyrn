---
title: lyrn status
description: Where the repository stands, on one screen - the branch, the work tree, the last release, its CI, the stack and the stage.
---

```console
$ lyrn status [path]
```

Where the repository stands, each line from the place that knows it: git for
the branch against what was pushed and for the work tree, the tags for the
last release and what has been committed since, GitHub for what the CI said
about the last pushed commit, the history for how old the stack is, and
[rigger](https://github.com/lacodda/rigger)'s record for the stage being built.

```console
$ lyrn status
lyrn status: ~/lyrn, lyrn 2.12.0

branch    main, 1 commit ahead of origin/main - not pushed
tree      clean
released  v2.12.0 on 2026-10-13, 1 commit since: 1 fix
ci        green on 4f5027b (the last pushed commit)
stack     updated 6 days ago, on 2026-10-13 - holds until 2026-10-20
stage     v2.13.0 · Rituals as commands, 3 tasks open - `rigger context lyrn`
```

- **released** counts the commits since the newest version tag behind this
  one, by their Conventional Commits type.
- **ci** asks GitHub about the commit the branch's upstream is at - with the
  token `gh` is logged in with, or `GH_TOKEN`, when there is one, and
  anonymously otherwise.
- **stack** holds for a week: the line brings its stack up to date at the start
  of a version, at most once a week, and [`lyrn deps`](/reference/deps/) says
  what an update would move.
- **stage** is rigger's; without rigger, or for a repository its record does
  not know, the line says so.

Nothing here is kept by lyrn, and `lyrn status` exits with 0.
