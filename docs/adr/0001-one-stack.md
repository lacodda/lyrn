# ADR 0001: One stack, assembled to the end

- Status: accepted
- Date: 2026-09-01

## Context

lyrn 1.x promised to start a project "with any major framework": React, Vue,
Svelte, each with its own template, all of them wrapping webpack and generating
its configuration. By 2026 both halves of that promise had expired. Vite became
the single build tool the ecosystem agreed on, and every framework ships its
own `create-*` command that does the framework part better than a third party
can.

What was left was not a gap in the market. Choosing between fifteen options is
not the hard part of starting a project; living with the consequences is.

## Decision

lyrn generates one stack — Vite, React, TypeScript, Tailwind and the dowel
design system — and no others. `--form` selects the shape of the repository
(an SPA, a CLI, a service), never the framework inside it.

Everything a product on the line would otherwise assemble by hand goes in with
it: the CI gate, the changelog config, the ADR directory, editor and git
configuration, and a first commit.

## Consequences

- A generated project is identical in structure to every other product on the
  line, so moving between them costs nothing and `doctor` can check all of them
  against one standard.
- lyrn is useless to anyone who wants a different stack. That is the trade:
  the value is in the decisions already made, and an option to undo them would
  remove it.
- The stack has to be maintained as one thing. When Vite or Tailwind moves,
  the template moves with it, and CI catches the break by building a generated
  project rather than by trusting the template to still be correct.
- 1.x is closed rather than migrated: `create`, `start`, `build` and `export`
  were webpack commands, and the thing they configured is gone.
