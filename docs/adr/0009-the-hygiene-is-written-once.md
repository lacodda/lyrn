# ADR 0009: What every repository carries is written once

- Status: accepted
- Date: 2026-10-07

## Context

Beside its code, every repository of the line carries the same things: how to
work on it, how to report a vulnerability, how people treat each other, issue
and pull request templates, an audit of its dependencies, the first decisions,
and the mark it wears until it has its own. Until 2.10 the forms carried none
of the first five, and the parts they did share - ADR 0001 and its index - were
kept as copies per form. The copies had already drifted: two texts of ADR 0001
in two formats, and a workspace whose 0001 and 0002 were written in different
ones.

Nine forms with a copy each is nine places to forget the next fix.

## Decision

- **One module adds them.** `src/templates/community.rs` writes the set, and
  `templates::sources_for` adds it to every form that starts a repository. A
  form writes only what is its own.
- **What differs by form is chosen from pieces written once.** CONTRIBUTING is
  the form's gate and layout followed by the line's shared part; the audit
  workflow is a job per toolchain the form builds with, each pointed at that
  toolchain's manifest; the ADR index is the shared decisions followed by the
  form's own. Pieces are joined at compile time, so a form's file is still one
  `&'static str` and `lyrn template export` writes it whole.
- **The shared decisions come first.** 0001 records decisions, 0002 turns
  Dependabot off; a form's own decisions are numbered from 0003.
- **The README is a shopfront.** Building, the gate and the layout moved from
  each form's README into its CONTRIBUTING.
- **Icons are drawn where the masters are.** The forms carry the mark's three
  masters in `assets/`; the documentation site carries `export-assets.mjs`,
  because `sharp` is a dependency of the site already, as it is in every
  product of the line. The exporter reads the form from `lyrn.toml` and writes
  the `.ico` only where that form's build reads it.
- **`adopt` takes the decision log whole.** Into a repository that keeps no
  decisions, the shared ones and their index; into one that keeps its own,
  none - a second 0002 would make "ADR 2" mean two things.

## Consequences

- A fix to the Code of Conduct, the security policy or the audit is one edit,
  and every form has it.
- Tests hold every form to the set, the ADR numbering to a sequence without
  gaps, the audit to the toolchains the form actually has, and `deny.toml` and
  the npm license check to one list of licenses.
- A form that starts a repository and wants to leave a piece of the set out has
  to say so in `community.rs`, where it is visible, rather than by forgetting.
