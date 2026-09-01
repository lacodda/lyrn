# ADR 0002: No telemetry

- Status: accepted
- Date: 2026-09-01

## Context

Scaffolding tools are a natural place to collect usage data: which forms are
picked, which options are passed, where generation fails. That data would
genuinely inform what to build next.

## Decision

lyrn collects nothing and phones home for nothing. The only network access it
ever makes is the npm wrapper downloading the binary it was asked to install,
and the installers resolving a release tag.

## Consequences

- What is popular has to be learned by asking rather than by measuring.
- `lyrn new` works offline: the templates are embedded in the binary, not
  fetched.
- No privacy policy, no opt-out flag, no environment variable to remember, and
  nothing to explain to somebody running it inside a company network.
- Should this ever be revisited, it is a breaking change of expectations rather
  than a feature, and this decision has to be superseded explicitly.
