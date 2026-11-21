# ADR 0014: One check for a release tag

- Status: accepted
- Date: 2026-11-21

## Context

`lyrn repo` holds the version tags to green CI with a ruleset on GitHub: a
tag can be created only on a commit whose required checks passed. It named
the jobs of the newest green run as the required checks - 66 of them for
lyrn, a matrix of forms, add-ons and systems.

A required check is a name. The first time the matrix changed, GitHub refused
the next tag: it waited for a job the new CI no longer ran, until `lyrn repo`
was run again. Every product of the line whose CI grew a job had the same
trap waiting before its next release.

## Decision

The CI the line's standard writes, from 2026.11.6, ends in one job, `ci`. It
needs every other job, runs always, and fails if any of them did not pass or
was not skipped on purpose. `needs` alone would not do: a job whose dependency
failed is skipped, and a required check that was skipped counts as passed.

`lyrn repo` requires `ci` alone where the newest green run has it, and every
job where it does not. A test holds every form's `ci` to need every other job
of its CI, with every set of add-ons, and lyrn's own CI the same way.

## Consequences

The CI can grow, lose and rename jobs under one name that does not change, and
the ruleset is written once. A job added without being named in `ci`'s `needs`
is a red test in lyrn rather than a job a tag never waits for.

A repository on an older standard keeps its list of checks until `lyrn
upgrade` brings `ci`, and `lyrn repo` after its next green run switches the
rule over.
