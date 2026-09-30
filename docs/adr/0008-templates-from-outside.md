# ADR 0008: A template from outside is used at a tag whose CI passed

- Status: accepted
- Date: 2026-09-30

## Context

Until 2.9 every template lyrn could generate from was built into the binary,
and lyrn's own CI generated each one and ran its gate on three platforms. That
is the whole reason a generated project can be trusted: a template that does
not build does not ship.

`--template` takes templates from outside - a directory, `~/.lyrn/templates`,
a GitHub repository - and none of them passes through lyrn's CI. Something has
to stand where that CI stood, or a template from outside is a template nobody
has built.

## Decision

- A template from GitHub is used at a **tag**, never a branch, so what a project
  was made from can be named again. The tag is resolved to its commit, GitHub
  is asked about the CI of that commit, and the clone must be that commit: the
  files generated from are the files the CI built. A tag moved between the
  question and the clone is a refusal.
- The CI passes when something ran, everything finished, and nothing failed;
  a commit whose every job was skipped has been built by nobody. No result, a
  red one, one still running, or no answer from GitHub - each refuses the
  template. There is no flag to skip it; a template cloned by hand and passed
  as a path is used on its owner's judgement, which is the honest way round.
- `lyrn.toml` records the source and the commit.
- A template's hooks are limited to what lyrn's own forms run - installing
  dependencies and starting the repository. A green CI proves a template's
  project builds, not that its hooks are harmless on the machine running them.
- A template's manifest refuses fields lyrn does not know, and may name the
  oldest lyrn that understands it: a field read past is a generation that
  differs from what the template meant, with nothing saying so.
- Only GitHub is supported, because only GitHub's CI can be asked.

A lyrn template is `template.toml` plus the project's files under `template/`,
so the repository's own README and CI never reach the project. `lyrn template
export` writes a built-in form in that shape, with a CI that generates every
combination of its add-ons, and lyrn's tests hold the export to generating the
built-in project byte for byte.

## Consequences

- A template repository needs its own CI before it is usable, and the first
  tag has to wait for it. `lyrn template export` writes that CI.
- Generating from GitHub needs the network and one or two API calls; anonymous
  calls are limited to sixty an hour, and a token lifts that.
- cargo-generate templates are read too, through the same Liquid engine and
  the same variables; their Rhai hooks are refused, for the same reason as a
  hook outside the vocabulary.
