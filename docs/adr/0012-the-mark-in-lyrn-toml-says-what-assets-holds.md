# ADR 0012: The mark in lyrn.toml says what assets/ holds

- Status: accepted
- Date: 2026-11-06

## Context

`lyrn.toml` records `mark = "placeholder"` or `"chosen"` so that `lyrn doctor`
need not compare icon bytes against a copy of the placeholder, which goes stale
the first time the placeholder is redrawn. Until 2.17 `lyrn new` decided it from
the accent: any colour other than the placeholder's was a choice.

So `lyrn new demo-app --accent kilna` wrote `chosen` over the umbrella mark's
masters, and the doctor told the owner the lambda was a mark of its own. The
record said what somebody typed, and the masters in `assets/` said something else.

## Decision

`mark` says what `assets/` holds, and nothing else decides it.

`--accent <product>` is the product's identity whole: dowel-ui publishes every
product's masters, lyrn vendors them all, and `lyrn new` lays them and draws every
raster before the first commit. A colour alone colours the palette and leaves the
umbrella mark on, so it stays `placeholder`. `lyrn adopt` reads the masters a
repository keeps. `lyrn doctor` says back a `chosen` over the umbrella mark's own
masters, with a repair.

A documentation site added to a repository draws its copies from the masters that
repository keeps, for the same reason: the doctor holds them to it byte for byte.

## Consequences

The binary carries sixty more small SVGs, and `tools/vendor-dowel.mjs` takes them
with the rest, so a new product of the line is known the day dowel-ui has it.

`demo-app --accent kilna` wears kilna's mark. That is what the flag now means, and
the banner, which says the product's name, is left out for that reason. A project
that wants the colour and no mark gives the colour.

Repositories started before 2.17 with a colour keep `chosen` over the lambda until
the doctor says so, at the next look, and `--fix` writes `placeholder`.
