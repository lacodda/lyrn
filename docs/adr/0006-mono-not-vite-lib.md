# ADR 0006: One monorepo form, not a standalone library form

- Status: accepted
- Date: 2026-09-03

## Context

The plan called for two TypeScript forms: `vite-lib`, a single publishable
package at the root of its repository, and `mono`, a pnpm workspace with
`packages/*`.

An inventory of the line contradicted it. There is **no standalone publishable
TypeScript package** here: dowel, kjui and lyrnui are all monorepos whose root
is `private: true` and whose shipped artefact sits under `packages/`. The
shape `vite-lib` describes exists in the world at large, and nowhere in the
line the template is meant to serve.

The name was also wrong about the build. dowel does not build its package with
Vite; it uses `tsc`, and deliberately - a library ships modules, and bundling
them throws away the information a consumer's bundler needs.

## Decision

One form, `mono`. The package lives in `packages/<name>` from the first day,
with one member and room for a second.

`--with stand` adds a Vite page as a second workspace member, where the package
can be seen running. It is an add-on rather than a default because half the
line's monorepos have one and half do not, and an unused stand drags React,
Vite and a DOM test environment behind it.

## Consequences

- A repository that only ever holds one package carries a `packages/`
  directory it does not strictly need. That is the smaller cost: the day a
  second package appears - a plugin, a CLI over the same core - no move is
  required, and no published import path changes.
- `vite-lib` is not deferred, it is rejected: the line does not use that shape,
  and a form nobody generates is a template nobody maintains. If a standalone
  package is ever wanted, it is one file's difference from this one.
- This is the third time a plan item named two things where the line knows one
  (see ADR 0004). The check that finds it is the same each time: write the two
  out side by side and look at what actually differs.
