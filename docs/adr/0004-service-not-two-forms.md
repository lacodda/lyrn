# ADR 0004: One service form, not two

- Status: accepted
- Date: 2026-09-02

## Context

The plan called for two forms: `service`, an axum server with a web UI
compiled into it after kasl-server, and `server`, a plain axum service for
efema, which has no interface of its own.

Written out, the two differed in exactly one thing: whether a frontend exists.
Everything else - the router, the config, the migrations, the error type, the
Dockerfile, the deploy - was the same file twice.

## Decision

One form, `service`, plain by default. `--with spa` adds the web UI.

The default is the smaller thing on purpose: a service that serves an API and
nothing else is the more common shape, and it is the one where an unused
frontend would be pure weight.

## Consequences

- `lyrn forms` lists one line where the plan implied two, and "service" has to
  be read as "a server, possibly with a UI" rather than "a server with a UI".
- The add-on mechanism carries it, so nothing new was needed to express the
  choice - the same machinery that gives `cli` its keyring.
- The frontend files are reused from the `spa` form rather than copied, mapped
  into `frontend/`. A file added to `spa` and not given a place there fails the
  build immediately, rather than landing somewhere wrong and being noticed by
  whoever first tries to build the UI.
- If the two shapes ever genuinely diverge - a different router, a different
  deployment story - this decision is what to revisit, and splitting them then
  is cheaper than merging them now.
