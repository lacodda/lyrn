# ADR 0003: Optional pieces are add-ons, not defaults

- Status: accepted
- Date: 2026-09-01

## Context

The `cli` form was specified as "a Rust CLI after the turnout pattern", and
turnout carries both secrets in the OS keyring and a `self-update` command.
Taking that literally would put both in every generated CLI.

Looking at what the line actually runs contradicts it. Of the four CLIs,
turnout and sefy use the keyring; kasl and turnout update themselves. Neither
feature is universal, and each drags in dependencies - a platform keyring, an
HTTP client - that a project which never calls them still has to build, audit
and keep current.

## Decision

A form declares a core it always writes, plus add-ons it understands.
`--with keyring,self-update` asks for them; nothing arrives unasked. An add-on
a form does not declare is an error, not a silent no-op.

Add-ons are expressed two ways. A whole file belongs to an add-on and is simply
not written when it was not asked for. Where an add-on contributes a few lines
to a file that always exists - a dependency line, a match arm, a `mod` - the
file carries `{{#addon}} ... {{/addon}}` sections. Sections are resolved before
placeholders, so a section that is off may mention variables that only make
sense with it on.

## Consequences

- A generated project contains only code it calls. That is the point: the
  alternative is a scaffold whose first task is deleting things.
- The seams are where the failures live - a `mod` declared but not written, a
  section that leaves a file badly formatted. Both happened while building
  this. So every combination of every form's add-ons is generated in tests and
  checked in CI, rather than only the default one.
- An unclosed section is silent damage: with the add-on off it swallows the
  rest of the file, and with it on it changes nothing. A test checks the
  markers balance and name real add-ons.
- Combinations grow as 2^n. Four is cheap to check exhaustively; a form that
  ever needs enough add-ons for that to hurt has too many.
