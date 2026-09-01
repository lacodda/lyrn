---
title: What the standard puts in
description: Why a generated project carries more than an app.
---

A generated project holds more than the code that runs. Everything else in it
is there because leaving it out costs more later.

## The files

| File | Why it is there |
| --- | --- |
| `.github/workflows/ci.yml` | The gate, running exactly what `pnpm lint` runs locally |
| `cliff.toml`, `CHANGELOG.md` | The changelog is generated from the history, so the history has to be worth generating from |
| `docs/adr/` | Decisions outlive the conversation that produced them |
| `.editorconfig` | Indentation stops being a matter of whose editor opened the file |
| `.gitattributes` | `eol=lf`, so a tool that rewrites a file to CRLF cannot hide the real change in the diff |
| `components.json` | `shadcn add` knows where to copy a dowel primitive |
| `lyrn.toml` | What this was generated from, for `doctor` and `upgrade` later |

## The gate is one script

`pnpm lint` is eslint, then `tsc --noEmit`, then the tests. CI runs that same
script rather than its own sequence, so there is no way for the two to drift
apart and no way to be surprised by CI.

## The first commit

lyrn does not leave an uncommitted directory behind. The project starts as a
repository on `main` with one Conventional Commit, because the changelog
config it ships with reads Conventional Commits — a history that starts
messy cannot be tidied retroactively.

## Colour goes through the theme

A component names a token and never writes a colour down, which is what lets
the theme swap it underneath and the product's accent move. `eslint` enforces
it via `dowel/no-raw-color`, so it is a rule rather than a habit.
