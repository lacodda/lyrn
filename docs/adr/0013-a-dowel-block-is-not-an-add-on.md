# ADR 0013: A dowel block is not an add-on

- Status: accepted
- Date: 2026-11-21

## Context

dowel-ui 0.36 started publishing blocks - whole screens made of its primitives:
a list beside its detail, settings in sections, a sign-in page, an empty
dashboard, and from 1.0 a reader. `lyrn new` was to write them on request,
and the nearest thing lyrn had was the add-on: `--with router`, `--with auth`.

An add-on is the form's. Its files are listed in the form, it has sections in
the form's shared files, `lyrn.toml` records it and `lyrn upgrade` renders the
standard with it, and the CI builds every set of the form's add-ons, because
the failures live in the seams between them. Five blocks as add-ons of the spa
form would have been nine add-ons and 512 sets to build, and a primitive two
blocks share - the frame - would have belonged to two add-ons, which a file of
a form cannot say.

## Decision

A block is asked for beside the add-ons, as `--with block:<name>`, and is not
one. It is dowel's: `block:list-detail` is the registry item
`list-detail-screen`, written with every primitive it is made of, resolved
through the registry's own `registryDependencies` from a catalogue
`tools/vendor-dowel.mjs` takes out of the dowel-ui the templates pin. It lands
where the plan's `components.json` puts the registry's places - `src/` for a
spa, `frontend/` for a service with its web UI - and a form that draws nothing
of dowel's refuses it. It is not in `lyrn.toml`'s `with`, and nothing renders
it: a route to it would need content only the product has.

`lyrn add <item>` is the same resolution later, from the registry inside the
project's own `node_modules/dowel-ui`, which is the file `check-registry` holds
the copies to.

## Consequences

A block dowel publishes is one vendoring away from `--with`, with no change to
any form. The CI builds all of them in one job, beside the sign-in screen,
which is the LoginScreen block itself and so the auth add-on's file.

A block is the product's from the moment it lands: `lyrn add` never writes
over one, while a primitive under `components/ui` that differs from the
registry is taken again. A project cannot ask `lyrn upgrade` to bring its
blocks up to a newer dowel - that is the product's code now, as it would be
after `shadcn add`.
