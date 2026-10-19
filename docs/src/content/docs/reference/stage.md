---
title: lyrn stage
description: Start a stage - what it is, what stands in its way - and leave a line about it in rigger's record.
---

```console
$ lyrn stage [path]
```

A stage is one version, built from the plan to a tag. Before its first commit
the same few questions come up every time: which stage is it, did the last
release really go out, is the tree clean and level with what was pushed, is the
stack due its weekly update - which is then the stage's first commit - and
what do the doctor and the shopfront check say. `lyrn stage` asks them, says
what stands in the way, and leaves one line in
[rigger](https://github.com/lacodda/rigger)'s record, so the start of the stage
is in the diary next to the work it began, and not only in git.

```console
$ lyrn stage
lyrn stage: ~/word-count, word-count

stage     v0.2.0 · Counting by lines, from rigger's record
            [41] `word-count --lines`
            [42] Read from standard input
release   v0.1.0 on 2026-10-19, nothing committed since; CI green on 25bd80d
tree      clean, main, level with origin/main
stack     updated today, on 2026-10-19 - holds until 2026-10-26, so the stage needs no update
doctor    1 to look at (mark), 12 passed
check     7 passed

Nothing stands in the way.

In rigger's record for word-count: Stage v0.2.0 began at 25bd80d; v0.1.0 out on 2026-10-19; tree clean; stack updated today; doctor: 1 to look at (mark), 12 passed; check: 7 passed.
```

- **stage** is the stage rigger's record says is being built, with its open
  tasks. A stage whose version is already tagged means the record's plan is
  behind git.
- **release** is the newest version tag behind the commit, what has been
  committed since, and what GitHub's CI said about the tag's commit: a release
  whose CI is red or still running is not finished.
- **tree** has to be clean, with nothing waiting to be pushed and nothing to
  pull.
- **stack** says whether the week since the last `chore(deps)` has passed; when
  it has, the stage starts with the update, and [`lyrn deps`](/reference/deps/)
  lists what it moves.
- **doctor** and **check** are [`lyrn doctor`](/reference/doctor/) and
  [`lyrn check`](/reference/check/), counted; a shopfront that disagrees is
  something to put right before the stage adds to it.

What stands in the way is listed at the end; `lyrn stage` itself exits with 0,
since a ritual reports rather than refuses. The line it leaves is a `state`
note - `rigger note <project> --kind state` - which rigger files in the sitting
that is open. Without rigger, or for a repository its record does not know,
nothing is recorded and the report says so.
