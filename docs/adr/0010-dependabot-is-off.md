# ADR 0010: Dependabot is off on purpose

- Status: accepted
- Date: 2026-10-07

## Context

Dependabot opens a pull request for each new version of each dependency, and
another for each advisory against one. For a project with one maintainer that
hands the rhythm of updates to a bot: a dozen pull requests a week, each merged
on the strength of a green check rather than a reading of what changed.

Its two halves do different jobs. Version updates can wait for a day that
suits the project; a vulnerability cannot.

## Decision

No `.github/dependabot.yml`. Dependencies, the toolchain included, are updated
by hand at most once a week, in a commit of their own, majors and all.

The security half is `.github/workflows/audit.yml`: `cargo deny check` against
`deny.toml` on every push, every pull request and every Monday morning -
published advisories, licenses outside the accepted list, crates from anywhere
but crates.io. The day it was added it found RUSTSEC-2026-0009 in `time`, which
`liquid` pulls in; the fixed release needs Rust 1.88, and the MSRV moved to it.

It is the same decision every repository lyrn generates starts with, as its
ADR 0002.

## Consequences

- Updates arrive in batches somebody reads.
- An advisory published while nothing was pushed still turns the audit red on
  the next Monday.
