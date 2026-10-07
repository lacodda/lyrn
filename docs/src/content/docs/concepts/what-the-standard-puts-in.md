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
| `lyrn.toml` | What this was generated from, and whether it has a mark yet, for `doctor` and `upgrade` later |
| `CONTRIBUTING.md` | The gate, where things live, how commits are written - so the README can stay a shopfront |
| `llms.txt` | The same map for an agent; assistants' own instruction files stay on the machine, out of git |
| `SECURITY.md` | A vulnerability is reported privately, not in a public issue |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 3.0, with reports going privately to the maintainers |
| `.github/ISSUE_TEMPLATE/`, `pull_request_template.md` | A report arrives with the version, the platform and a way to reproduce it |
| `.github/workflows/audit.yml` | Published advisories and licenses, on every push and every Monday |
| `deny.toml` | What the audit holds Rust crates to; npm packages get `tools/check-licenses.mjs` |
| `assets/` | The mark's three masters - the line's umbrella mark until the product has its own |

## Dependabot is off, the audit is on

Every repository starts with two decisions recorded: that decisions are
recorded, and that Dependabot is off on purpose. Version updates are done by
hand, at most once a week, in a commit somebody reads - majors included. The
half of Dependabot worth keeping is the audit workflow: it fails on a published
advisory against a dependency, and on a dependency under a license outside the
list the line accepts (permissive licenses, and MPL-2.0, whose copyleft stops
at the file). It also runs on Monday mornings, so an advisory published while
nothing was pushed still turns it red.

Private vulnerability reporting has to be switched on for the repository
(Settings, Advanced Security) for the link in `SECURITY.md` to work; both it and
the code of conduct say what to do if it is not.

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
